//! CLF (Coelanox Library File) — reader and packer for pre-compiled hardware kernel archives.
//!
//! This crate provides:
//! - **Format types** (`format`): header, manifest entry, constants (CLF_MAGIC, etc.).
//! - **Op ID registry** (`op_registry`): canonical op_id list and `op_type_to_clf_id` / `clf_id_to_op_type`.
//! - **Reader** (`reader`): `ClfReader::open(path)` and `get_blob(op_id)` for the packager.
//! - **Packer** (binary `coelanox-packer`): build .clf from (op_id, blob) pairs and optional vendor/version.
//!
//! See [SPEC.md](SPEC.md) and [docs/op_ids.md](docs/op_ids.md) for the full specification and op_id registry.

pub mod format;
pub mod op_registry;
pub mod packer;
pub mod reader;

pub use format::{ClfHeader, ClfKind, ManifestEntry, CLF_MAGIC, CLF_VERSION, SIG_BLOCK_LEN, SIG_MAGIC};
pub use op_registry::{clf_id_to_op_type, op_type_to_clf_id, OpType};
pub use packer::{append_signature, pack_clf, PackError, PackOptions};
pub use reader::{ClfError, ClfReader, MissingOpIdPolicy};
