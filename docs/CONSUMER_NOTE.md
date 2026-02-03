# How the Coelanox Packager uses CLF (consumer note)

This note describes how the packager discovers CLFs, matches them to a target, builds the code section, and how that relates to the three HALs.

## Discovery and target matching

- **Backend loader** scans backend search paths for `.clf` (and optionally `.so`/`.dll`). For each `.clf` it registers a backend with `kind: BackendKind::Clf`, `library_path: path to .clf`, and `supported_targets` (e.g. from the CLF **target** field in the header, or from a convention like `cpu.clf` / `gpu.clf`).
- When generating machine code for a **target**, the packager calls `find_backend_for_target(target)`. If the returned backend is CLF, it opens the `.clf` at `backend_info.library_path` and uses the CLF reader. Optionally it checks that `reader.header.target` matches the requested target (or relies on the loader having set `supported_targets` from the header or filename).

## Building the code section

1. Open the CLF: `ClfReader::open(path)`.
2. **(Optional)** Verify before use: `reader.verify_signature()`. If the platform requires a valid signature, refuse to use the file when verification fails.
3. Walk the optimized IR’s nodes in **execution order**. For each node:
   - `op_id = op_type_to_clf_id(node.op_type)` (canonical registry).
   - `blob = reader.get_blob(op_id)`.
   - **Missing op_id policy:** If `blob` is `None`, either **Fail** (error and abort) or **Skip** (append nothing; partial code). The library provides `build_code_section(reader, op_ids, policy)` with `MissingOpIdPolicy::Fail` or `MissingOpIdPolicy::Skip`; the packager chooses the policy (e.g. Fail by default, Skip for partial/stub builds).
4. Append each blob to the code buffer (or use `build_code_section` with the chosen policy).
5. The resulting buffer is the container’s **code section** (possibly combined with other backends or stubs). Write it into the `.cnox` container.

## Three uses for the three HALs

- **Codegen / backend:** CLF **supplies the machine code** that would otherwise come from a BackendTranslator. When the backend is `BackendKind::Clf`, the packager uses the CLF reader and op_id registry as above.
- **Memory HAL:** The code section (built from CLF blobs) is what the runtime allocates or backs via the Memory HAL (e.g. executable region). CLF-derived bytes are the **content** of those regions.
- **Protection HAL:** The region(s) that hold the code section (and optionally weights/scratch) are the ones the Protection HAL configures (e.g. code → RX, weights → RO). CLF-derived content defines **which regions** get which protection.

## Order of blobs

Blobs are appended in **execution order** of the optimized IR. The packager does not reorder; it follows the IR. So the code section is the concatenation of blobs in that order (with optional alignment padding as stored in the CLF).

## No runtime CLF loading

The runtime does **not** read `.clf` files. It loads the `.cnox` container; the code section is already the concatenated kernel blobs. Memory HAL and Protection HAL operate on that in-memory region.
