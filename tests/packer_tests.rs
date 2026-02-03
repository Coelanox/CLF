//! Packer tests: produce a .clf and read it back with the reader.

use std::io::{Cursor, Write};

use clf::{append_signature, pack_clf, ClfReader, PackOptions};

/// Produce a .clf in memory (two blobs), then read it back with ClfReader and verify blobs.
#[test]
fn packer_produce_and_read_back() {
    let entries: Vec<(u32, Vec<u8>)> = vec![
        (1, vec![0x01, 0x02, 0x03]),
        (50, vec![0x50, 0x51, 0x52, 0x53]),
    ];
    let options = PackOptions {
        vendor: "packer-test".to_string(),
        target: String::new(),
        blob_alignment: 0,
        version: 1,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    let _data_len = pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();

    // Write to temp file so ClfReader::open(path) can use it.
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    assert_eq!(reader.header.vendor, "packer-test");

    let b1 = reader.get_blob(1).unwrap().unwrap();
    assert_eq!(b1, &[0x01, 0x02, 0x03]);

    let b50 = reader.get_blob(50).unwrap().unwrap();
    assert_eq!(b50, &[0x50, 0x51, 0x52, 0x53]);
}

/// Produce a signed .clf and verify signature when reading.
#[test]
fn packer_signed_and_verify() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(10, b"relu_kernel".to_vec())];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        version: 1,
        sign: true,
    };

    let mut file = tempfile::NamedTempFile::new().unwrap();
    let data_len = pack_clf(&mut file, &entries, &options).unwrap();
    file.flush().unwrap();
    append_signature(&mut file, data_len).unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    let blob = reader.get_blob(10).unwrap().unwrap();
    assert_eq!(blob, b"relu_kernel");
    let ok = reader.verify_signature().unwrap();
    assert!(ok);
    assert!(reader.signature_verified());
}

/// Produce .clf with blob alignment 16; reader returns stored (padded) blob.
#[test]
fn packer_blob_alignment() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, vec![0x01, 0x02])]; // 2 bytes
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 16,
        version: 1,
        sign: false,
    };

    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();

    let mut reader = ClfReader::open(file.path()).unwrap();
    assert_eq!(reader.header.blob_alignment, 16);
    let blob = reader.get_blob(1).unwrap().unwrap();
    assert_eq!(blob.len(), 16); // padded to 16
    assert_eq!(&blob[..2], &[0x01, 0x02]);
}
