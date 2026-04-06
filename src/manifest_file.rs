//! TOML manifest for `coelanox-packer --from` (batch pack with optional per-blob metadata for sidecars).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::format::ClfKind;

#[derive(Debug, Deserialize)]
struct TomlRoot {
    vendor: Option<String>,
    target: Option<String>,
    kind: Option<String>,
    align: Option<u8>,
    sign: Option<bool>,
    blobs: Vec<TomlBlob>,
}

#[derive(Debug, Deserialize)]
struct TomlBlob {
    op_id: u32,
    path: String,
    symbol: Option<String>,
    notes: Option<String>,
}

/// One blob line from a pack manifest (path on disk + optional audit fields).
#[derive(Debug, Clone)]
pub struct PackManifestBlob {
    pub op_id: u32,
    pub path: PathBuf,
    pub symbol: Option<String>,
    pub notes: Option<String>,
}

/// Fully resolved manifest: same defaults as CLI (`PackOptions`).
#[derive(Debug, Clone)]
pub struct PackManifestResolved {
    pub vendor: String,
    pub target: String,
    pub kind: ClfKind,
    pub align: u8,
    pub sign: bool,
    pub blobs: Vec<PackManifestBlob>,
}

/// Load and validate `pack.toml`-style manifest from disk.
pub fn load_pack_manifest(path: &Path) -> Result<PackManifestResolved, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let root: TomlRoot = toml::from_str(&text).map_err(|e| format!("TOML parse: {e}"))?;
    if root.blobs.is_empty() {
        return Err("manifest must contain at least one [[blobs]] entry".into());
    }

    let mut seen = std::collections::HashSet::new();
    let mut blobs = Vec::with_capacity(root.blobs.len());
    for b in root.blobs {
        if !seen.insert(b.op_id) {
            return Err(format!("duplicate op_id {} in manifest", b.op_id));
        }
        let path = PathBuf::from(&b.path);
        blobs.push(PackManifestBlob {
            op_id: b.op_id,
            path,
            symbol: b.symbol,
            notes: b.notes,
        });
    }

    let kind = match root.kind {
        Some(ref s) => s.parse::<ClfKind>().map_err(|e| e.to_string())?,
        None => ClfKind::Compute,
    };

    Ok(PackManifestResolved {
        vendor: root.vendor.unwrap_or_default(),
        target: root.target.unwrap_or_default(),
        kind,
        align: root.align.unwrap_or(0),
        sign: root.sign.unwrap_or(false),
        blobs,
    })
}
