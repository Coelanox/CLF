# CLF (Coelanox Library File)

CLF is the standard binary format for shipping pre-compiled hardware kernels in the Coelanox ecosystem. It is a static kernel archive: one file containing optimized machine-code blobs keyed by numeric op_id. The packager looks up blobs by op_id and copies them into the container at package time; there is no runtime code generation or kernel loading.

## Quickstart (60 seconds)

```bash
# 1) Build a CLF from op_id:path pairs
clf -o out.clfc 1:add.bin 50:matmul.bin

# 2) Inspect header + manifest
clf -i out.clfc

# 3) Verify signed integrity (SIG0 + SHA-256)
clf --verify out.clfc --verify-policy integrity-only
```

`require-authenticity` exists as a forward-compatibility policy and intentionally fails today until authenticated signatures are added to the format.

**CLF family (kind = what the runtime uses; extension = for discovery):**

- **CLFC** (Compute): op_id → kernel blobs. Extension `.clfc`.
- **CLFMM** (Memory movement): memory copy/move blobs. Extension `.clfmm`.
- **CLFMP** (Memory protection): region protection blobs. Extension `.clfmp`.
- **CLFE** (Executor): plan runner / dispatcher; see [docs/clfe.md](docs/clfe.md). Extension `.clfe`.

**Three uses for the three HALs (compute / memory / protection):**

- **Codegen / backend:** CLF supplies the machine code that would otherwise come from a BackendTranslator. When the backend is `BackendKind::Clf`, the packager opens the `.clf`, maps each IR node’s OpType to op_id, and appends the corresponding blob to the code section.
- **Memory HAL:** The code section (built from CLF blobs) is what the runtime allocates or backs via the Memory HAL (e.g. executable region). CLF-derived bytes are the content of those regions.
- **Protection HAL:** The region(s) holding the code section are the ones the Protection HAL configures (e.g. code RX, weights RO). CLF-derived content defines which regions get which protection.

**Spec and docs:**

- [SPEC.md](SPEC.md) — full format specification (binary layout, target, alignment, **kind** including Executor, signature, version policy). Section 1 and 3.1.1 describe the CLF family (CLFC, CLFMM, CLFMP, **CLFE**).
- [docs/op_ids.md](docs/op_ids.md) — canonical op_id registry (single source of truth; custom range 256–65535; stability).
- [docs/clfe.md](docs/clfe.md) — **CLFE (Executor):** execution plan format and dispatch contract. Referenced from the spec.
- [docs/PRODUCER_GUIDE.md](docs/PRODUCER_GUIDE.md) — how to produce a valid .clf (packer usage, op_ids, **--kind**, optional signing).
- [docs/CONSUMER_NOTE.md](docs/CONSUMER_NOTE.md) — how the packager and runtime use CLF (CLF family, discovery, target match, op_id lookup, missing-op policy, HALs).

**This repo provides:**

- **Reader** (Rust): `ClfReader::open(path)`, `get_blob(op_id)`, `build_code_section(op_ids, MissingOpIdPolicy)` (Fail or Skip when op_id missing), optional `verify_signature()` or policy-based `verify_with_policy()` before use. Header includes `target` and `blob_alignment` for packager matching and layout.
- **Packer** (Rust): **`clf`** CLI (same program as `coelanox-packer`; pack with **`--from`** TOML manifest, **`--dry-run`**, **`--verify`**, **`--inspect --json`**, **`--write-sidecar`**) and `pack_clf` / `append_signature` / `parse_op_blob_arg` / `load_pack_manifest` / sidecar helpers. Open source so producers can audit it.
- **Op ID registry**: `op_type_to_clf_id(OpType)`, `clf_id_to_op_type(op_id)` and canonical op_id list in code and docs/op_ids.md (custom range 256–u32::MAX).

## Verification model

- **Integrity today:** `verify_signature()` and `verify_with_policy(IntegrityOnly)` validate the optional SIG0 + SHA-256 tail.
- **Authenticity later:** `verify_with_policy(RequireAuthenticity)` and CLI `--verify-policy require-authenticity` are intentionally fail-closed placeholders until authenticated signatures are implemented.

See [docs/SIGNING.md](docs/SIGNING.md) for CLI examples and rollout semantics.

## Installation

### Vendors (recommended)

Use release binaries with the provided install scripts:

```bash
bash scripts/install.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

Script defaults:
- repository: `Coelanox/CLF`
- version: `latest`
- install path:
  - Linux: `~/.local/bin/clf`
  - Windows: `%USERPROFILE%\.local\bin\clf.exe`

Overrides are supported via environment variables: `CLF_REPO`, `CLF_VERSION`, `CLF_INSTALL_DIR`.

Release assets expected by scripts:
- Linux: `clf-x86_64-unknown-linux-gnu.tar.gz` (and optionally `clf-aarch64-unknown-linux-gnu.tar.gz`)
- Windows: `clf-x86_64-pc-windows-msvc.zip` (and optionally `clf-aarch64-pc-windows-msvc.zip`)

### Developers

```bash
cargo install clf
```

Build: `cargo build` (MSRV in `Cargo.toml` `rust-version`). Tests: `cargo test`. Pack: `cargo run --bin clf -- -o out.clf 1:blob1.bin 50:blob50.bin` (or `--bin coelanox-packer`). After `cargo install clf`, use the `clf` command on your `PATH`. Inspect: `clf -i out.clf`. Verify SIG0: `clf --verify out.clf` (or explicitly `--verify-policy integrity-only`). Optional TOML manifest and JSON sidecar: [docs/PRODUCER_GUIDE.md](docs/PRODUCER_GUIDE.md). Fuzzing: `cd fuzz && cargo fuzz run clf_open` (requires [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)). Changelog: [CHANGELOG.md](CHANGELOG.md). Architecture overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Release process and asset contract: [docs/RELEASE.md](docs/RELEASE.md).
