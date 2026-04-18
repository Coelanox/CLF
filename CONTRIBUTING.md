# Contributing to CLF

Thank you for helping improve CLF. This document explains how we work, what to run before opening a pull request, and how maintainers can finish one-time GitHub setup (description, topics, roadmap issues).

## Principles

- **Security first:** treat parsers and verification paths as trust boundaries. Prefer explicit bounds checks, clear errors, and tests over silent coercion.
- **Spec alignment:** behavior that affects on-disk layout or verification should match [SPEC.md](SPEC.md) and stay documented in [README.md](README.md) or `docs/` as appropriate.
- **Small, reviewable changes:** one logical change per PR when possible; avoid unrelated refactors mixed with fixes.

## Before you open a PR

1. **Build and test (same as CI):**

   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   ```

2. **License:** new files should remain under the project’s existing license terms (see repository `LICENSE` / `Cargo.toml`).

3. **Documentation:** if you change user-visible CLI behavior, verification, or format handling, update the relevant doc (`README.md`, `SPEC.md`, or `docs/…`) in the same PR.

## Fuzzing (optional but valuable)

The `fuzz/` crate exercises the reader against random inputs. After [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) is installed:

```bash
cd fuzz && cargo fuzz run clf_open
```

## Reporting security issues

Please report security-sensitive bugs through [GitHub Security Advisories](https://github.com/Coelanox/CLF/security/advisories) for this repository (or another channel the maintainers publish). Avoid posting exploit details in public issues before there is a coordinated fix.

## Maintainer: one-time GitHub visibility

The public API currently may show an empty description and no topics. To set a concise description, add discovery topics, and open labeled roadmap issues in one step, install the [GitHub CLI](https://cli.github.com/) and run:

```bash
bash scripts/gh-bootstrap-repo.sh
```

The script expects `gh auth login` (or `GH_TOKEN` with appropriate scopes) and repository permission to edit metadata and create issues. It is intended to be run **once**; re-running may create duplicate issues unless you adjust titles or close old ones first.

If you prefer manual steps, see the same script for the exact `gh repo edit` and `gh issue create` commands.

## Questions

Open a [discussion issue](https://github.com/Coelanox/CLF/issues) with what you are trying to build (producer, consumer, or tooling); point to the relevant doc section if you can.
