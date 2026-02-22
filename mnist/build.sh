#!/usr/bin/env bash
# Build SPIR-V kernels from GLSL and pack into resnet_tiny_mnist.spv.clfc
# Requires: glslc (Vulkan SDK or shaderc), cargo (CLF packer)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SHADERS_DIR="${SCRIPT_DIR}/shaders"
OUT_DIR="${SCRIPT_DIR}/spv"
CLF_REPO="${SCRIPT_DIR}/.."
PACKED="${SCRIPT_DIR}/resnet_tiny_mnist.spv.clfc"

# Compiler: prefer glslc (Vulkan SDK), then glslangValidator
GLSLC=""
if command -v glslc &>/dev/null; then
    GLSLC=glslc
elif command -v glslangValidator &>/dev/null; then
    GLSLC=glslangValidator
else
    echo "Error: need glslc or glslangValidator (e.g. install Vulkan SDK or glslang)" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

compile_one() {
    local name="$1"
    local src="${SHADERS_DIR}/${name}.comp"
    local dst="${OUT_DIR}/${name}.spv"
    if [[ ! -f "$src" ]]; then
        echo "Missing $src" >&2
        exit 1
    fi
    if [[ "$GLSLC" == "glslangValidator" ]]; then
        glslangValidator -V "$src" -o "$dst" -S comp
    else
        # Use vulkan1.0 so produced SPIR-V 1.0 is accepted by Vulkan 1.0 instance (avoid "Invalid SPIR-V binary version 1.5").
        glslc "$src" -o "$dst" --target-env=vulkan1.0 2>/dev/null || glslc "$src" -o "$dst"
    fi
    echo "  $name.comp -> $dst"
}

echo "Compiling GLSL compute shaders to SPIR-V..."
compile_one add
compile_one relu
compile_one conv
compile_one globalavgpool
compile_one batchnorm
compile_one matmul

echo "Packing SPIR-V blobs into CLF (Compute backend, .clfc)..."
cd "$CLF_REPO"
cargo run --bin coelanox-packer -- \
    --vendor "CLF-MNIST-SPIRV" \
    --target "SPIR-V" \
    --kind compute \
    --output "$PACKED" \
    --align 16 \
    1:"${OUT_DIR}/add.spv" \
    10:"${OUT_DIR}/relu.spv" \
    30:"${OUT_DIR}/conv.spv" \
    34:"${OUT_DIR}/globalavgpool.spv" \
    35:"${OUT_DIR}/batchnorm.spv" \
    50:"${OUT_DIR}/matmul.spv"

echo "Done: $PACKED"
