# CLF signing

CLF v1 uses an optional **integrity tail** at the end of the file (see [SPEC.md](../SPEC.md)):

- **Magic** `SIG0` (4 bytes)
- **SHA-256** (32 bytes) over **all bytes before this block** (header + manifest + blob store)

Producers run `coelanox-packer --sign` (or call `append_signature` after `pack_clf`). Consumers call `ClfReader::verify_signature()` before trusting the archive.

## CLI

| Command | Purpose |
|---------|---------|
| `coelanox-packer --verify path.clf` | Exit `0` if SIG0 is present and the hash matches; non-zero otherwise (CI-friendly). |
| `coelanox-packer -i path.clf --verify-signature` | Inspect output only after a successful hash check. |

## Per-blob integrity (sidecar)

The optional **`*.meta.json`** file (see [PRODUCER_GUIDE.md](PRODUCER_GUIDE.md)) records **SHA-256 per blob** at pack time. It does not replace SIG0; it helps audit which object file produced which slice of the blob store.

## Ed25519 (future)

A possible future extension is a second block (e.g. public key + Ed25519 signature) over the same or a derived domain. That would be a **format revision** and must stay coordinated with Coelanox consumers. Until then, SIG0 + SHA-256 remains the on-disk integrity mechanism.
