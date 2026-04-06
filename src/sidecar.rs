//! Optional JSON sidecar next to a `.clf` with per-blob SHA-256 and audit fields (does not change the CLF bytes; Coelanox ignores this file).

use std::fs;
use std::path::Path;

use serde::Serialize;

/// One row per packed blob (hashes are hex-encoded SHA-256 of raw blob bytes).
#[derive(Debug, Serialize)]
pub struct SidecarBlob {
    pub op_id: u32,
    pub path: String,
    pub sha256_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Top-level sidecar document (`*.clf.meta.json`).
#[derive(Debug, Serialize)]
pub struct SidecarDocument {
    pub schema: &'static str,
    pub clf_crate_version: &'static str,
    pub output: String,
    pub blobs: Vec<SidecarBlob>,
}

impl SidecarDocument {
    pub fn new(output_display: String, blobs: Vec<SidecarBlob>) -> Self {
        Self {
            schema: "clf.sidecar.v1",
            clf_crate_version: env!("CARGO_PKG_VERSION"),
            output: output_display,
            blobs,
        }
    }
}

/// Write pretty-printed JSON UTF-8.
pub fn write_sidecar_json(path: &Path, doc: &SidecarDocument) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}
