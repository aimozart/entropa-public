# Security practices

## Secret scanning gate

Before any public push, employer-facing handoff, demo repo publication, or application asset submission:

1. Run gitleaks against committed git history:
   `gitleaks detect --source .`
2. If multiple repos are involved, run it in each repo.
3. Do not treat working-tree heuristic hits as confirmed leaks until checked against git history.
4. If test fixtures contain public cryptographic vectors, confirm they are standardized public test
   material and document why they are safe.
5. Never commit real API keys, tokens, private keys, service-account JSON, database URLs, or webhook
   secrets. Use environment variables or a secrets manager.

Run it via `scripts/security/secret-scan.sh` (scans committed history, not just the working tree —
build artifacts and local tool config routinely produce heuristic false positives that don't reflect
what's actually in the repo).

### Known safe pattern: NIST KAT vectors

`crates/core/tests/vectors/` and `crates/core/tests/nist_kat.rs` embed FIPS-204 ACVP test vectors as
string literals — hex-encoded key material, signatures, and seeds shaped exactly like real secrets to a
naive scanner. This is safe: it's NIST's own published, standardized, public test data
(`tests/vectors/fetch.sh` reproduces the full upstream set), never a real credential. Allowlisted in
`.gitleaks.toml` so a future rule change doesn't silently start flagging it.

## Dependency vulnerability scanning

Before any public push, employer-facing handoff, demo repo publication, or application asset
submission (same gate as secret scanning above):

1. Run `cargo audit` against the committed `Cargo.lock`: `./scripts/security/dependency-audit.sh`.
2. `Cargo.lock` is committed (not gitignored) — this is an application binary, not a pure library;
   `cargo audit` needs exact resolved versions, not the ranges in `Cargo.toml`.

Enforced continuously in CI (`dependency-audit` job, `.github/workflows/ci.yml`): runs on every push
*and* on a daily schedule, so a newly published CVE against an already-vendored dependency gets caught
within a day even with zero code changes — not just "checked when someone happens to push."

## Static analysis notes (for future builder-report / heuristic scan reads)

Written 2026-08-15 after a Paxel builder report (scoped to the private `entropa-chain` counterpart)
re-flagged three items that were already resolved or never real — so the *next* automated read has
less room to repeat the same stale/heuristic-blind findings as open remediation.

### FIPS-204 test vectors

Public NIST FIPS-204 test vectors (see "Known safe pattern" above) are intentional cryptographic
fixtures and may resemble key material to a naive scanner. **They are not secrets.** `gitleaks` is the
source of truth for secret scanning here, not a generic heuristic — full-history scans pass with
**0 leaks**, and `.gitleaks.toml` documents the allowlist. No rotation or history-scrubbing is
appropriate for public NIST fixtures — there is nothing to rotate.

### CI boundary

This repo, `entropa-public`, is where CI actually lives: `test`, `fmt+clippy`, `docker-build`,
`gitleaks` secret-scan, and `cargo-audit` dependency-scan — 5 jobs, all green, running on every push
plus a daily schedule for the dependency-audit job. Its private counterpart, `entropa-chain`
(Gitea-hosted), deliberately has **no** GitHub-hosted CI — it deploys through a local git hook straight
to Cloud Build so the proprietary `entropa-agents` crate never touches any third-party CI runner. A
scan of the private repo alone will correctly report "no CI" — that's a repo-topology fact given the
private/public split, not a missing engineering practice.

### Rust test layout

This workspace uses Rust's standard co-located test pattern — `#[cfg(test)] mod tests { ... }` inside
the same source file as the code it tests, or, once a test module grows past the file-size ceiling,
extracted into its own file via `#[path]` (e.g. `chain_tests.rs`) while remaining the exact same module
in the exact same crate. A heuristic that only counts files literally matching `*_test.rs`/`*_tests.rs`
naming, or only looks in a top-level `tests/` directory, will undercount real coverage here.
`cargo test --workspace` is the canonical verification command — **64 tests passing** as of this
writing, across `entropa-api`, `entropa-core`, and `entropa-node`.

## What actually gates writes/secrets in this project

`ENTROPA_API_KEYS`, `ENTROPA_STRIPE_CUSTOMERS`, and related config are environment variables /
GCP Secret Manager values on the live private deployment — never literals in source. This public repo
ships the open-core demo build (no proprietary agent brain, no billing/admin code) and has never held
those values.
