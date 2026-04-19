# How to produce a valid .clf (producer guide)

This guide describes how to build a `.clf` file so that the Coelanox Packager or runtime can use it for a given target. CLF supports four **kinds** (Compute, Memory Movement, Memory Protection, Executor); use `--kind` to select the role and output the correct extension (e.g. `.clfc`, `.clfe`).

## CLF kinds (family)

| Kind | Extension | Use |
|------|-----------|-----|
| **Compute** | `.clfc` | Kernel blobs keyed by op_id. Packager builds the container code section from these. Default. |
| **Memory Movement** | `.clfmm` | Blobs for allocate / deallocate / move (Memory HAL). |
| **Memory Protection** | `.clfmp` | Blobs for configure_region / enable / disable (Protection HAL). |
| **Executor** | `.clfe` | Executor blob (plan runner) and/or plan data. Runtime or packager uses it to run the graph by dispatching to CLFC blobs. See [clfe.md](clfe.md) for plan format and dispatch contract. |

Producers of **CLFC** (compute) typically also provide a matching **CLFE** (executor) for the same target so the host can “load and call once.”

## Inputs

- **Op_id → blob:** For each op you support, you need the canonical **op_id** (see [op_ids.md](op_ids.md)) and the compiled machine-code blob (e.g. from `.o` or raw binary). For Executor (CLFE), you may ship a single executor blob (e.g. op_id 1 = executor entry).
- **Optional:** Vendor string (display/audit), **target** (e.g. `CPU`, `GPU`, `CDNA`) so the packager can match this CLF to a target, **blob alignment** (e.g. 16 for code), **kind** (compute | memory-movement | memory-protection | executor), and **signature** (SHA-256 of file minus signature block).

## Using the packer CLI (`clf` / `coelanox-packer`)

The packer is open source so producers can audit it (no exfiltration of code). The **`clf`** and **`coelanox-packer`** commands are the **same program** (install both with `cargo install clf`).

```text
clf [OPTIONS] -o <OUT.clf> <op_id:path> [op_id:path ...]
clf --from pack.toml -o <OUT.clf>   # batch manifest (TOML)
clf -i <FILE.clf>                  # read-only: header + manifest table
clf -i <FILE.clf> --json           # machine-readable inspect
clf --verify <FILE.clf>            # SIG0 + SHA-256 only (exit 0/1)
```

Run `clf --help` (or `coelanox-packer --help`) for the full option list and examples.

**Options (pack):**

- `--output`, `-o` — Output path (required when packing). Use extension `.clfc`, `.clfmm`, `.clfmp`, or `.clfe` to match `--kind`.
- `--vendor <string>` — Vendor identifier (optional).
- `--target <string>` — Target/architecture (e.g. CPU, GPU, CDNA). Packager uses this to match CLF to target (optional).
- `--kind <compute|memory-movement|memory-protection|executor>` — File kind. Aliases: `c`, `mm`, `mp`, `e`. Default: compute. Writes the Kind byte in the v2 header; consumers use it for discovery and routing.
- `--align <0–255>` — Blob alignment in bytes (e.g. 16 for code). 0 = no alignment (optional).
- `--sign` — Append SIG0 + SHA-256 of file (optional; recommended for integrity).
- `--from <FILE.toml>` — Load entries and defaults from a TOML manifest (see below). CLI flags override manifest fields when set.
- `--dry-run` — Validate blobs and print a summary; do not write a `.clf`.
- `--write-sidecar` — After a successful pack, write `<output>.meta.json` with per-blob SHA-256 and optional `symbol` / `notes` from the manifest.

**Inspect (read-only):**

- `--inspect`, `-i <FILE>` — Print format version, kind, vendor, target, alignment, blob store layout, signature presence, and a manifest table (`op_id`, offset, size). Does not hash blobs unless you add `--verify-signature` (checks SIG0 + SHA-256).
- `--json` (with `-i`) — Print the same information as one JSON object on stdout (stable for CI).

**Verify only:**

- `--verify <FILE>` — Exit with status 0 if a SIG0 block is present and the SHA-256 matches; non-zero otherwise. Useful in pipelines.
- `--verify-policy <integrity-only|require-authenticity>` — Applies to `--verify`, or `--inspect` when used with `--verify-signature`. `require-authenticity` is reserved for future authenticated signatures and currently fails closed with an explicit unsupported error.
- Recommended today: `--verify-policy integrity-only` for explicit CI intent.

**Arguments (pack):** Each `op_id:path` gives one op_id and the path to the raw blob (or object file). Example:

```text
coelanox-packer --output gpu.clf --target GPU --align 16 --sign \
  1:add.o 10:relu.o 50:matmul.o
```

The tool reads each file as a raw blob and writes header + manifest + blob store (+ optional signature) to `gpu.clf`.

## Installing `clf` for vendor workflows

For non-Rust environments, use release binaries with installer scripts:

- From a git checkout: Linux `bash scripts/install.sh`, Windows `powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1`.
- From a **GitHub release** archive: the same scripts are included at the root as `install.sh` / `install.ps1` next to the binary.

Defaults:
- repo: `Coelanox/CLF`
- version: `latest`
- install dir:
  - Linux: `~/.local/bin`
  - Windows: `%USERPROFILE%\.local\bin`

You can override repo/version/install-dir with `CLF_REPO`, `CLF_VERSION`, `CLF_INSTALL_DIR`.

Release assets (each archive includes the binary plus `install.sh` or `install.ps1` at the root):
- `clf-x86_64-unknown-linux-gnu.tar.gz` (optional arm64 variant: `clf-aarch64-unknown-linux-gnu.tar.gz`)
- `clf-x86_64-pc-windows-msvc.zip` (optional arm64 variant: `clf-aarch64-pc-windows-msvc.zip`)

Release automation details: [RELEASE.md](RELEASE.md).

## TOML pack manifest (`--from`)

Use a manifest when you have many blobs or want to attach **optional metadata** for the JSON sidecar (`symbol`, `notes`). Paths are resolved relative to the **current working directory** when you invoke `coelanox-packer` (not relative to the manifest file unless you use absolute paths).

```toml
vendor = "my-org"
target = "GPU"
kind = "compute"
align = 16
sign = true

[[blobs]]
op_id = 1
path = "build/add.bin"
symbol = "_clf_add"
notes = "AVX2 kernel"

[[blobs]]
op_id = 50
path = "build/matmul.bin"
```

Then:

```text
coelanox-packer --from pack.toml -o out.clfc --write-sidecar
```

Produces `out.clfc` and `out.clfc.meta.json` (per-blob SHA-256 + labels). The **`.meta.json` file is not part of the CLF format**; Coelanox can ignore it. It is for audits, reproducibility, and CI.

## Output layout

- **Header:** Magic, version, vendor length + vendor, target length + target, blob alignment, kind (v2).
- **Manifest:** Num entries, then for each entry: op_id (4 B LE), offset (4 B LE), size (4 B LE) into blob store.
- **Blob store:** Blobs concatenated (with optional padding to `--align`).
- **Optional signature:** 4 B `SIG0` + 32 B SHA-256 of everything before.

## Op IDs

Use the canonical [op_id registry](op_ids.md). For custom ops, use op_ids in **256–2³²−1** (u32::MAX) so they do not collide with the canonical set.

## Signing (optional)

If you use `--sign`, the packer appends a 36-byte block (SIG0 + SHA-256). Consumers can call `verify_signature()` before use. Verification keys / PKI are reserved for future; in v1, verification is “hash matches.”

## Security bounds

The reference reader applies defensive resource bounds when parsing untrusted files. In particular, `vendor` and `target` header fields are currently capped at **64 KiB** each, even though the on-wire field lengths are encoded as u32.

## Library API

You can also build a .clf from code using the `clf` crate:

- `pack_clf(&mut out, &[(op_id, blob), ...], &PackOptions)` — writes header + manifest + blob store; returns bytes written.
- `append_signature(&mut out, data_len)` — call after `pack_clf` if `PackOptions.sign` is true.
- `parse_op_blob_arg("12:path/to/blob.bin")` — parses the same `op_id:path` tokens as the CLI (first `:` separates id from path).

`PackOptions` includes `vendor`, `target`, `blob_alignment`, `kind`, `version`, `sign` (see `Default`).

`ClfReader` exposes `manifest_entries()`, `blob_store_offset()`, `blob_store_len()`, `signature_block_present()`, and `blobs_iter()` for tooling.

With the **`serde` feature** (enabled by default): `load_pack_manifest`, `write_sidecar_json`, and `Serialize`/`Deserialize` on header/manifest types for custom pipelines.
