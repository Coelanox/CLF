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

Each archive contains the `clf` CLI binary plus the matching installer at the archive root:

- Linux archive: `clf`, `install.sh` (copy of `scripts/install.sh` from the tagged tree)
- Windows archive: `clf.exe`, `install.ps1` (copy of `scripts/install.ps1`)

The same scripts live under `scripts/` in the repository; release archives bundle them so an unzip/tar extract is self-contained. The installers still download the matching archive from GitHub by default (override with `CLF_INSTALL_DIR` if you only want to place the binary you already extracted).

## Verification

After release creation, verify that:

1. All expected assets are present.
2. `SHA256SUMS` includes every uploaded archive.
3. Installer scripts can fetch `latest` successfully.

## Notes

- If you later add arm64 assets, keep naming consistent with current installer conventions:
  - `clf-aarch64-unknown-linux-gnu.tar.gz`
  - `clf-aarch64-pc-windows-msvc.zip`
