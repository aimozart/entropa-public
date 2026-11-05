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
Built and running: a signed Merkle ledger with Google Cloud KMS signing; an evidence ledger for healthcare data engineering; a decision review council (critic on Azure, contestor on Google, a Human Principal's decision last, with a computed quorum line); independent witnesses on Azure and AWS that co-sign the AI-decision ledger; an in-browser verifier; per-module pipelines with signed receipts; a decision memory and an agent harness used by the council's models; an independent auditor loop (report published). Known limits: the three ledgers share one signing key, so only the AI-decision ledger is witnessed; scheduling of council sessions is manual; the per-module workflow is not yet on a hosted CI runner. Wording: aligned with and mapped to HIPAA, HITRUST, NIST AI RMF and ISO/IEC 42001 themes; not assessed, not certified.

## Download the node (Linux)
The node and the checkpoint witness are published as free binaries (source not published): [node-v0.2.0](https://github.com/aimozart/entropa-public/releases/tag/node-v0.2.0), x86_64 Linux, glibc 2.34 or newer, with a checksum file. Exact commands: https://docs.entropa.space/install-linux

## Tested on Linux
The install commands and every node type were run on fresh server images of **Ubuntu 24.04.5 LTS** (apt) and **Rocky Linux 10.2** (dnf) on 2026-11-05: create a node, record AI decisions, audit evidence and council steps, verify the three ledgers, serve them over HTTP, and cross-sign a checkpoint with a witness node; the open-core test suite (106 tests) passes on both. Exact packages per step and the bugs the test found: [LINUX-TEST-MATRIX](https://github.com/aimozart/scryon-health-data/blob/main/docs/LINUX-TEST-MATRIX.md). Install commands: https://docs.entropa.space/install-linux (Linux only).

## Method
Tests first (red, then green); fuzzing and mutation testing on parsing and verification; secret scanning (gitleaks) and a public-repo preflight before every push; signed, data-minimal records; directed AI assistance, reviewed and verified by tests.
