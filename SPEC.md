# CLF (Coelanox Library File) — Full Specification

## 1. Purpose and role

CLF is the **standard binary format** for shipping **pre-compiled hardware kernels** in the Coelanox ecosystem. It is a **static kernel archive**: a single file that acts as a warehouse of optimized machine-code blobs, each identified by a numeric **op_id**. The Coelanox Packager does not interpret or execute these blobs; it only **looks them up by op_id** and **copies** them into the container’s code section at **package time**. There is no runtime code generation and no loading of kernel libraries at inference. The runtime runs code that was fully embedded in the `.cnox` container when it was built.

**Why CLF exists:**

- Avoid dependency/DLL hell: no .so version skew; kernels are baked into the container at package time.
- Static linking: only the kernels required by the model (by walking the IR and op_ids) are extracted and embedded.
- Versioning safety: host or SDK updates do not change an already-built container.
- Clear contract: one file per backend; op_id → blob; Coelanox never sees source; kernel providers never see IR.

**Where CLF fits:**

- **Codegen / backend (OpType / BackendTranslator):** CLF is an alternative backend source. When the backend for a target is `BackendKind::Clf`, the packager opens the `.clf` at the path registered by the backend loader, reads the manifest, and for each node in the optimized IR (in execution order) maps `OpType` → `op_id`, gets the blob for that `op_id` from the CLF, and appends it to the code section. So CLF **supplies the machine code** that would otherwise come from a BackendTranslator.
- **Memory HAL:** The code section (built in part or entirely from CLF blobs) is what the runtime allocates or backs using the Memory HAL (e.g. executable region). So CLF-derived bytes are the **content** that ends up in Memory HAL–managed regions.
- **Protection HAL:** The region(s) that hold CLF-derived code (and optionally weights/scratch) are the ones the Protection HAL configures (e.g. code RX, weights RO). So CLF-derived content defines **which regions** get which protection.

---

## 2. Workflow (end-to-end)

**Producer side (kernel provider):**

1. Implement kernels (e.g. C++/assembly) for the Coelanox op set (op_ids).
2. Compile to object code (e.g. `.o`).
3. Run **coelanox-packer** (or equivalent): input = object files + manifest (op_id → which symbol or blob); output = one `.clf` file.
4. The packer strips symbols (obfuscation), lays out blobs, builds the manifest (op_id → offset, size), writes header + manifest + blob store + optional signature.
5. Ship the single `.clf` file (e.g. to the platform or installer).

**Consumer side (Coelanox Packager):**

1. Backend loader discovers `.clf` files in backend search paths and registers them as backends with `BackendKind::Clf` and a path.
2. When generating machine code for a target that has a CLF backend, the packager opens the `.clf` at that path.
3. It walks the optimized IR’s nodes in execution order. For each node it maps `OpType` to the canonical **op_id** (via a fixed registry).
4. It looks up that op_id in the CLF manifest, reads the blob at (offset, size), and appends it to the code buffer.
5. The resulting buffer is the container’s code section (possibly combined with other backends or stubs).
6. That code section is written into the `.cnox` container; at runtime it is loaded into memory (Memory HAL) and protected (Protection HAL) as code.

**Runtime:**

- Does **not** read `.clf` files. It loads the `.cnox` container; the code section is already the concatenated kernel blobs. Memory HAL allocates/backs the region; Protection HAL sets permissions (e.g. RX for code).

---

## 3. File format (binary layout)

**Overall structure:**

```
[Header][Manifest][Blob store][Optional signature]
```

**Byte order:** Little-endian for all multi-byte fields.

### 3.1 Header (fixed + variable)

| Field           | Size   | Type / meaning                                      |
|-----------------|--------|-----------------------------------------------------|
| Magic           | 4 B    | `CLF1` (0x43 0x4C 0x46 0x31)                        |
| Version         | 1 B    | Format version (e.g. 1)                             |
| Vendor length   | 2 B    | Little-endian u16 (N)                               |
| Vendor          | N B    | UTF-8 identifier (display/audit only)                |
| Padding         | 0–7 B  | Optional: align next section to 8 bytes (reserved)   |

- Header size = 4 + 1 + 2 + N (+ padding if alignment is used).
- **Version** is reserved for future format changes; readers must reject unsupported versions.
- **Reserved / future:** Extra header bytes may be added later; readers should skip unknown header extension if version allows.

### 3.2 Manifest

| Field        | Size   | Type / meaning                                      |
|--------------|--------|-----------------------------------------------------|
| Num entries  | 2 B    | Little-endian u16                                   |
| Entries      | 10 B each | For each: **op_id** (2 B LE), **offset** (4 B LE), **size** (4 B LE) |

- **Offset** and **size** are relative to the **start of the blob store** (first byte after the manifest).
- No duplicate op_ids; op_id is the key.
- Entries may appear in any order; lookup is by op_id.

### 3.3 Blob store

- Contiguous byte range immediately after the manifest.
- For entry `i`: blob starts at `blob_store_start + manifest[i].offset`, length `manifest[i].size`.
- Blobs are opaque binary (e.g. machine code for one op). Alignment/padding between blobs is optional and defined by the producer.

### 3.4 Signature (optional)

- At **end of file**:
  - 4 bytes: signature magic `SIG0` (0x53 0x49 0x47 0x30).
  - 32 bytes: SHA-256 hash of **everything before the signature** (header + manifest + blob store).
- Used to verify integrity and origin. A reader may require a valid signature before use.
- If present, the total file length is header_size + manifest_size + blob_store_size + 4 + 32.

---

## 4. Extension and reserved points

- **Version byte:** Readers must check version; reject if greater than supported.
- **Reserved header bits/bytes:** Future header fields may be added; document in spec revisions.
- **op_id 0:** Reserved (unknown/custom). op_id 100+ reserved or custom per agreement between producer and consumer.

---

## 5. Security and IP

- **Producer:** Ships only the `.clf` (binary). Coelanox never sees source or object files.
- **Consumer:** Only performs op_id → blob lookup and copy; no parsing or execution of kernel code.
- **IR/model:** Never sent to the producer; the producer only needs the op_id list from the public registry.
- **Signature:** Optional but recommended so the platform can verify that a `.clf` was not tampered with and (if desired) that it comes from a trusted source.
