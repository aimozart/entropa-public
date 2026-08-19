# Testing

This repository uses idiomatic Rust co-located unit tests via `#[cfg(test)] mod tests` — tests live
beside the code they exercise, not separated into a parallel directory structure. Integration tests
(crypto correctness against an external reference dataset) live under `crates/core/tests/`.

A test-file count that only looks inside directories literally named `tests/` will undercount this
repository — most of its real, passing test coverage lives inside source files as co-located unit
test modules, which is the standard, idiomatic way to write and organize tests in Rust.

Run the real suite:

```bash
cargo test --workspace
```

or `scripts/verify.sh`, the canonical entrypoint that also runs fmt/clippy/gitleaks/cargo-audit
(same checks as `.github/workflows/ci.yml`).

## Current verified suite (2026-08-19)

- **13 source files** contain `#[cfg(test)] mod tests` blocks (`entropa-api`, `entropa-core`,
  `entropa-node`).
- **72 passing tests** total (`cargo test --workspace`).
- **1 integration test file** under `crates/core/tests/nist_kat.rs` — official FIPS-204 ACVP vectors,
  byte-exact ML-DSA-65 correctness against NIST's own published reference data.

These numbers move as the code grows — re-run `cargo test --workspace` rather than trusting this
file if it's more than a session or two old. See `SECURITY.md` § Static analysis notes and
`ANALYSIS_NOTES.md` for the related note on why heuristic scanners undercount Rust's test layout,
and the NIST-vector false-positive explanation for the secret-scanning finding.
