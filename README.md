# Entropa

**A tamper-evident, post-quantum audit trail for AI decisions and data-engineering evidence.** Every record is hash-only, signed with ML-DSA-65 (FIPS 204), and appended to a Merkle transparency log whose checkpoints anyone can verify with a public key. No accounts, no logins, no demos.

- Site: https://entropa.space · Docs: https://docs.entropa.space
- AI Audit Explorer: https://scryon.entropa.space
- Health Data Explorer: https://scryon-health.entropa.space (verifies every signature in your browser)
- Crates on crates.io (Trusted Publishing): `entropa-core`, `entropa-node`

## What is in this repository
This repository is the **public face** of Entropa: the architecture, the scope, the record and verification specification, and the original MIT-licensed open-core crates (`crates/`, branch history preserved on `rust-archive`). The production source is developed privately. Public here means: documents, specs, test vectors and code we are glad to have public. Never keys, credentials, deployment identifiers or real data. All data is synthetic.

| Read | For |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | How the current system fits together |
| [docs/SCOPE.md](docs/SCOPE.md) | Full scope, and the new Rust system compared with the older Rust site |
| [docs/archive/README-2026-08-open-core.md](docs/archive/README-2026-08-open-core.md) | The previous README (open-core demo) |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability |

## Status (2026-11-05)
Built and running: signed Merkle ledger with Google Cloud KMS signing, evidence ledger for healthcare data engineering, in-browser verifier, independent checkpoint witness on Azure (temporary, credit-funded), contact service, public explorers, docs site. Planned, not built: decision review council, decision memory, cloud agent harness, AWS witness. Wording: aligned with and mapped to HIPAA, HITRUST, NIST AI RMF and ISO/IEC 42001 themes; not assessed, not certified.

## Method
Tests first (red, then green); fuzzing and mutation testing on parsing and verification; secret scanning (gitleaks) and a public-repo preflight before every push; signed, data-minimal records; directed AI assistance, reviewed and verified by tests.
