//! CLF reader: open .clf, parse header and manifest, expose get_blob(op_id).
//!
//! Does not interpret blob contents. Optional signature verification before use.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::format::{
    ClfHeader, ManifestEntry, CLF_MAGIC, CLF_VERSION, SIG_BLOCK_LEN, SIG_MAGIC,
};

/// Errors produced by the CLF reader.
#[derive(Debug, Error)]
pub enum ClfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic: expected CLF1")]
    InvalidMagic,
    #[error("unsupported version: {0} (supported: {1})")]
    UnsupportedVersion(u8, u8),
    #[error("invalid vendor: UTF-8 error")]
    InvalidVendorUtf8,
    #[error("signature missing or invalid")]
    SignatureInvalid,
}

/// CLF reader: parses header and manifest, provides get_blob(op_id).
pub struct ClfReader {
    /// Parsed header (vendor, version).
    pub header: ClfHeader,
    /// Manifest: op_id → (offset, size) relative to blob store start.
    manifest: HashMap<u16, ManifestEntry>,
    /// File handle; blob store starts at blob_store_offset.
    reader: BufReader<File>,
    /// Byte offset in file where blob store starts.
    blob_store_offset: u64,
    /// Total length of blob store (so we can bounds-check reads).
    blob_store_len: u64,
    /// If true, file has a valid signature block at end (verified by verify_signature).
    signature_verified: bool,
}

impl ClfReader {
    /// Open a .clf file and parse header + manifest. Does not verify signature.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ClfError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // --- Header ---
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != CLF_MAGIC {
            return Err(ClfError::InvalidMagic);
        }

        let mut version_byte = [0u8; 1];
        reader.read_exact(&mut version_byte)?;
        let version = version_byte[0];
        if version > CLF_VERSION {
            return Err(ClfError::UnsupportedVersion(version, CLF_VERSION));
        }

        let mut vendor_len_buf = [0u8; 2];
        reader.read_exact(&mut vendor_len_buf)?;
        let vendor_len = u16::from_le_bytes(vendor_len_buf) as usize;

        let mut vendor_bytes = vec![0u8; vendor_len];
        if vendor_len > 0 {
            reader.read_exact(&mut vendor_bytes)?;
        }
        let vendor = String::from_utf8(vendor_bytes).map_err(|_| ClfError::InvalidVendorUtf8)?;

        let header_end = reader.stream_position()?;
        let header = ClfHeader {
            version,
            vendor,
            header_end,
        };

        // --- Manifest ---
        let mut num_entries_buf = [0u8; 2];
        reader.read_exact(&mut num_entries_buf)?;
        let num_entries = u16::from_le_bytes(num_entries_buf) as usize;

        let mut manifest = HashMap::with_capacity(num_entries);
        for _ in 0..num_entries {
            let mut entry_buf = [0u8; ManifestEntry::ENTRY_SIZE];
            reader.read_exact(&mut entry_buf)?;
            let op_id = u16::from_le_bytes(entry_buf[0..2].try_into().unwrap());
            let offset = u32::from_le_bytes(entry_buf[2..6].try_into().unwrap());
            let size = u32::from_le_bytes(entry_buf[6..10].try_into().unwrap());
            let entry = ManifestEntry { op_id, offset, size };
            manifest.insert(op_id, entry);
        }

        let blob_store_offset = reader.stream_position()?;

        // Compute blob store length: either from file size minus optional signature, or from max(offset+size).
        let file_len = reader.seek(SeekFrom::End(0))?;
        let has_sig = file_len >= (SIG_BLOCK_LEN as u64)
            && {
                reader.seek(SeekFrom::End(-(SIG_BLOCK_LEN as i64)))?;
                let mut sig_magic = [0u8; 4];
                reader.read_exact(&mut sig_magic).is_ok() && sig_magic == SIG_MAGIC
            };
        let blob_store_len = if has_sig {
            file_len.saturating_sub(SIG_BLOCK_LEN as u64).saturating_sub(blob_store_offset)
        } else {
            file_len.saturating_sub(blob_store_offset)
        };

        // Re-seek to start of blob store for future get_blob reads.
        reader.seek(SeekFrom::Start(blob_store_offset))?;

        Ok(ClfReader {
            header,
            manifest,
            reader,
            blob_store_offset,
            blob_store_len,
            signature_verified: false,
        })
    }

    /// Return the blob for the given op_id if present. No interpretation of blob contents.
    pub fn get_blob(&mut self, op_id: u16) -> Result<Option<Vec<u8>>, ClfError> {
        let entry = match self.manifest.get(&op_id) {
            Some(e) => e,
            None => return Ok(None),
        };

        let start = self.blob_store_offset + u64::from(entry.offset);
        let end = start + u64::from(entry.size);
        if end > self.blob_store_offset + self.blob_store_len {
            return Err(ClfError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "manifest entry extends past blob store",
            )));
        }

        self.reader.seek(SeekFrom::Start(start))?;
        let mut blob = vec![0u8; entry.size as usize];
        self.reader.read_exact(&mut blob)?;
        Ok(Some(blob))
    }

    /// Verify the optional signature at end of file (SIG0 + SHA-256 of everything before it).
    /// Call after open() if the consumer requires a valid signature before use.
    pub fn verify_signature(&mut self) -> Result<bool, ClfError> {
        let file_len = self.reader.seek(SeekFrom::End(0))?;
        if file_len < SIG_BLOCK_LEN as u64 {
            return Ok(false);
        }

        self.reader.seek(SeekFrom::End(-(SIG_BLOCK_LEN as i64)))?;
        let mut sig_block = [0u8; SIG_BLOCK_LEN];
        self.reader.read_exact(&mut sig_block)?;
        let (sig_magic, stored_hash) = sig_block.split_at(4);
        if sig_magic != SIG_MAGIC {
            return Ok(false);
        }

        let data_len = file_len - SIG_BLOCK_LEN as u64;
        self.reader.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha256::new();
        let mut to_read = data_len as usize;
        let mut buf = [0u8; 4096];
        while to_read > 0 {
            let n = to_read.min(buf.len());
            self.reader.read_exact(&mut buf[..n])?;
            hasher.update(&buf[..n]);
            to_read -= n;
        }
        let computed = hasher.finalize();
        if computed.as_slice() != stored_hash {
            return Err(ClfError::SignatureInvalid);
        }

        self.signature_verified = true;
        // Re-seek to blob store for subsequent get_blob.
        self.reader.seek(SeekFrom::Start(self.blob_store_offset))?;
        Ok(true)
    }

    /// Whether verify_signature() was called and succeeded.
    #[must_use]
    pub fn signature_verified(&self) -> bool {
        self.signature_verified
    }

    /// List all op_ids present in the manifest.
    #[must_use]
    pub fn op_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self.manifest.keys().copied().collect();
        ids.sort_unstable();
        ids
    }
}
