#!/usr/bin/env bash
# Pack a CLFE (Executor) file with at least one blob so the package is a concrete artifact.
# op_id 0 is reserved for descriptor/stub. Uses coelanox-packer --kind executor.
# Run from CLF repo root or from this directory.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLF_REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_FOLDER="${OUTPUT_FOLDER:-$CLF_REPO/output}"
STUB_DIR="${SCRIPT_DIR}/stub"
STUB_BIN="${STUB_DIR}/descriptor.bin"
PACKED="${OUTPUT_FOLDER}/executor.clfe"

mkdir -p "$STUB_DIR"
mkdir -p "$OUTPUT_FOLDER"
# Minimal x86-64 stub: single RET (0xC3). Satisfies packer requirement of at least one blob.
printf '\xc3' > "$STUB_BIN"

cd "$CLF_REPO"
cargo run --bin coelanox-packer -- \
    --vendor "CLF-Executor" \
    --target "x86_64" \
    --kind executor \
    --output "$PACKED" \
    --align 16 \
    0:"${STUB_BIN}"

echo "Done: $PACKED"
