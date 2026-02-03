//! Reader tests: parse a minimal .clf and get_blob for two op_ids.

use std::io::{Cursor, Write};

use clf::{pack_clf, ClfReader, MissingOpIdPolicy, PackOptions};

/// Build a minimal .clf in memory (op_id 1 and 50, fake blobs), then open with ClfReader and get_blob.
#[test]
fn reader_parse_minimal_and_get_blob() {
    let entries: Vec<(u16, Vec<u8>)> = vec![
        (1, b"blob_for_add".to_vec()),
        (50, b"blob_for_matmul".to_vec()),
    ];
    let options = PackOptions {
        vendor: "test-vendor".to_string(),
        target: String::new(),
        blob_alignment: 0,
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

/// Invalid magic → clear error.
#[test]
fn reader_invalid_magic() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"XXXX").unwrap(); // wrong magic
    file.write_all(&[1u8]).unwrap();
    file.write_all(&0u16.to_le_bytes()).unwrap();
    file.flush().unwrap();

    match ClfReader::open(file.path()) {
        Ok(_) => panic!("expected invalid magic error"),
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("magic") || msg.contains("CLF1"), "expected invalid magic: {}", msg);
        }
    }
}

/// Truncated file (no manifest) → IO or clear error.
#[test]
fn reader_truncated_file() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&clf::CLF_MAGIC).unwrap();
    file.write_all(&[1u8]).unwrap();
    file.write_all(&0u16.to_le_bytes()).unwrap(); // vendor len 0
    file.write_all(&0u16.to_le_bytes()).unwrap(); // target len 0
    file.write_all(&[0u8]).unwrap();               // blob align 0
    // No manifest (num_entries) → read_exact will fail
    file.flush().unwrap();

    match ClfReader::open(file.path()) {
        Ok(_) => panic!("expected truncated file error"),
        Err(e) => assert!(!e.to_string().is_empty()),
    }
}

/// build_code_section with Fail policy: missing op_id returns error.
#[test]
fn reader_build_code_section_fail_on_missing() {
    let entries: Vec<(u16, Vec<u8>)> = vec![(1, b"a".to_vec())];
    let options = PackOptions::default();
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let op_ids = [1u16, 99]; // 99 not in CLF
    let err = reader
        .build_code_section(&op_ids, MissingOpIdPolicy::Fail)
        .unwrap_err();
    assert!(err.to_string().contains("99") || err.to_string().contains("missing"));
}

/// build_code_section with Skip policy: missing op_id skips, result non-empty.
#[test]
fn reader_build_code_section_skip_missing() {
    let entries: Vec<(u16, Vec<u8>)> = vec![(1, b"a".to_vec())];
    let options = PackOptions::default();
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let op_ids = [99u16, 1]; // 99 missing, 1 present
    let code = reader
        .build_code_section(&op_ids, MissingOpIdPolicy::Skip)
        .unwrap();
    assert_eq!(code, b"a");
}
