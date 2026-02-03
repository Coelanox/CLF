//! Minimal example: build a .clf with two blobs (op_id 1 and 50), then read it back and print blob lengths.
//!
//! Run: cargo run --example build_and_read

use std::io::Cursor;

use clf::{pack_clf, ClfReader, PackOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<(u16, Vec<u8>)> = vec![
        (1, b"blob_for_add".to_vec()),
        (50, b"blob_for_matmul".to_vec()),
    ];
    let options = PackOptions {
        vendor: "example".to_string(),
        version: 1,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options)?;
    let bytes = buf.into_inner();

    let mut tmp = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut tmp, &bytes)?;
    let path = tmp.path();

    let mut reader = ClfReader::open(path)?;
    println!("vendor: {}", reader.header.vendor);
    println!("op_ids: {:?}", reader.op_ids());
    for op_id in reader.op_ids() {
        let blob = reader.get_blob(op_id)?.unwrap();
        println!("  op_id {}: {} bytes", op_id, blob.len());
    }
    Ok(())
}
