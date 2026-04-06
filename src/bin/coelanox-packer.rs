// CLF packer CLI: build .clf archives, inspect them, or verify SIG0 + SHA-256.
// Installed as `clf` or `coelanox-packer` (same behavior; see src/bin/clf.rs).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use sha2::{Digest, Sha256};

use clf::{
    append_signature, load_pack_manifest, pack_clf, parse_op_blob_arg, sidecar, ClfReader,
    PackManifestBlob, PackManifestResolved, PackOptions, CLF_VERSION,
};

#[derive(Parser)]
#[command(
    name = env!("CARGO_BIN_NAME"),
    version,
    about = "Build, inspect, or verify Coelanox Library Files (.clf, .clfc, …)",
    long_about = "Pack: write a CLF from op_id:path pairs or a TOML manifest (--from).\n\
                  Inspect: print header and manifest (-i), optional JSON (--json).\n\
                  Verify: check SIG0 + SHA-256 only (--verify).\n\
                  \n\
                  Examples:\n\
                    clf -o out.clfc --align 16 1:a.bin 50:b.bin\n\
                    clf --from pack.toml -o out.clfc --dry-run\n\
                    clf -i out.clfc --json\n\
                    clf --verify out.clfc\n"
)]
struct Cli {
    /// Print header and manifest (human-readable); use --json for machine output
    #[arg(long, short = 'i', value_name = "FILE", conflicts_with_all = ["verify", "output", "from_manifest"])]
    inspect: Option<PathBuf>,

    /// Verify SIG0 + SHA-256 and exit 0 (ok) or 1 (missing/invalid); for CI
    #[arg(long, value_name = "FILE", conflicts_with_all = ["inspect", "output", "from_manifest", "entries"])]
    verify: Option<PathBuf>,

    /// With --inspect: verify hash before printing
    #[arg(long, requires = "inspect")]
    verify_signature: bool,

    /// With --inspect: print JSON to stdout (stable for scripts)
    #[arg(long, requires = "inspect")]
    json: bool,

    /// Output path (required when packing)
    #[arg(short, long, value_name = "PATH", conflicts_with_all = ["inspect", "verify"])]
    output: Option<PathBuf>,

    /// Pack using a TOML manifest (see PRODUCER_GUIDE.md); merges with CLI flags where set
    #[arg(long, value_name = "PATH", conflicts_with = "entries")]
    from_manifest: Option<PathBuf>,

    /// Validate inputs and print summary; do not write a .clf
    #[arg(long)]
    dry_run: bool,

    /// Write `<output>.meta.json` sidecar with per-blob SHA-256 and optional symbol/notes
    #[arg(long)]
    write_sidecar: bool,

    #[arg(long)]
    vendor: Option<String>,

    #[arg(long)]
    target: Option<String>,

    #[arg(long, value_parser = clap::value_parser!(clf::ClfKind))]
    kind: Option<clf::ClfKind>,

    #[arg(long, value_name = "N")]
    align: Option<u8>,

    #[arg(long)]
    sign: bool,

    #[arg(value_name = "OP_ID:PATH")]
    entries: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(path) = &cli.verify {
        return verify_file(path);
    }

    if let Some(path) = &cli.inspect {
        return inspect_file(path, cli.verify_signature, cli.json);
    }

    // Pack
    let output_path = cli
        .output
        .clone()
        .ok_or("packing requires --output / -o (or use --inspect / --verify)")?;

    let (resolved, from_manifest) = if let Some(manifest_path) = &cli.from_manifest {
        let m = load_pack_manifest(manifest_path)?;
        (m, true)
    } else if !cli.entries.is_empty() {
        (
            PackManifestResolved {
                vendor: String::new(),
                target: String::new(),
                kind: clf::ClfKind::Compute,
                align: 0,
                sign: false,
                blobs: cli
                    .entries
                    .iter()
                    .map(|arg| {
                        let (op_id, p) = parse_op_blob_arg(arg)?;
                        Ok(PackManifestBlob {
                            op_id,
                            path: PathBuf::from(p),
                            symbol: None,
                            notes: None,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            },
            false,
        )
    } else {
        return Err("packing requires at least one OP_ID:PATH or --from MANIFEST.toml".into());
    };

    let vendor = cli.vendor.unwrap_or_else(|| resolved.vendor.clone());
    let target = cli.target.unwrap_or_else(|| resolved.target.clone());
    let kind = cli.kind.unwrap_or(resolved.kind);
    let blob_alignment = cli.align.unwrap_or(resolved.align);
    let sign = if cli.sign { true } else { resolved.sign };

    let blobs: Vec<(u32, Vec<u8>)> = resolved
        .blobs
        .iter()
        .map(|b| {
            let mut f =
                File::open(&b.path).map_err(|e| format!("open {}: {e}", b.path.display()))?;
            let mut blob = Vec::new();
            f.read_to_end(&mut blob)
                .map_err(|e| format!("read {}: {e}", b.path.display()))?;
            Ok((b.op_id, blob))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let options = PackOptions {
        vendor,
        target,
        blob_alignment,
        kind,
        version: CLF_VERSION,
        sign,
    };

    if cli.dry_run {
        let total_blob: usize = blobs.iter().map(|(_, b)| b.len()).sum();
        eprintln!(
            "dry-run: would write {} ({} blobs, {} raw bytes, align={}, sign={})",
            output_path.display(),
            blobs.len(),
            total_blob,
            options.blob_alignment,
            options.sign
        );
        if from_manifest {
            eprintln!("dry-run: manifest had {} entries", resolved.blobs.len());
        }
        return Ok(());
    }

    let mut sidecar_blobs = Vec::new();
    if cli.write_sidecar {
        for bmeta in &resolved.blobs {
            let data = blobs
                .iter()
                .find(|(id, _)| *id == bmeta.op_id)
                .map(|(_, d)| d.as_slice())
                .ok_or("internal: missing blob for sidecar")?;
            sidecar_blobs.push(sidecar::SidecarBlob {
                op_id: bmeta.op_id,
                path: bmeta.path.display().to_string(),
                sha256_hex: sha256_hex(data),
                symbol: bmeta.symbol.clone(),
                notes: bmeta.notes.clone(),
            });
        }
    }

    let mut out = File::create(&output_path)?;
    let data_len = pack_clf(&mut out, &blobs, &options)?;
    if options.sign {
        out.sync_all()?;
        append_signature(&mut out, data_len)?;
    }
    out.sync_all()?;

    let total = data_len
        + if options.sign {
            clf::SIG_BLOCK_LEN as u64
        } else {
            0
        };
    eprintln!("wrote {} ({} bytes)", output_path.display(), total);

    if cli.write_sidecar {
        let side = sidecar_path(&output_path);
        let doc = sidecar::SidecarDocument::new(output_path.display().to_string(), sidecar_blobs);
        sidecar::write_sidecar_json(&side, &doc)?;
        eprintln!("wrote sidecar {}", side.display());
    }

    Ok(())
}

fn sidecar_path(output: &Path) -> PathBuf {
    let mut p = output.as_os_str().to_owned();
    p.push(".meta.json");
    PathBuf::from(p)
}

fn sha256_hex(data: &[u8]) -> String {
    let h = Sha256::digest(data);
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn verify_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = ClfReader::open(path)?;
    if !reader.signature_block_present() {
        return Err("verify: no SIG0 signature block".into());
    }
    match reader.verify_signature() {
        Ok(true) => {
            println!("verify: OK ({})", path.display());
            Ok(())
        }
        Ok(false) => Err("verify: invalid or unreadable SIG0 block".into()),
        Err(e) => Err(format!("verify: {e}").into()),
    }
}

fn inspect_file(path: &Path, verify: bool, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = ClfReader::open(path)?;

    if verify {
        match reader.verify_signature() {
            Ok(true) => {}
            Ok(false) => return Err("SIG0 missing or unreadable".into()),
            Err(e) => return Err(format!("signature verification failed: {e}").into()),
        }
    }

    if json {
        return inspect_json(&reader, path);
    }

    let h = &reader.header;
    println!("File: {}", path.display());
    println!("Format version: {}", h.version);
    println!(
        "Kind: {} (suggested extension .{})",
        h.kind,
        h.kind.extension()
    );
    if h.vendor.is_empty() {
        println!("Vendor: (empty)");
    } else {
        println!("Vendor: {}", h.vendor);
    }
    if h.target.is_empty() {
        println!("Target: (empty)");
    } else {
        println!("Target: {}", h.target);
    }
    println!("Blob alignment: {} bytes", h.blob_alignment);
    println!(
        "Blob store: offset {}  length {}",
        reader.blob_store_offset(),
        reader.blob_store_len()
    );
    println!(
        "Signature block: {}",
        if reader.signature_block_present() {
            "present"
        } else {
            "absent"
        }
    );

    let entries = reader.manifest_entries();
    println!("\nManifest ({} entries):", entries.len());
    println!("{:>8}  {:>10}  {:>12}", "op_id", "offset", "size (bytes)");
    for e in &entries {
        println!("{:>8}  {:>10}  {:>12}", e.op_id, e.offset, e.size);
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct InspectJson {
    file: String,
    format_version: u8,
    kind: String,
    kind_extension: &'static str,
    vendor: String,
    target: String,
    blob_alignment: u8,
    blob_store_offset: u64,
    blob_store_len: u64,
    signature_block_present: bool,
    manifest: Vec<clf::ManifestEntry>,
}

fn inspect_json(reader: &ClfReader, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let h = &reader.header;
    let out = InspectJson {
        file: path.display().to_string(),
        format_version: h.version,
        kind: h.kind.to_string(),
        kind_extension: h.kind.extension(),
        vendor: h.vendor.clone(),
        target: h.target.clone(),
        blob_alignment: h.blob_alignment,
        blob_store_offset: reader.blob_store_offset(),
        blob_store_len: reader.blob_store_len(),
        signature_block_present: reader.signature_block_present(),
        manifest: reader.manifest_entries(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
    );
    Ok(())
}
