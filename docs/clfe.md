# CLFE (Coelanox Library File — Executor)

CLFE is the **executor** component of the CLF family. It defines the **execution model** for running a graph of ops: plan format and dispatch contract. The runtime uses the plan to walk the graph and dispatch each step to the appropriate kernel (from CLFC or another backend).

**Kind:** `Executor` (3). **Extension:** `.clfe`.

**Role:** Under the CLF umbrella, CLFC = compute (kernels), CLFMM = memory movement, CLFMP = protection, **CLFE = executor** (plan runner / dispatcher). The executor is hardware-agnostic by abstraction: the same plan and dispatch contract can be implemented for CPU (call into code section), GPU (enqueue kernels), or other accelerators.

---

## 1. What CLFE specifies

1. **Plan format** — Binary layout of the execution plan (graph order, buffer offsets, kernel indices). Used by the packager to emit the plan and by the runtime/executor to run it.
2. **Dispatch contract** — Abstract “run step i with these buffers.” Implementations: CPU = call function pointer; GPU = enqueue kernel; etc. The runtime chooses the implementation by backend/target; the plan itself is target-agnostic.

The **kind** (Executor) is what the loader/runtime uses to route the file. The **extension** (.clfe) is for humans and tooling (sorting, discovery). When opening a `.clfe` file, the consumer should validate that the header kind is Executor.

---

## 2. Execution plan format (binary, little-endian)

The plan is a contiguous byte buffer (e.g. stored in the container or in a CLFE blob). Layout:

| Field               | Size    | Description |
|---------------------|---------|-------------|
| workspace_elements  | 4 B     | Workspace size in f32 elements (input + intermediates + output). |
| num_nodes           | 4 B     | Number of steps (graph nodes in execution order). |
| **Per node** (7 × 4 B each) | 28 B × num_nodes | For each step: |
| ↳ op_id             | 4 B     | Canonical op_id (matches CLFC / op_registry). |
| ↳ in_off            | 4 B     | Input buffer offset (elements) into workspace. |
| ↳ out_off           | 4 B     | Output buffer offset (elements) into workspace. |
| ↳ w_off             | 4 B     | Weights offset (elements) into weights buffer. |
| ↳ in_len            | 4 B     | Input length (elements). |
| ↳ out_len           | 4 B     | Output length (elements). |
| ↳ w_len             | 4 B     | Weights length (elements). |
| num_kernels         | 4 B     | Number of kernel offsets (must equal num_nodes when code section is per-op blobs). |
| kernel_offset[i]    | 4 B each| Byte offset of kernel i from code base (same order as nodes). |

- **Workspace:** Single contiguous buffer (e.g. `[input | intermediates | output]`). The host or runtime allocates `workspace_elements` f32s; the executor copies input into the start and reads output from the final step’s output region.
- **Weights:** Separate buffer (decompressed); offsets and lengths are in elements (f32).
- **Code section:** For CPU, the code section is a concatenation of kernel blobs; `kernel_offset[i]` is the byte offset of the blob for step i. For GPU/accelerator, the implementation may map kernel index to a device kernel handle instead.

---

## 3. Dispatch contract (abstract)

The executor (or runtime) does:

1. Parse the plan (workspace size, node list, kernel offsets).
2. For each step `i` in order: resolve input/output/weight pointers (or handles) from the plan; then **dispatch**: “run kernel i with these buffers.”

**Dispatch** is backend-specific:

- **CPU:** Code section = concatenated blobs. Dispatch = call function at `code_base + kernel_offset[i]` with `(input_ptr, output_ptr, weights_ptr, in_len, out_len, w_len)` (e.g. x86-64 SysV ABI).
- **GPU / NPU:** Dispatch = enqueue the kernel for step i with the given buffer descriptors (device pointers or buffer IDs). Same plan; different implementation.

So the **plan is hardware-agnostic**; only the **dispatcher** is backend-specific. This mirrors CLF: op_id + ABI is the contract; each target provides the blobs and the way to run them.

---

## 4. CLFE as a .clf file (kind = Executor)

A `.clfe` file is a standard CLF file (same header, manifest, blob store, optional signature) with **kind = Executor**. The blob store may contain:

- A single **executor blob** (e.g. mini-executor machine code to be placed at the start of the code section), and/or
- **Plan blob(s)** (op_id could designate “plan” if the producer uses the manifest to key plans).

Concrete use: producer builds an executor binary per target (e.g. CPU x86_64), packs it with `--kind executor` into `executor.clfe`. The packager or runtime can then discover `.clfe` files and prepend the executor blob to the code section so the host only “load and call once.” The plan may live in the container (as today) or be stored in the same .clfe or a separate blob; the spec does not mandate where the plan bytes come from, only their format once present.

---

## 5. Summary

| Item            | Content |
|-----------------|---------|
| **Kind**        | Executor (3). |
| **Extension**   | `.clfe`. |
| **Plan format** | workspace_elements, num_nodes, per-node (op_id, in_off, out_off, w_off, in_len, out_len, w_len), num_kernels, kernel_offset[]. |
| **Dispatch**    | Abstract “run step i with these buffers”; implementation per backend (CPU call, GPU enqueue, etc.). |
| **Hardware**    | Plan and contract are agnostic; dispatcher is backend-specific. |
