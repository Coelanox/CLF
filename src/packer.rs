//! CLF packer: build .clf from (op_id, blob) pairs and optional vendor/version.
//!
//! Used by the coelanox-packer binary. Writes header + manifest + blob store + optional signature.

use std::io::{Read, Seek, Write};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::format::{CLF_MAGIC, CLF_VERSION, SIG_MAGIC};

/// Errors produced by the packer.
#[derive(Debug, Error)]
pub enum PackError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("duplicate op_id: {0}")]
    DuplicateOpId(u16),
    #[error("vendor string too long (max 65535 bytes)")]
    VendorTooLong,
}

/// Options for building a .clf file.
#[derive(Debug, Clone)]
pub struct PackOptions {
    /// Vendor identifier (UTF-8); display/audit only.
    pub vendor: String,
    /// Format version to write (default CLF_VERSION).
    pub version: u8,
    /// If true, append SIG0 + SHA-256 of everything before the signature.
    pub sign: bool,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            vendor: String::new(),
            version: CLF_VERSION,
            sign: false,
        }
    }
}

/// Build a .clf file from (op_id, blob) pairs. Entries must have unique op_ids.
/// Writes to `out`: header + manifest + blob store. Returns the number of bytes written
/// (caller may then call `append_signature` if options.sign is true).
pub fn pack_clf<W: Write + Seek>(
    out: &mut W,
    entries: &[(u16, Vec<u8>)],
    options: &PackOptions,
) -> Result<u64, PackError> {
    let vendor_bytes = options.vendor.as_bytes();
    if vendor_bytes.len() > u16::MAX as usize {
        return Err(PackError::VendorTooLong);
    }

    // Check for duplicate op_ids.
    let mut seen = std::collections::HashSet::new();
    for (op_id, _) in entries {
        if !seen.insert(*op_id) {
            return Err(PackError::DuplicateOpId(*op_id));
        }
    }

    // --- Header ---
    out.write_all(&CLF_MAGIC)?;
    out.write_all(&[options.version])?;
    let vendor_len = vendor_bytes.len() as u16;
    out.write_all(&vendor_len.to_le_bytes())?;
    out.write_all(vendor_bytes)?;

    // --- Manifest: num_entries (2) + entries (10 each) ---
    let num_entries = entries.len() as u16;
    out.write_all(&num_entries.to_le_bytes())?;

    let mut offset: u32 = 0;
    for (op_id, blob) in entries {
        let size = blob.len() as u32;
        out.write_all(&op_id.to_le_bytes())?;
        out.write_all(&offset.to_le_bytes())?;
        out.write_all(&size.to_le_bytes())?;
        offset = offset.saturating_add(size);
    }

    // --- Blob store ---
    for (_, blob) in entries {
        out.write_all(blob)?;
    }

    let data_len = out.stream_position()?;
    Ok(data_len)
}

/// Append signature block (SIG0 + SHA-256) to the end of an open file. Call after pack_clf when options.sign is true.
/// `data_len` must be the number of bytes written so far (header + manifest + blob store).
/// The file must support Read, Write, and Seek.
pub fn append_signature<W: Read + Write + Seek>(
    out: &mut W,
    data_len: u64,
) -> Result<(), PackError> {
    out.seek(std::io::SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut to_read = data_len as usize;
    let mut buf = [0u8; 4096];
    while to_read > 0 {
        let n = to_read.min(buf.len());
        let got = out.read(&mut buf[..n])?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        to_read -= got;
    }
    let hash = hasher.finalize();
    out.seek(std::io::SeekFrom::End(0))?;
    out.write_all(&SIG_MAGIC)?;
    out.write_all(&hash)?;
    Ok(())
}
