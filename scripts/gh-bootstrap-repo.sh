#!/usr/bin/env bash
# One-time (or rare) setup for public visibility: repository description, topics,
# and roadmap issues. Requires: gh CLI, auth with permission to edit the repo and
# create issues. Intended for maintainers; running twice may duplicate issues.
set -euo pipefail

REPO="${GITHUB_REPOSITORY:-Coelanox/CLF}"

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh (GitHub CLI) is not installed. See https://cli.github.com/" >&2
  exit 1
fi

gh auth status

# Short description for the GitHub repo page (keep under typical UI limits).
DESC="Binary format for shipping pre-compiled hardware kernels (Coelanox). Rust reader and packer CLI with SIG0 integrity verification—auditable, compliance-oriented workflows for AI inference runtimes."

gh repo edit "${REPO}" \
  --description "${DESC}" \
  --add-topic ai-inference \
  --add-topic auditable-ai \
  --add-topic rust \
  --add-topic onnx \
  --add-topic compliance

# Roadmap label (ignore error if it already exists).
gh label create roadmap --color "0E8A16" --description "Planned or tracked future work" --repo "${REPO}" 2>/dev/null || true

create_issue() {
  local title="$1"
  local body="$2"
  gh issue create --repo "${REPO}" --title "${title}" --body "${body}" --label roadmap
}

create_issue "Roadmap: expand fuzz coverage for malformed length/count/kind" "$(cat <<'EOF'
## Summary
Add fuzz corpus seeds and/or harness paths that stress malformed `length`, `count`, and `kind` combinations in the reader.

## Why
Reduces risk of parser gaps and allocation surprises on hostile inputs.

## References
- `fuzz/` crate
- `CONTEXT.md` open follow-ups
EOF
)"

create_issue "Roadmap: review MAX_HEADER_TEXT_LEN for vendor/target metadata" "$(cat <<'EOF'
## Summary
If real deployments need larger `vendor` / `target` header strings, evaluate raising `MAX_HEADER_TEXT_LEN` with an explicit threat review and spec/docs updates.

## Why
Balance usability with DoS-resistant parsing.

## References
- `src/reader.rs`
- `SPEC.md`, `README.md`
EOF
)"

create_issue "Roadmap: authenticated signatures for RequireAuthenticity" "$(cat <<'EOF'
## Summary
Design and implement an authenticated trailer (algorithm ID, key ID, signature bytes) and key-management policy so `RequireAuthenticity` / `--verify-policy require-authenticity` can succeed with a defined trust model.

## Why
Integrity-only SIG0 + SHA-256 is documented today; authenticity is intentionally fail-closed until this exists.

## References
- `docs/SIGNING.md`
- `VerificationPolicy::RequireAuthenticity`
EOF
)"

create_issue "Roadmap: optional aarch64 release binaries" "$(cat <<'EOF'
## Summary
Extend CI/release assets to publish `clf-aarch64-unknown-linux-gnu.tar.gz` and/or `clf-aarch64-pc-windows-msvc.zip` with naming consistent with `scripts/install.sh` / `scripts/install.ps1`.

## Why
Improves onboarding for arm64 dev machines and edge devices without breaking existing `latest` asset expectations.

## References
- `.github/workflows/release.yml`
- `docs/RELEASE.md`
EOF
)"

create_issue "Roadmap: consumer integration examples" "$(cat <<'EOF'
## Summary
Add minimal, copy-paste-friendly examples (Rust or pseudocode) for common consumer flows: open, verify with policy, iterate blobs, build code section.

## Why
Lowers time-to-first-success for new integrators auditing the format.

## References
- `docs/CONSUMER_NOTE.md`
- `docs/ARCHITECTURE.md`
EOF
)"

create_issue "Roadmap: SPDX / SBOM hooks in manifests (evaluate)" "$(cat <<'EOF'
## Summary
Evaluate whether optional SPDX or SBOM fields belong in pack manifests or sidecars without complicating the core CLF wire format.

## Why
Helps compliance-heavy deployments trace kernel blobs to sources and licenses.

## References
- `docs/PRODUCER_GUIDE.md`
EOF
)"

echo "Done. Verify: gh repo view ${REPO} --json description,repositoryTopics"
echo "Issues: gh issue list --repo ${REPO} --label roadmap"
