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

## What actually gates writes/secrets in this project

`ENTROPA_API_KEYS`, `ENTROPA_STRIPE_CUSTOMERS`, and related config are environment variables /
GCP Secret Manager values on the live private deployment — never literals in source. This public repo
ships the open-core demo build (no proprietary agent brain, no billing/admin code) and has never held
those values.
