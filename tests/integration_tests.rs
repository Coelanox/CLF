//! Integration test: simulate packager with CLF backend — build code section from .clf with target match, assert non-empty.

use std::io::Write;

use clf::{pack_clf, ClfReader, MissingOpIdPolicy, PackOptions};

/// Drop a minimal .clf (target CPU, two blobs) in a temp path, open with reader, build code section
/// from op_ids [1, 50] (execution order), assert code section is non-empty and contains both blobs.
#[test]
fn packager_clf_backend_produces_non_empty_code() {
    let entries: Vec<(u32, Vec<u8>)> = vec![
        (1, b"add_kernel".to_vec()),
        (50, b"matmul_kernel".to_vec()),
    ];
    let options = PackOptions {
        vendor: "integration-test".to_string(),
        target: "CPU".to_string(),
        blob_alignment: 0,
        version: 1,
        sign: false,
    };

    let mut file = tempfile::NamedTempFile::new().unwrap();
    pack_clf(&mut file, &entries, &options).unwrap();
    file.flush().unwrap();
    let path = file.path();

    let mut reader = ClfReader::open(path).unwrap();
    assert_eq!(reader.header.target, "CPU");

    // Simulate packager: execution order op_ids for a tiny "model"
    let op_ids = [1u32, 50];
    let code = reader.build_code_section(&op_ids, MissingOpIdPolicy::Fail).unwrap();

    assert!(!code.is_empty(), "code section must be non-empty when CLF has both ops");
    assert!(code.starts_with(b"add_kernel"));
    assert!(code.ends_with(b"matmul_kernel"));
}
