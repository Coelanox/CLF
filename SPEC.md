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

| Field            | Size   | Type / meaning                                      |
|------------------|--------|-----------------------------------------------------|
| Magic            | 4 B    | `CLF1` (0x43 0x4C 0x46 0x31)                        |
| Version          | 1 B    | Format version (1 = legacy, 2 = with kind)         |
| Vendor length    | 4 B    | Little-endian u32 (N)                               |
| Vendor           | N B    | UTF-8 identifier (display/audit only)               |
| Target length    | 4 B    | Little-endian u32 (M); 0 = no target                |
| Target           | M B    | UTF-8 target/architecture (e.g. "CPU", "GPU", "CDNA"). Packager uses this to match CLF to target. |
| Blob alignment   | 1 B    | Alignment in bytes for blobs in blob store (0 = none). Producer pads each blob to this alignment (e.g. 16 for code). |
| Kind             | 1 B    | *(v2 only)* File kind: 0 = Compute, 1 = MemoryMovement, 2 = MemoryProtection. Source of truth for the file's role. |

- **Header size:** Version 1: 4 + 1 + 4 + N + 4 + M + 1 bytes. Version 2: + 1 byte (kind) = 4 + 1 + 4 + N + 4 + M + 1 + 1 bytes.
- **Version policy:** Version 1 = layout without kind. Version 2 = layout with kind. Readers must reject version &gt; supported. No renumbering of existing fields.
- **Kind (v2):** 0 = Compute, 1 = MemoryMovement, 2 = MemoryProtection. For v1 files, kind is absent and defaults to Compute (backwards compatibility).
- **Validate on open:** Consumers may validate that the header kind matches the expected kind (e.g. when opening a `.clfmm` file, expect MemoryMovement); reject if mismatch.
- **Target:** Optional. If target length is 0, no target bytes follow. Enables the packager to select a CLF by target (e.g. from header) in addition to filename (e.g. `cpu.clf`, `gpu.clf`).
- **Blob alignment:** 0 = blobs stored back-to-back. If &gt; 0, each blob is padded to a multiple of this value in the blob store; manifest offset/size refer to the stored (padded) layout.

### 3.1.1 File extensions (discovery and routing)

| Extension | Meaning                                      | Kind (when absent, treat as Compute) |
|-----------|----------------------------------------------|--------------------------------------|
| `.clfc`   | Coelanox Library File Compute                | Compute                              |
| `.clfmm`  | Coelanox Library File Memory Movement        | MemoryMovement                       |
| `.clfmp`  | Coelanox Library File Memory Protection      | MemoryProtection                     |
| `.clf`    | Legacy; compute-only                         | Compute (backwards compatibility)   |

### 3.2 Manifest

| Field        | Size   | Type / meaning                                      |
|--------------|--------|-----------------------------------------------------|
| Num entries  | 4 B    | Little-endian u32                                   |
| Entries      | 12 B each | For each: **op_id** (4 B LE), **offset** (4 B LE), **size** (4 B LE) |

- **Manifest size:** 4 + num_entries × 12 bytes (num_entries field + all entries).
- **Offset** and **size** are relative to the **start of the blob store** (first byte after the manifest).
- No duplicate op_ids; op_id is the key.
- Entries may appear in any order; lookup is by op_id.

### 3.3 Blob store

- **Blob store start:** Byte offset from file start = header size + manifest size. v1: `4 + 1 + 4 + vendor_len + 4 + target_len + 1 + 4 + (num_entries × 12)`. v2: + 1 (kind) = `4 + 1 + 4 + vendor_len + 4 + target_len + 1 + 1 + 4 + (num_entries × 12)`.
- Contiguous byte range immediately after the manifest.
- For entry `i`: blob starts at `blob_store_start + manifest[i].offset`, length `manifest[i].size` (stored length; includes padding if blob alignment &gt; 0).
- Blobs are opaque binary (e.g. machine code for one op). If header blob alignment is &gt; 0, each blob is padded to that alignment; the reader returns the stored bytes (including padding).

### 3.4 Signature (optional)

- At **end of file**:
  - 4 bytes: signature magic `SIG0` (0x53 0x49 0x47 0x30).
  - 32 bytes: SHA-256 hash of **everything before the signature** (header + manifest + blob store).
- **Algorithm:** SHA-256. **Scope:** Whole file minus the 36-byte signature block (4 + 32). No keys or certificates in version 1; verification is “hash matches.” **Verification keys / PKI:** Reserved for future (e.g. v2 could add optional signature scheme with public keys).
- Used to verify integrity (and optionally origin). A reader may call `verify_signature()` before use and refuse to use the file if verification fails.
- If present, the total file length is header_size + manifest_size + blob_store_size + 4 + 32.

---

## 4. Limits and future-proofing

### 4.1 Current limits (version 1 & 2)

| What | Type / limit | Practical impact |
|------|----------------|------------------|
| **op_id** | u32 (0–2³²−1) | Billions of distinct op_ids; canonical uses 0–255, custom 256–2³²−1. |
| **Manifest entries** | u32 (num_entries) | Billions of blobs per .clf. |
| **Vendor / target length** | u32 each | Max 2³²−1 bytes per string. |
| **Blob offset / size** | u32 each | Max ~4 GiB per blob; blob store can be very large. Sufficient for any single kernel. |
| **Format version** | u8 | 256 versions; new layout = new version. |
| **Blob alignment** | u8 (0–255) | Alignment in bytes; 16–64 covers all common ISAs. |
| **Kind** | u8 (v2) | 0 = Compute, 1 = MemoryMovement, 2 = MemoryProtection. |

### 4.2 Future-proofing

- **Version policy:** Readers reject version &gt; supported. New layout = new version; **existing fields are not renumbered**. So v2 can add header fields, longer op_id, or new sections without breaking v1 readers (they simply refuse v2 files until updated).
- **Reserved / extension:** Spec allows future header fields and trailer extensions (e.g. new signature scheme, key ID, attestation) in new versions.
- **Op_id stability:** Canonical op_ids are stable; new ops get new ids. Custom range 256–u32::MAX avoids collision with future canonical ids.
- **Signature:** v1 = hash-only; verification keys / PKI reserved for future (e.g. v2 optional key ID or cert after the hash).

### 4.3 If limits are ever hit

- **Blob &gt; 4 GiB:** Not expected for kernel blobs; a future version could use u64 for offset/size.

---

## 5. Extension and reserved points (summary)

- **Version policy:** Version 1 = layout without kind (defaults to Compute). Version 2 = layout with kind. Reader rejects unknown version (e.g. version &gt; 2). New formats get a new version; existing fields are not renumbered.
- **Reserved header bits/bytes:** Future header fields may be added; document in spec revisions.
- **op_id 0:** Reserved (unknown/custom). **op_id 256–u32::MAX:** Custom range for producers; no collision with canonical registry (see op_id registry doc).

---

## 6. Security and IP

- **Producer:** Ships only the `.clf` (binary). Coelanox never sees source or object files.
- **Consumer:** Only performs op_id → blob lookup and copy; no parsing or execution of kernel code.
- **IR/model:** Never sent to the producer; the producer only needs the op_id list from the public registry.
- **Signature:** Optional but recommended so the platform can verify that a `.clf` was not tampered with and (if desired) that it comes from a trusted source.
