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

## Using coelanox-packer (CLI)

The packer is open source so producers can audit it (no exfiltration of code).

```text
coelanox-packer [OPTIONS] --output <OUT.clf> <op_id:path> [op_id:path ...]
```

**Options:**

- `--output`, `-o` — Output path (required). Use extension `.clfc`, `.clfmm`, `.clfmp`, or `.clfe` to match `--kind`.
- `--vendor <string>` — Vendor identifier (optional).
- `--target <string>` — Target/architecture (e.g. CPU, GPU, CDNA). Packager uses this to match CLF to target (optional).
- `--kind <compute|memory-movement|memory-protection|executor>` — File kind. Aliases: `c`, `mm`, `mp`, `e`. Default: compute. Writes the Kind byte in the v2 header; consumers use it for discovery and routing.
- `--align <0–255>` — Blob alignment in bytes (e.g. 16 for code). 0 = no alignment (optional).
- `--sign` — Append SIG0 + SHA-256 of file (optional; recommended for integrity).

**Arguments:** Each `op_id:path` gives one op_id and the path to the raw blob (or object file). Example:

```text
coelanox-packer --output gpu.clf --target GPU --align 16 --sign \
  1:add.o 10:relu.o 50:matmul.o
```

The tool reads each file as a raw blob and writes header + manifest + blob store (+ optional signature) to `gpu.clf`.

## Output layout

- **Header:** Magic, version, vendor length + vendor, target length + target, blob alignment, kind (v2).
- **Manifest:** Num entries, then for each entry: op_id (4 B LE), offset (4 B LE), size (4 B LE) into blob store.
- **Blob store:** Blobs concatenated (with optional padding to `--align`).
- **Optional signature:** 4 B `SIG0` + 32 B SHA-256 of everything before.

## Op IDs

Use the canonical [op_id registry](op_ids.md). For custom ops, use op_ids in **256–2³²−1** (u32::MAX) so they do not collide with the canonical set.

## Signing (optional)

If you use `--sign`, the packer appends a 36-byte block (SIG0 + SHA-256). Consumers can call `verify_signature()` before use. Verification keys / PKI are reserved for future; in v1, verification is “hash matches.”

## Library API

You can also build a .clf from code using the `clf` crate:

- `pack_clf(&mut out, &[(op_id, blob), ...], &PackOptions)` — writes header + manifest + blob store; returns bytes written.
- `append_signature(&mut out, data_len)` — call after `pack_clf` if `PackOptions.sign` is true.

`PackOptions` includes `vendor`, `target`, `blob_alignment`, `version`, `sign`.
