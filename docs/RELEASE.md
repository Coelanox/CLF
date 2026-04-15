# CLF release runbook

Last updated: 2026-04-15

## Purpose

This page defines how CLF releases are cut and which assets are expected for vendor installation scripts.

## Triggering a release

Release workflow: `.github/workflows/release.yml`

It runs automatically when you push a tag that matches `v*`.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Published assets

The release workflow builds and uploads:

- `clf-x86_64-unknown-linux-gnu.tar.gz`
- `clf-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS`

Each archive contains the `clf` CLI binary only:

- Linux archive: `clf`
- Windows archive: `clf.exe`

These names are consumed by:

- `scripts/install.sh`
- `scripts/install.ps1`

## Verification

After release creation, verify that:

1. All expected assets are present.
2. `SHA256SUMS` includes every uploaded archive.
3. Installer scripts can fetch `latest` successfully.

## Notes

- If you later add arm64 assets, keep naming consistent with current installer conventions:
  - `clf-aarch64-unknown-linux-gnu.tar.gz`
  - `clf-aarch64-pc-windows-msvc.zip`
