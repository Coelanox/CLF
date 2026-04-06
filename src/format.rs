//! CLF binary format types and constants.
//!
//! Defines header layout, manifest entries, and magic/signature constants
//! for the Coelanox Library File (.clf) format. All multi-byte fields are little-endian.

use std::fmt;
use std::str::FromStr;

/// Magic bytes at the start of every CLF file: "CLF1".
pub const CLF_MAGIC: [u8; 4] = [0x43, 0x4C, 0x46, 0x31];

/// Current format version written by the packer; readers reject version > CLF_VERSION.
/// Version 1 = header without kind (legacy); kind defaults to Compute.
/// Version 2 = header with kind field (Compute / MemoryMovement / MemoryProtection).
pub const CLF_VERSION: u8 = 2;

/// CLF file kind: role of the file in the Coelanox ecosystem.
/// Used for discovery and routing via extensions (.clfc, .clfmm, .clfmp, .clfe).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClfKind {
    /// Compute kernels: op_id → machine code blobs for execution.
    Compute = 0,
    /// Memory movement: blobs for memory copy/move operations.
    MemoryMovement = 1,
    /// Memory protection: blobs for region protection setup.
    MemoryProtection = 2,
    /// Executor: plan runner / dispatcher (runs graph; dispatches to compute blobs).
    Executor = 3,
}

impl ClfKind {
    /// Default kind for v1 files (backwards compatibility).
    pub const fn default_for_v1() -> Self {
        Self::Compute
    }

    /// Parse from byte; invalid values default to Compute for robustness.
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::MemoryMovement,
            2 => Self::MemoryProtection,
            3 => Self::Executor,
            _ => Self::Compute,
        }
    }

    /// Extension for this kind (for discovery/routing).
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Compute => "clfc",
            Self::MemoryMovement => "clfmm",
            Self::MemoryProtection => "clfmp",
            Self::Executor => "clfe",
        }
    }
}

impl fmt::Display for ClfKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ClfKind::Compute => "compute",
            ClfKind::MemoryMovement => "memory-movement",
            ClfKind::MemoryProtection => "memory-protection",
            ClfKind::Executor => "executor",
        };
        f.write_str(s)
    }
}

impl FromStr for ClfKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "compute" | "c" | "clfc" => Ok(Self::Compute),
            "memory-movement" | "memorymovement" | "mm" | "clfmm" => Ok(Self::MemoryMovement),
            "memory-protection" | "memoryprotection" | "mp" | "clfmp" => Ok(Self::MemoryProtection),
            "executor" | "e" | "clfe" => Ok(Self::Executor),
            _ => Err(format!(
                "unknown kind {s:?} (expected compute, memory-movement, memory-protection, executor)"
            )),
        }
    }
}

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

/// Parsed CLF header (after reading magic, version, vendor, target, alignment, kind).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// File kind (Compute / MemoryMovement / MemoryProtection). v1: defaults to Compute; v2: read from header.
    pub kind: ClfKind,
    /// Byte offset in file where header ends (start of manifest).
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub header_end: u64,
}

/// Single manifest entry: op_id → (offset, size) into blob store.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct ManifestEntry {
    /// Canonical op identifier (see op_registry / docs/op_ids.md).
    pub op_id: u32,
    /// Offset from start of blob store, in bytes.
    pub offset: u32,
    /// Stored length in the blob store (includes tail padding when alignment is used).
    pub size: u32,
}

impl ManifestEntry {
    /// Size of one manifest entry in the file: op_id (4) + offset (4) + size (4).
    pub const ENTRY_SIZE: usize = 4 + 4 + 4;
}
