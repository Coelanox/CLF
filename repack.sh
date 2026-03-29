#!/usr/bin/env bash
# Repack simd.clfc and executor.clfe into the output/ folder.
# Run from CLF repo root. Set OUTPUT_FOLDER to override (default: output).
set -euo pipefail
CLF_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_FOLDER="${OUTPUT_FOLDER:-$CLF_REPO/output}"
export OUTPUT_FOLDER

echo "Output folder: $OUTPUT_FOLDER"
echo "Packing simd.clfc..."
"$CLF_REPO/kernels/simd/pack_clfc.sh"
echo "Packing executor.clfe..."
"$CLF_REPO/clfe/pack_clfe.sh"
echo "Done. Artifacts in $OUTPUT_FOLDER:"
ls -la "$OUTPUT_FOLDER"
