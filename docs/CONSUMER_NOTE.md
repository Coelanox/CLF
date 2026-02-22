# How the Coelanox Packager uses CLF (consumer note)

This note describes how the packager discovers CLFs, matches them to a target, builds the code section, and how that relates to the CLF family and HALs.

## CLF family (four kinds)

CLF files share one binary format; the **Kind** byte in the v2 header (and the file extension) select the role:

- **CLFC** (Compute, `.clfc`): op_id → kernel blobs. The **packager** uses these to build the container’s code section (see below). Primary consumer: packager.
- **CLFMM** (Memory Movement, `.clfmm`): blobs for allocate / deallocate / move. May be embedded in the container; **runtime** uses them for Memory HAL when present.
- **CLFMP** (Memory Protection, `.clfmp`): blobs for configure_region / enable / disable. May be embedded in the container; **runtime** uses them for Protection HAL when present.
- **CLFE** (Executor, `.clfe`): executor blob (plan runner) and/or plan data. The **runtime** (or host) can load an executor blob to run the execution plan (parse plan, dispatch each step to the code section or device). Same vendor that provides CLFC for a target typically provides the matching CLFE. See [clfe.md](clfe.md) for plan format and dispatch contract.

Discovery and code-section building below apply to **CLFC** (and legacy `.clf`). CLFE / CLFMM / CLFMP are discovered by extension or kind for runtime or packager embedding.

## Discovery and target matching

- **Backend loader** scans backend search paths for `.clf` and `.clfc` (and optionally `.clfmm`, `.clfmp`, `.clfe`, `.so`/`.dll`). For each CLF it may check the header **kind** (e.g. only register as code backend when kind is Compute). It registers a backend with `kind: BackendKind::Clf`, `library_path: path to .clf/.clfc`, and `supported_targets` (e.g. from the CLF **target** field in the header, or from a convention like `cpu.clfc` / `gpu.clfc`).
- When generating machine code for a **target**, the packager calls `find_backend_for_target(target)`. If the returned backend is CLF, it opens the file at `backend_info.library_path` and uses the CLF reader. Optionally it checks that `reader.header.target` matches the requested target and that **kind** is Compute (or legacy). See [clfe.md](clfe.md) for how the executor (CLFE) uses the plan and code section.

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
