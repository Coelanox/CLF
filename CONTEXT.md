# CONTEXT

Last updated: 2026-04-19

## Current State

- README presents the project with centered Coelanox logo (`assets/coelanox-logo.png`), badges, and structured overview for public visibility; technical content preserved.
- Contributor-facing docs: `CONTRIBUTING.md` describes PR workflow and CI parity; `scripts/gh-bootstrap-repo.sh` sets GitHub description/topics and opens roadmap issues (requires `gh` auth on maintainer machine).
- CLF parsing is now strict for v2+ `kind` bytes (`0..=3` only).
- Reader parsing now includes guardrails against oversized header text fields and impossible manifest counts.
- CLI sidecar generation was optimized to avoid quadratic lookup behavior.
- Verification policy scaffolding is in place without changing CLF wire format.
- CLI now rejects `--verify-policy` unless a verification flow is requested.
- Core docs were refreshed for first-release presentation quality (quickstart + unified verification language).
- Cross-platform installer scripts were added for vendor onboarding (Linux + Windows release binaries).
- GitHub release automation now publishes installer-compatible assets and checksums.

## Changes Applied

- `README.md`, `assets/coelanox-logo.png` (renamed from designer export `Group 1 (1).png`)
  - Professional landing layout: logo, crates/CI/license badges, tables for format family and docs index.

- `CONTRIBUTING.md`, `scripts/gh-bootstrap-repo.sh`
  - Contribution guidelines aligned with CI (`fmt`, `clippy`, `test`, `cargo deny`).
  - Maintainer script to apply GitHub repo description, topics (`ai-inference`, `auditable-ai`, `rust`, `onnx`, `compliance`), and labeled roadmap issues (run locally with `gh`).

- `src/format.rs`
  - Replaced permissive kind parsing with `ClfKind::try_from_byte(u8) -> Option<ClfKind>`.

- `src/reader.rs`
  - Added `MAX_HEADER_TEXT_LEN` safety cap for header text fields.
  - Added `read_len_prefixed_utf8` helper with explicit length bound checks before allocation.
  - Added distinct errors:
    - `InvalidTargetUtf8`
    - `InvalidKindByte(u8)`
  - Updated v2+ parsing to strictly reject unknown kind bytes.
  - Added early manifest size validation before allocating/parsing entries for both file-backed and in-memory readers.

- `src/bin/coelanox-packer.rs`
  - Sidecar generation now builds a `HashMap<u32, &[u8]>` once and performs O(1) lookup per blob.
  - Added `--verify-policy` flag (`integrity-only` default, `require-authenticity` reserved).
  - Enforced argument semantics: `--verify-policy` is valid only with `--verify`, or `--inspect` when `--verify-signature` is set.

- `tests/reader_tests.rs`
  - Added regressions for:
    - invalid v2 kind rejection
    - oversized vendor length rejection
    - manifest count overflow rejection

- `src/reader.rs` / `src/lib.rs`
  - Added `VerificationPolicy` enum with:
    - `IntegrityOnly` (current SIG0 + SHA-256 verification)
    - `RequireAuthenticity` (explicit unsupported placeholder)
  - Added `ClfReader::verify_with_policy(policy)`.

- `tests/packer_tests.rs` / `tests/cli_packer.rs`
  - Added regressions proving:
    - `IntegrityOnly` succeeds on signed CLF.
    - `RequireAuthenticity` fails with explicit unsupported error until format-level auth exists.

- `docs/SIGNING.md`
  - Documented policy scaffold and CLI usage.
- `SPEC.md`, `README.md`, `docs/PRODUCER_GUIDE.md`
  - Aligned published behavior with implementation details:
    - reference-reader 64 KiB safety cap for `vendor`/`target` header fields
    - verification policy behavior and current `require-authenticity` fail-closed semantics
- Documentation refresh pass (presentability):
  - `README.md`: added 60-second quickstart and explicit verification model section.
  - `docs/ARCHITECTURE.md`: added policy semantics and `verify_with_policy` mention.
  - `docs/CONSUMER_NOTE.md`: clarified integrity vs future authenticity verification APIs.
  - `docs/PRODUCER_GUIDE.md`: added explicit recommendation for `--verify-policy integrity-only`.
  - `docs/SIGNING.md`: aligned API references with policy-based verification.
- Installer distribution support:
  - Added `scripts/install.sh` (Linux) and `scripts/install.ps1` (Windows).
  - Updated `README.md` and `docs/PRODUCER_GUIDE.md` with vendor-first install guidance, environment overrides, and expected release asset names.
- Release automation:
  - Added `.github/workflows/release.yml` (tag-triggered release workflow) building:
    - `clf-x86_64-unknown-linux-gnu.tar.gz`
    - `clf-x86_64-pc-windows-msvc.zip`
    - `SHA256SUMS`
  - Added `docs/RELEASE.md` runbook describing tagging, published assets, and post-release checks.

## Security Notes

- Main DoS vector from untrusted length-driven allocation in reader parsing has been reduced with explicit caps and structure-size validation.
- Unknown on-disk `kind` bytes are treated as malformed input instead of silently coercing to compute.
- Policy-driven verification API allows consumers to adopt "require authenticity" semantics now and fail closed until authenticated signatures are implemented.

## Release review (2026-04-19)

- **Fixed:** `release.yml` build job now checks out `inputs.tag` when using `workflow_dispatch`, so published binaries match the tag (previously built default-branch HEAD).
- **Flagged (unchanged):** Linux install script maps `aarch64` to `clf-aarch64-unknown-linux-gnu.tar.gz`, but the release workflow only builds x86_64 assets until an arm64 matrix row exists — ARM Linux users get a failed download until then.
- **Note:** `SHA256SUMS` step assumes at least one `.tar.gz` and one `.zip` artifact; true for the current matrix.

## Open Follow-ups

- Consider adding fuzz corpus entries for malformed length/count/kind combinations.
- If future requirements need larger vendor/target metadata, adjust `MAX_HEADER_TEXT_LEN` with accompanying threat review.
- Define authenticated trailer design (algorithm ID, key ID, signature bytes) and key-management policy before enabling `RequireAuthenticity`.
