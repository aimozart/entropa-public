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

## Current verified suite (2026-08-15)

- **12 source files** contain `#[cfg(test)] mod tests` blocks (`entropa-api`, `entropa-core`,
  `entropa-node`).
- **64 passing tests** total (`cargo test --workspace`).
- **1 integration test file** under `crates/core/tests/nist_kat.rs` — official FIPS-204 ACVP vectors,
  byte-exact ML-DSA-65 correctness against NIST's own published reference data.

See `SECURITY.md` § Static analysis notes for the related note on why heuristic scanners undercount
Rust's test layout, and the NIST-vector false-positive explanation for the secret-scanning finding.
