//! coelanox-packer CLI smoke tests (help, inspect).

use std::io::Write;
use std::process::Command;

use clf::CLF_VERSION;
use tempfile::NamedTempFile;

#[test]
fn coelanox_packer_help_exits_zero() {
    let bin = env!("CARGO_BIN_EXE_coelanox-packer");
    let out = Command::new(bin).arg("--help").output().expect("spawn");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("coelanox-packer") && text.contains("--inspect"));
}

#[test]
fn clf_help_is_primary_binary_name() {
    let bin = env!("CARGO_BIN_EXE_clf");
    let out = Command::new(bin).arg("--help").output().expect("spawn");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Usage: clf") && text.contains("--inspect"));
}

#[test]
fn coelanox_packer_inspect_shows_manifest() {
    let bin = env!("CARGO_BIN_EXE_coelanox-packer");
    let mut tmp = NamedTempFile::new().expect("temp");
    // Minimal valid blob
    tmp.write_all(&[0xc3]).expect("write");
    tmp.flush().expect("flush");
    let blob_path = tmp.path().to_owned();

    let out = NamedTempFile::new().expect("out");
    let clf_path = out.path().to_owned();

    let entry = format!("0:{}", blob_path.display());
    let pack = Command::new(bin)
        .arg("-o")
        .arg(&clf_path)
        .arg(&entry)
        .status()
        .expect("pack");
    assert!(pack.success(), "pack failed");

    let inspect = Command::new(bin)
        .args(["-i", clf_path.to_str().expect("utf8")])
        .output()
        .expect("inspect");
    assert!(inspect.status.success(), "inspect failed: {:?}", inspect);
    let s = String::from_utf8_lossy(&inspect.stdout);
    assert!(s.contains("Manifest"), "expected manifest section: {s}");
    assert!(s.contains("op_id"), "expected manifest table: {s}");
}

#[test]
fn coelanox_packer_inspect_json_stdout() {
    let bin = env!("CARGO_BIN_EXE_coelanox-packer");
    let mut tmp = NamedTempFile::new().expect("temp");
    tmp.write_all(&[0xc3]).expect("write");
    tmp.flush().expect("flush");
    let blob_path = tmp.path().to_owned();
    let out = NamedTempFile::new().expect("out");
    let clf_path = out.path().to_owned();
    let entry = format!("0:{}", blob_path.display());
    assert!(Command::new(bin)
        .arg("-o")
        .arg(&clf_path)
        .arg(&entry)
        .status()
        .expect("pack")
        .success());

    let inspect = Command::new(bin)
        .args(["-i", clf_path.to_str().expect("utf8"), "--json"])
        .output()
        .expect("inspect");
    assert!(inspect.status.success(), "{:?}", inspect);
    let json: serde_json::Value = serde_json::from_slice(&inspect.stdout).expect("valid JSON");
    assert_eq!(
        json["format_version"].as_u64(),
        Some(u64::from(CLF_VERSION)),
        "{json:?}"
    );
    assert!(json["manifest"].is_array());
}

#[test]
fn coelanox_packer_verify_unsigned_fails() {
    let bin = env!("CARGO_BIN_EXE_coelanox-packer");
    let mut tmp = NamedTempFile::new().expect("temp");
    tmp.write_all(&[0xc3]).expect("write");
    tmp.flush().expect("flush");
    let blob_path = tmp.path().to_owned();
    let out = NamedTempFile::new().expect("out");
    let clf_path = out.path().to_owned();
    let entry = format!("0:{}", blob_path.display());
    assert!(Command::new(bin)
        .arg("-o")
        .arg(&clf_path)
        .arg(&entry)
        .status()
        .expect("pack")
        .success());

    let st = Command::new(bin)
        .args(["--verify", clf_path.to_str().expect("utf8")])
        .status()
        .expect("verify");
    assert!(!st.success(), "unsigned clf should fail verify");
}
