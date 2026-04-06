#![no_main]

use std::io::Write;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 512 * 1024 {
        return;
    }
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let p = dir.path().join("input.clf");
    if std::fs::File::create(&p).and_then(|mut f| f.write_all(data)).is_err() {
        return;
    }
    let _ = clf::ClfReader::open(&p);
});
