# CLFE kernel call ABI (6-arg contract)

The executor calls **every** kernel blob with a single calling convention: the **6-arg ABI**. There is no per-op dispatch; all CLFC blobs invoked by the executor must implement this contract.

## Signature

Each kernel entry point is called with:

- **input_ptr** — pointer to input buffer (read-only, `const float*`)
- **output_ptr** — pointer to output buffer (write-only, `float*`)
- **weights_ptr** — pointer to weights buffer (read-only, `const float*`; may be unused by some ops)
- **in_len** — input length in elements (f32 count)
- **out_len** — output length in elements (f32 count)
- **w_len** — weights length in elements (f32 count; 0 if no weights)

All lengths are in **elements** (f32), not bytes.

## x86-64 SysV

| Argument   | Register |
|-----------|----------|
| input_ptr | rdi      |
| output_ptr| rsi      |
| weights_ptr | rdx   |
| in_len    | rcx      |
| out_len   | r8       |
| w_len     | r9       |

## Contract

- No other calling convention is used for execution. Blobs that were built with a different (per-op) ABI must not be packed as executor-invoked kernels; use 6-arg wrappers that delegate to the inner implementation instead.
- See [clfe.md](clfe.md) for the execution plan format and dispatch contract.

## Executor (CLFE) package

For `.clfe` files (kind = Executor), the blob store may contain an executor blob and/or plan data. **op_id 0** is reserved for a descriptor or stub blob when building a CLFE package (e.g. so the packer has at least one entry).
