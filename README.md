<div align="center">
  <img src="assets/coelanox-logo.png" alt="Coelanox" width="440">
</div>

# Coelanox Library File (CLF)

[![Crates.io](https://img.shields.io/crates/v/clf)](https://crates.io/crates/clf)
[![CI](https://github.com/Coelanox/CLF/actions/workflows/ci.yml/badge.svg)](https://github.com/Coelanox/CLF/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**CLF is the canonical binary format for packaging pre-compiled hardware kernels in the Coelanox stack.** A single file holds machine-code blobs keyed by numeric `op_id`; runtimes and packagers consume the same layout, with optional SIG0 + SHA-256 integrity verification and a forward path toward stronger authenticity policies.

This repository ships the **reference Rust implementation** (library + `clf` CLI), the **on-disk specification**, and **producer/consumer documentation** intended for security review and compliance-oriented workflows.

## Overview

- **Static archive:** one container, many blobs—no runtime code generation or kernel loading from CLF itself.
- **Deterministic consumption:** the packager resolves `op_id` → blob at package time; the runtime executes the resulting code regions.
- **Integrity today:** optional `verify_signature()` / `verify_with_policy(IntegrityOnly)` for the SIG0 + SHA-256 trailer.
- **Authenticity roadmap:** `RequireAuthenticity` and `--verify-policy require-authenticity` are intentionally fail-closed until authenticated signatures are defined in the format.

## What this repository provides

| Area | Contents |
|------|----------|
| **Reader (Rust)** | `ClfReader::open`, `get_blob(op_id)`, `build_code_section` with missing-op policy, optional `verify_with_policy` |
| **Packer (Rust / CLI)** | `clf` / `coelanox-packer`: `--from` TOML manifests, `--inspect --json`, `--verify`, `--write-sidecar`, `--dry-run` |
| **Registry** | Canonical `op_id` mapping and docs in [docs/op_ids.md](docs/op_ids.md) |
| **Specification** | [SPEC.md](SPEC.md) — binary layout, `kind`, alignment, signatures, versioning |

## Quickstart

```bash
# 1) Build a CLF from op_id:path pairs
clf -o out.clfc 1:add.bin 50:matmul.bin

# 2) Inspect header + manifest
clf -i out.clfc

# 3) Verify signed integrity (SIG0 + SHA-256)
clf --verify out.clfc --verify-policy integrity-only
```

`require-authenticity` exists as a forward-compatibility policy and fails closed until the format supports authenticated signatures.

## CLF format family

`kind` selects runtime semantics; the file extension supports discovery:

| Kind | Role | Typical extension |
|------|------|-------------------|
| **CLFC** | Compute kernels (`op_id` → blob) | `.clfc` |
| **CLFMM** | Memory movement | `.clfmm` |
| **CLFMP** | Memory protection | `.clfmp` |
| **CLFE** | Executor / dispatcher plans ([docs/clfe.md](docs/clfe.md)) | `.clfe` |

## Role in the Coelanox stack

- **Codegen / backend:** when the backend is CLF-backed, the packager maps IR `OpType` → `op_id` and emits the corresponding blob into the code section.
- **Memory HAL:** CLF-derived bytes back executable (or other) regions allocated through the Memory HAL.
- **Protection HAL:** the Protection HAL applies attributes (for example code RX, read-only weights) to the regions that contain those bytes.

## Verification model

- **Integrity:** `verify_signature()` and `verify_with_policy(IntegrityOnly)` validate the optional SIG0 + SHA-256 tail.
- **Authenticity (planned):** `verify_with_policy(RequireAuthenticity)` remains explicit and unsupported until authenticated trailer design lands.

Details: [docs/SIGNING.md](docs/SIGNING.md).

## Documentation index

| Document | Purpose |
|----------|---------|
| [SPEC.md](SPEC.md) | Full binary specification (layout, `kind`, CLFE, signatures) |
| [docs/PRODUCER_GUIDE.md](docs/PRODUCER_GUIDE.md) | Producing valid archives, `--kind`, signing |
| [docs/CONSUMER_NOTE.md](docs/CONSUMER_NOTE.md) | Consumer behavior: discovery, target match, missing-op policy, HALs |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Internal structure and verification APIs |
| [docs/SIGNING.md](docs/SIGNING.md) | Signing and policy semantics |
| [docs/RELEASE.md](docs/RELEASE.md) | Release process and published assets |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

## Installation

### Vendors (release binaries)

```bash
bash scripts/install.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

Defaults: repository `Coelanox/CLF`, version `latest`, install paths `~/.local/bin/clf` (Linux) or `%USERPROFILE%\.local\bin\clf.exe` (Windows). Override with `CLF_REPO`, `CLF_VERSION`, `CLF_INSTALL_DIR`.

Expected assets include `clf-x86_64-unknown-linux-gnu.tar.gz`, `clf-x86_64-pc-windows-msvc.zip`, and `SHA256SUMS`.

### Developers

```bash
cargo install clf
```

From a clone: `cargo build` (MSRV in `Cargo.toml`), `cargo test --all-features`, `cargo clippy` as in CI. Pack example: `cargo run --bin clf -- -o out.clf 1:blob1.bin 50:blob50.bin`. Fuzzing: `cd fuzz && cargo fuzz run clf_open` ([cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for workflow, CI parity, and maintainer GitHub setup.
