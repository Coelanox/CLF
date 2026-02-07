//! CLF reader: open .clf, parse header and manifest, expose get_blob(op_id).
//!
//! Does not interpret blob contents. Optional signature verification before use.
//! When building a code section from a list of op_ids, use `build_code_section` with
//! a `MissingOpIdPolicy`: **Fail** (default) if any op_id is missing, **Skip** to allow partial code.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::format::{
    ClfHeader, ClfKind, ManifestEntry, CLF_MAGIC, CLF_VERSION, SIG_BLOCK_LEN, SIG_MAGIC,
};

/// Policy when an op_id required by the model is not present in the CLF.
/// The packager can choose: fail (strict), skip (partial code), or eventually fall back to another backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingOpIdPolicy {
    /// If any op_id is missing, return an error. Use when the CLF must fully cover the model.
    Fail,
    /// If an op_id is missing, skip it (append nothing). Use for partial code or stubs.
    Skip,
}

/// Errors produced by the CLF reader.
#[derive(Debug, Error)]
pub enum ClfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic: expected CLF1")]
    InvalidMagic,
    #[error("unsupported version: {0} (supported: {1})")]
    UnsupportedVersion(u8, u8),
    #[error("invalid vendor or target: UTF-8 error")]
    InvalidVendorUtf8,
    #[error("signature missing or invalid")]
    SignatureInvalid,
    #[error("missing op_id {0} in CLF (policy: Fail)")]
    MissingOpId(u32),
    #[error("CLF kind mismatch: expected {expected:?}, got {actual:?}")]
    KindMismatch { expected: ClfKind, actual: ClfKind },
}

/// CLF reader: parses header and manifest, provides get_blob(op_id).
#[derive(Debug)]
pub struct ClfReader {
    /// Parsed header (vendor, version).
    pub header: ClfHeader,
    /// Manifest: op_id → (offset, size) relative to blob store start.
    manifest: HashMap<u32, ManifestEntry>,
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
    /// For kind validation, use `open_with_expected_kind`.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ClfError> {
        Self::open_with_expected_kind(path, None)
    }

    /// Open a .clf file and parse header + manifest. When `expected_kind` is `Some(k)`,
    /// rejects the file if the header kind does not match (e.g. opening a .clfmm when
    /// expecting MemoryMovement).
    pub fn open_with_expected_kind<P: AsRef<Path>>(
        path: P,
        expected_kind: Option<ClfKind>,
    ) -> Result<Self, ClfError> {
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

        let mut vendor_len_buf = [0u8; 4];
        reader.read_exact(&mut vendor_len_buf)?;
        let vendor_len = u32::from_le_bytes(vendor_len_buf) as usize;

        let mut vendor_bytes = vec![0u8; vendor_len];
        if vendor_len > 0 {
            reader.read_exact(&mut vendor_bytes)?;
        }
        let vendor = String::from_utf8(vendor_bytes).map_err(|_| ClfError::InvalidVendorUtf8)?;

        // Target length (4 B LE), target (M bytes), blob alignment (1 B).
        let mut target_len_buf = [0u8; 4];
        reader.read_exact(&mut target_len_buf)?;
        let target_len = u32::from_le_bytes(target_len_buf) as usize;
        let mut target_bytes = vec![0u8; target_len];
        if target_len > 0 {
            reader.read_exact(&mut target_bytes)?;
        }
        let target = String::from_utf8(target_bytes).map_err(|_| ClfError::InvalidVendorUtf8)?;
        let mut blob_align_byte = [0u8; 1];
        reader.read_exact(&mut blob_align_byte)?;
        let blob_alignment = blob_align_byte[0];

        // v2: read kind byte; v1: default to Compute (backwards compatibility).
        let kind = if version >= 2 {
            let mut kind_byte = [0u8; 1];
            reader.read_exact(&mut kind_byte)?;
            ClfKind::from_byte(kind_byte[0])
        } else {
            ClfKind::default_for_v1()
        };

        let header_end = reader.stream_position()?;
        let header = ClfHeader {
            version,
            vendor,
            target,
            blob_alignment,
            kind,
            header_end,
        };

        if let Some(expected) = expected_kind {
            if header.kind != expected {
                return Err(ClfError::KindMismatch {
                    expected,
                    actual: header.kind,
                });
            }
        }

        // --- Manifest ---
        let mut num_entries_buf = [0u8; 4];
        reader.read_exact(&mut num_entries_buf)?;
        let num_entries = u32::from_le_bytes(num_entries_buf) as usize;

        let mut manifest = HashMap::with_capacity(num_entries);
        for _ in 0..num_entries {
            let mut entry_buf = [0u8; ManifestEntry::ENTRY_SIZE];
            reader.read_exact(&mut entry_buf)?;
            let op_id = u32::from_le_bytes(entry_buf[0..4].try_into().unwrap());
            let offset = u32::from_le_bytes(entry_buf[4..8].try_into().unwrap());
            let size = u32::from_le_bytes(entry_buf[8..12].try_into().unwrap());
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
    pub fn get_blob(&mut self, op_id: u32) -> Result<Option<Vec<u8>>, ClfError> {
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
    pub fn op_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.manifest.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Build the code section by concatenating blobs for the given op_ids in order.
    /// If an op_id is missing: **Fail** returns `Err(ClfError::MissingOpId(id))`, **Skip** appends nothing for that op.
    pub fn build_code_section(
        &mut self,
        op_ids: &[u32],
        policy: MissingOpIdPolicy,
    ) -> Result<Vec<u8>, ClfError> {
        let mut out = Vec::new();
        for &op_id in op_ids {
            match self.get_blob(op_id)? {
                Some(blob) => out.extend_from_slice(&blob),
                None => {
                    if policy == MissingOpIdPolicy::Fail {
                        return Err(ClfError::MissingOpId(op_id));
                    }
                    // Skip: append nothing.
                }
            }
        }
        Ok(out)
    }
}
