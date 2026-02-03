//! CLF binary format types and constants.
//!
//! Defines header layout, manifest entries, and magic/signature constants
//! for the Coelanox Library File (.clf) format. All multi-byte fields are little-endian.

/// Magic bytes at the start of every CLF file: "CLF1".
pub const CLF_MAGIC: [u8; 4] = [0x43, 0x4C, 0x46, 0x31];

/// Current format version written by the packer; readers reject version > CLF_VERSION.
/// Version 1 = this layout (including optional target and blob_alignment).
pub const CLF_VERSION: u8 = 1;

/// Signature magic at end of file when signature is present: "SIG0".
pub const SIG_MAGIC: [u8; 4] = [0x53, 0x49, 0x47, 0x30];

/// Length of SHA-256 signature in bytes (after SIG_MAGIC).
pub const SIG_HASH_LEN: usize = 32;

/// Total signature block size: magic + hash.
pub const SIG_BLOCK_LEN: usize = 4 + SIG_HASH_LEN;

/// No blob alignment (blobs stored back-to-back).
pub const BLOB_ALIGN_NONE: u8 = 0;

/// Common alignment for machine code (e.g. 16-byte for many ISAs).
pub const BLOB_ALIGN_CODE: u8 = 16;

/// Parsed CLF header (after reading magic, version, vendor, target, alignment).
#[derive(Debug, Clone)]
pub struct ClfHeader {
    /// Format version (must be <= supported version).
    pub version: u8,
    /// Vendor identifier (UTF-8); display/audit only.
    pub vendor: String,
    /// Target/architecture (e.g. "CPU", "GPU", "CDNA"); empty if not set. Used by packager to match CLF to target.
    pub target: String,
    /// Blob alignment in bytes (0 = no alignment). Producer pads blobs to this alignment in the blob store.
    pub blob_alignment: u8,
    /// Byte offset in file where header ends (start of manifest).
    pub header_end: u64,
}

/// Single manifest entry: op_id → (offset, size) into blob store.
#[derive(Debug, Clone, Copy)]
pub struct ManifestEntry {
    /// Canonical op identifier (see op_registry / docs/op_ids.md).
    pub op_id: u32,
    /// Offset from start of blob store, in bytes.
    pub offset: u32,
    /// Blob length in bytes.
    pub size: u32,
}

impl ManifestEntry {
    /// Size of one manifest entry in the file: op_id (4) + offset (4) + size (4).
    pub const ENTRY_SIZE: usize = 4 + 4 + 4;
}
