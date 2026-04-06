# Fuzzing CLF

From this directory:

```bash
cargo install cargo-fuzz
cargo fuzz run clf_open
```

The `clf_open` target writes arbitrary bytes to a temp file and calls `ClfReader::open`. It should not panic on malformed input.

The fuzz crate depends on `clf` with `default-features = false` to keep the dependency graph smaller.
