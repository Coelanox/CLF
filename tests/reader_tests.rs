//! Reader tests: parse a minimal .clf and get_blob for two op_ids.

use std::io::{Cursor, Write};

use clf::{pack_clf, ClfKind, ClfReader, MissingOpIdPolicy, PackOptions};

/// Build a minimal .clf in memory (op_id 1 and 50, fake blobs), then open with ClfReader and get_blob.
#[test]
fn reader_parse_minimal_and_get_blob() {
    let entries: Vec<(u32, Vec<u8>)> = vec![
        (1, b"blob_for_add".to_vec()),
        (50, b"blob_for_matmul".to_vec()),
    ];
    let options = PackOptions {
        vendor: "test-vendor".to_string(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::Compute,
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
    file.write_all(&0u32.to_le_bytes()).unwrap(); // vendor len
    file.flush().unwrap();

    match ClfReader::open(file.path()) {
        Ok(_) => panic!("expected invalid magic error"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("magic") || msg.contains("CLF1"),
                "expected invalid magic: {}",
                msg
            );
        }
    }
}

/// Truncated file (no manifest) → IO or clear error.
#[test]
fn reader_truncated_file() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&clf::CLF_MAGIC).unwrap();
    file.write_all(&[1u8]).unwrap();
    file.write_all(&0u32.to_le_bytes()).unwrap(); // vendor len 0
    file.write_all(&0u32.to_le_bytes()).unwrap(); // target len 0
    file.write_all(&[0u8]).unwrap(); // blob align 0
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
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"a".to_vec())];
    let options = PackOptions::default();
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let op_ids = [1u32, 99]; // 99 not in CLF
    let err = reader
        .build_code_section(&op_ids, MissingOpIdPolicy::Fail)
        .unwrap_err();
    assert!(err.to_string().contains("99") || err.to_string().contains("missing"));
}

/// v2 file with kind MemoryMovement: parse and expose kind.
#[test]
fn reader_parse_kind_v2() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"blob".to_vec())];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::MemoryMovement,
        version: 2,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let reader = ClfReader::open(file.path()).unwrap();
    assert_eq!(reader.header.version, 2);
    assert_eq!(reader.header.kind, ClfKind::MemoryMovement);
}

/// open_with_expected_kind: reject when kind mismatch.
#[test]
fn reader_expected_kind_mismatch() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"blob".to_vec())];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::Compute,
        version: 2,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let err =
        ClfReader::open_with_expected_kind(file.path(), Some(ClfKind::MemoryMovement)).unwrap_err();
    assert!(err.to_string().contains("mismatch") || err.to_string().contains("Kind"));
}

/// open_with_expected_kind: succeed when kind matches.
#[test]
fn reader_expected_kind_matches() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"blob".to_vec())];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::MemoryProtection,
        version: 2,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let reader =
        ClfReader::open_with_expected_kind(file.path(), Some(ClfKind::MemoryProtection)).unwrap();
    assert_eq!(reader.header.kind, ClfKind::MemoryProtection);
}

/// v1 file: kind defaults to Compute.
#[test]
fn reader_v1_kind_defaults_to_compute() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"blob".to_vec())];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::Compute,
        version: 1,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let reader = ClfReader::open(file.path()).unwrap();
    assert_eq!(reader.header.version, 1);
    assert_eq!(reader.header.kind, ClfKind::Compute);
}

/// build_code_section with Skip policy: missing op_id skips, result non-empty.
#[test]
fn reader_build_code_section_skip_missing() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, b"a".to_vec())];
    let options = PackOptions::default();
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let op_ids = [99u32, 1]; // 99 missing, 1 present
    let code = reader
        .build_code_section(&op_ids, MissingOpIdPolicy::Skip)
        .unwrap();
    assert_eq!(code, b"a");
}

/// `blobs_iter` matches manifest order and blob bytes.
#[test]
fn reader_blobs_iter_matches_get_blob() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(2, b"bb".to_vec()), (1, b"a".to_vec())];
    let options = PackOptions::default();
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let from_iter: Vec<_> = reader.blobs_iter().map(|r| r.unwrap()).collect();
    assert_eq!(from_iter.len(), 2);
    assert_eq!(from_iter[0].0, 1);
    assert_eq!(from_iter[0].1, b"a");
    assert_eq!(from_iter[1].0, 2);
    assert_eq!(from_iter[1].1, b"bb");
}

/// v2 file with unknown kind byte must be rejected.
#[test]
fn reader_rejects_invalid_kind_byte() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&clf::CLF_MAGIC);
    bytes.push(2); // version
    bytes.extend_from_slice(&0u32.to_le_bytes()); // vendor len
    bytes.extend_from_slice(&0u32.to_le_bytes()); // target len
    bytes.push(0); // alignment
    bytes.push(0xFF); // invalid kind
    bytes.extend_from_slice(&0u32.to_le_bytes()); // num_entries

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let err = ClfReader::open(file.path()).unwrap_err();
    assert!(
        err.to_string().contains("invalid kind byte"),
        "unexpected error: {err}"
    );
}

/// Oversized vendor length must not trigger large allocations.
#[test]
fn reader_rejects_oversized_vendor_length() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&clf::CLF_MAGIC);
    bytes.push(2); // version
    bytes.extend_from_slice(&((70 * 1024) as u32).to_le_bytes()); // vendor len > cap

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let err = ClfReader::open(file.path()).unwrap_err();
    assert!(
        err.to_string().contains("vendor too large"),
        "unexpected error: {err}"
    );
}

/// Manifest count larger than available bytes must be rejected early.
#[test]
fn reader_rejects_manifest_count_overflow() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&clf::CLF_MAGIC);
    bytes.push(2); // version
    bytes.extend_from_slice(&0u32.to_le_bytes()); // vendor len
    bytes.extend_from_slice(&0u32.to_le_bytes()); // target len
    bytes.push(0); // align
    bytes.push(0); // compute kind
    bytes.extend_from_slice(&2u32.to_le_bytes()); // num_entries says 2
    bytes.extend_from_slice(&1u32.to_le_bytes()); // only one partial entry follows
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let err = ClfReader::open(file.path()).unwrap_err();
    assert!(
        err.to_string()
            .contains("manifest entry count exceeds available file data"),
        "unexpected error: {err}"
    );
}
