# CLF (Coelanox Library File)

CLF is the standard binary format for shipping pre-compiled hardware kernels in the Coelanox ecosystem. It is a static kernel archive: one file containing optimized machine-code blobs keyed by numeric op_id. The packager looks up blobs by op_id and copies them into the container at package time; there is no runtime code generation or kernel loading.

**Three uses for the three HALs:**

- **Codegen / backend:** CLF supplies the machine code that would otherwise come from a BackendTranslator. When the backend is `BackendKind::Clf`, the packager opens the `.clf`, maps each IR node’s OpType to op_id, and appends the corresponding blob to the code section.
- **Memory HAL:** The code section (built from CLF blobs) is what the runtime allocates or backs via the Memory HAL (e.g. executable region). CLF-derived bytes are the content of those regions.
- **Protection HAL:** The region(s) holding the code section are the ones the Protection HAL configures (e.g. code RX, weights RO). CLF-derived content defines which regions get which protection.

**Spec and registry:**

- [SPEC.md](SPEC.md) — full format specification (binary layout, workflow, signature).
- [docs/op_ids.md](docs/op_ids.md) — canonical op_id registry (op_id ↔ op name / OpType).

**This repo provides:**

- **Reader** (Rust): `ClfReader::open(path)`, `get_blob(op_id)`, optional `verify_signature()`. For use by the Coelanox Packager.
- **Packer** (Rust): `coelanox-packer` CLI and `pack_clf` / `append_signature` library API. For kernel providers building `.clf` files from (op_id, blob) pairs.
- **Op ID registry**: `op_type_to_clf_id(OpType)`, `clf_id_to_op_type(op_id)` and canonical op_id list in code and in docs/op_ids.md.

Build: `cargo build`. Tests: `cargo test`. Pack a .clf: `cargo run --bin coelanox-packer -- --output out.clf 1:blob1.bin 50:blob50.bin`.
