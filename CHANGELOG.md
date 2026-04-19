# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) for the **Rust crate** version. The **CLF on-disk format** version is defined separately (`CLF_VERSION` in `src/format.rs` and [SPEC.md](SPEC.md)).

## [Unreleased]

### Changed

- GitHub release archives (`.zip` / `.tar.gz`) now include `install.ps1` / `install.sh` at the archive root alongside the binary (see `.github/workflows/release.yml`).

## [0.1.1] - 2026-04-19

### Changed

- **`scripts/install.ps1`:** after a successful install, append the install directory to the **per-user** `Path` when it is not already present (open a new terminal to pick it up). Set `CLF_NO_PATH=1` to skip.
- **`scripts/install.sh`:** optional `CLF_ADD_TO_PATH=1` appends a guarded `PATH` export to `~/.profile` when the directory is not already mentioned there; default remains print-only instructions.

## [0.1.0]

### Added

- **README presentation:** centered Coelanox logo (`assets/coelanox-logo.png`), crates.io / CI / license badges, structured overview and documentation index tables.

- **Contributor onboarding:** [CONTRIBUTING.md](CONTRIBUTING.md) and `scripts/gh-bootstrap-repo.sh` for one-time GitHub description, topics, and roadmap issues (maintainers).

- **`clf` binary**: same implementation as `coelanox-packer` (short name for `cargo install` / PATH). `cargo install clf` installs both `clf` and `coelanox-packer`.
- `rust-version` (MSRV) in `Cargo.toml`.
- Optional `serde` feature: `Serialize`/`Deserialize` for `ClfKind`, `ClfHeader`, `ManifestEntry` (when feature enabled).
- TOML pack manifests (`--from`) and optional JSON sidecars (`--write-sidecar`) with per-blob SHA-256 and optional labels.
- CLI: `--verify`, `--inspect --json`, `--dry-run`.
- `ClfReader::blobs_iter` and `ClfReaderFromBytes::blobs_iter`.
- Fuzz target under `fuzz/` (`cargo fuzz run clf_open` after `cargo install cargo-fuzz`).
- CI workflow (test, clippy, fmt, `cargo deny`, fuzz crate build).
- `deny.toml` for `cargo deny`.
- Docs: `docs/ARCHITECTURE.md`, `docs/SIGNING.md`, this file.

### Notes

- Compression of the blob store is intentionally **not** implemented, to keep a single linear layout compatible with existing Coelanox tooling.
