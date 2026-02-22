#!/usr/bin/env bash
# Build SIMD kernels, extract per-op blobs, and pack into a .clfc (CLF Compute) file.
# Requires: cargo, ar, objcopy (GNU binutils), and the CLF repo packer.
# Op_ids: 1=Add, 10=Relu, 30=Convolution, 34=GlobalAvgPool, 35=BatchNorm, 50=MatMul
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLF_REPO="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${SCRIPT_DIR}/blobs"
PACKED="${SCRIPT_DIR}/resnet_tiny_mnist_simd.clfc"
TARGET_DIR="${SCRIPT_DIR}/target/release"

# Build staticlib with AVX2; CLFE 6-arg wrappers are in their own link sections (uniform ABI: in, out, w, in_len, out_len, w_len)
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+avx2"
cargo build --release 2>&1

# Find the static library (libclf_kernels_simd.a)
STATICLIB="${TARGET_DIR}/libclf_kernels_simd.a"
if [[ ! -f "$STATICLIB" ]]; then
    echo "Error: static lib not found at $STATICLIB" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
BLOB_DIR="$(mktemp -d)"
trap 'rm -rf "$BLOB_DIR"' EXIT

# Extract object file(s) from the archive
(cd "$BLOB_DIR" && ar x "$STATICLIB")
# With codegen-units=1 we typically get one .o; find the one that has our CLFE 6-arg wrapper sections
OBJ=""
for f in "$BLOB_DIR"/*.o; do
    [[ -e "$f" ]] || continue
    if objcopy -O binary -j .text.clf_abi6_add "$f" "$BLOB_DIR/add.bin" 2>/dev/null && [[ -s "$BLOB_DIR/add.bin" ]]; then
        OBJ="$f"
        break
    fi
done
if [[ -z "$OBJ" ]]; then
    echo "Error: no .o with .text.clf_abi6_add found in archive (build with CLFE 6-arg wrappers)" >&2
    exit 1
fi

# Extract each CLFE 6-arg wrapper section as a raw blob (op_id : section_name : output_name)
extract() {
    local op_id="$1"
    local section="$2"
    local name="$3"
    if objcopy -O binary -j "$section" "$OBJ" "$OUT_DIR/${name}.bin" 2>/dev/null && [[ -s "$OUT_DIR/${name}.bin" ]]; then
        echo "  op_id $op_id -> ${name}.bin"
    else
        echo "Error: failed to extract section $section" >&2
        exit 1
    fi
}

echo "Extracting CLFE 6-arg kernel blobs..."
extract 1  ".text.clf_abi6_add"                  add
extract 10 ".text.clf_abi6_relu"                 relu
extract 30 ".text.clf_abi6_conv"                 conv
extract 34 ".text.clf_abi6_global_avg_pool"      globalavgpool
extract 35 ".text.clf_abi6_batchnorm"            batchnorm
extract 50 ".text.clf_abi6_matmul"               matmul

echo "Packing into CLFC..."
cd "$CLF_REPO"
cargo run --bin coelanox-packer -- \
    --vendor "CLF-SIMD-AVX2" \
    --target "x86_64-AVX2" \
    --kind compute \
    --output "$PACKED" \
    --align 16 \
    1:"${OUT_DIR}/add.bin" \
    10:"${OUT_DIR}/relu.bin" \
    30:"${OUT_DIR}/conv.bin" \
    34:"${OUT_DIR}/globalavgpool.bin" \
    35:"${OUT_DIR}/batchnorm.bin" \
    50:"${OUT_DIR}/matmul.bin"

echo "Done: $PACKED"
