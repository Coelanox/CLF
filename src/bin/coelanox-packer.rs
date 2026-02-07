//! coelanox-packer: producer tool to build .clf from (op_id, blob) pairs.
//!
//! Input: list of (op_id, path to raw blob or object file). Optional: --vendor, --sign.
//! Output: one .clf file (header + manifest + blob store + optional signature).
//! Open source so producers can audit it (no exfiltration of code).

use std::fs::File;
use std::io::Read;

use clf::{append_signature, pack_clf, ClfKind, PackOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut vendor = String::new();
    let mut target = String::new();
    let mut blob_align: Option<u8> = None;
    let mut kind = ClfKind::Compute;
    let mut output_path: Option<String> = None;
    let mut sign = false;
    let mut entries: Vec<(u32, String)> = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--vendor" => {
                vendor = args.next().ok_or("--vendor requires a value")?;
            }
            "--target" => {
                target = args.next().ok_or("--target requires a value (e.g. CPU, GPU, CDNA)")?;
            }
            "--kind" => {
                let v = args.next().ok_or("--kind requires compute|memory-movement|memory-protection")?;
                kind = match v.to_lowercase().as_str() {
                    "compute" | "c" => ClfKind::Compute,
                    "memory-movement" | "memorymovement" | "mm" => ClfKind::MemoryMovement,
                    "memory-protection" | "memoryprotection" | "mp" => ClfKind::MemoryProtection,
                    _ => return Err(format!("--kind must be compute|memory-movement|memory-protection, got: {}", v).into()),
                };
            }
            "--align" => {
                let v = args.next().ok_or("--align requires a value (e.g. 16)")?;
                blob_align = Some(v.parse().map_err(|_| "align must be 0–255")?);
            }
            "--output" | "-o" => {
                output_path = Some(args.next().ok_or("--output requires a path")?);
            }
            "--sign" => {
                sign = true;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option: {}", arg).into());
            }
            _ => {
                // Expect "op_id:path" (e.g. 1:blob1.bin 50:blob50.bin)
                let part: Vec<&str> = arg.splitn(2, ':').collect();
                if part.len() != 2 {
                    return Err(format!("expected op_id:path, got: {}", arg).into());
                }
                let op_id: u32 = part[0].parse().map_err(|_| "op_id must be u32")?;
                entries.push((op_id, part[1].to_string()));
            }
        }
    }

    let output_path = output_path.ok_or("--output is required")?;
    if entries.is_empty() {
        return Err("at least one op_id:path entry is required".into());
    }

    // Load blobs from paths (raw binary; no symbol stripping in this minimal tool).
    let blobs: Vec<(u32, Vec<u8>)> = entries
        .into_iter()
        .map(|(op_id, path)| {
            let mut f = File::open(&path)
                .map_err(|e| format!("open {}: {}", path, e))?;
            let mut blob = Vec::new();
            f.read_to_end(&mut blob)
                .map_err(|e| format!("read {}: {}", path, e))?;
            Ok((op_id, blob))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

    let options = PackOptions {
        vendor,
        target,
        blob_alignment: blob_align.unwrap_or(0),
        kind,
        version: clf::CLF_VERSION,
        sign,
    };

    let mut out = File::create(&output_path)?;
    let data_len = pack_clf(&mut out, &blobs, &options)?;
    if options.sign {
        out.sync_all()?;
        append_signature(&mut out, data_len)?;
    }
    out.sync_all()?;

    eprintln!("wrote {} ({} bytes)", output_path, data_len + if options.sign { 4 + 32 } else { 0 });
    Ok(())
}
