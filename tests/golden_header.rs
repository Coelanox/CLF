//! Deterministic header bytes for a fixed pack (magic + version smoke test).

use std::io::Cursor;

use clf::{pack_clf, ClfKind, PackOptions, CLF_MAGIC, CLF_VERSION};

#[test]
fn golden_packed_header_starts_with_clf_magic() {
    let entries: Vec<(u32, Vec<u8>)> = vec![(1, vec![0u8])];
    let options = PackOptions {
        vendor: String::new(),
        target: String::new(),
        blob_alignment: 0,
        kind: ClfKind::Compute,
        version: CLF_VERSION,
        sign: false,
    };
    let mut buf = Cursor::new(Vec::new());
    pack_clf(&mut buf, &entries, &options).unwrap();
    let bytes = buf.into_inner();
    assert_eq!(&bytes[0..4], &CLF_MAGIC);
    assert_eq!(bytes[4], CLF_VERSION);
}
