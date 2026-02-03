//! Reader tests: parse a minimal .clf and get_blob for two op_ids.

use std::io::{Cursor, Write};

use clf::{pack_clf, ClfReader, PackOptions};

/// Build a minimal .clf in memory (op_id 1 and 50, fake blobs), then open with ClfReader and get_blob.
#[test]
fn reader_parse_minimal_and_get_blob() {
    let entries: Vec<(u16, Vec<u8>)> = vec![
        (1, b"blob_for_add".to_vec()),
        (50, b"blob_for_matmul".to_vec()),
    ];
    let options = PackOptions {
        vendor: "test-vendor".to_string(),
        version: 1,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    let _ = pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();

    // Re-open as CLF via reader (from bytes: write to temp file then open, or use a reader that works on bytes).
    // ClfReader::open takes a path, so we need a temp file.
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut file, &bytes).unwrap();
    file.flush().unwrap();

    let path = file.path();
    let mut reader = ClfReader::open(path).unwrap();

    assert_eq!(reader.header.version, 1);
    assert_eq!(reader.header.vendor, "test-vendor");
    assert!(reader.op_ids().contains(&1));
    assert!(reader.op_ids().contains(&50));

    let blob1 = reader.get_blob(1).unwrap().unwrap();
    assert_eq!(blob1, b"blob_for_add");

    let blob50 = reader.get_blob(50).unwrap().unwrap();
    assert_eq!(blob50, b"blob_for_matmul");

    let missing = reader.get_blob(99).unwrap();
    assert!(missing.is_none());
}
