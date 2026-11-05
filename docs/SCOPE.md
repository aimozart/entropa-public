# Scope: Entropa now, and compared with the older Rust site

## Full scope
1. Audit trail for AI decisions (signed, hash-only, plain English).
2. Audit evidence for healthcare data engineering (Scryon Health Data), filed under HIPAA/HITRUST-mapped control codes.
3. Public verification: explorers, in-browser verifier, independent witnesses on other clouds.
4. Guardrails for AI agents: MCP server, hook, Cedar gate, approvals, kill switch (design).
5. Planned: decision review council with a contestor, decision memory (retrieval as evidence, never authority), cloud agent harness, independent auditor loop.

## Older Rust site (branch `rust-archive`, summer 2026) versus now
| Topic | Older Rust site | Now |
|---|---|---|
| Core idea | Open-core demo: N=3 quorum consensus seeded by a public randomness beacon | Single-operator Merkle transparency log with signed checkpoints and independent witnesses |
| Why changed | A single-operator ledger never had the multi-validator trust problem the quorum solved | See OBSERVABILITY.md in `rust-archive` for the load-test story |
| Signing | ML-DSA keys held by nodes | ML-DSA-65 in a managed cloud KMS; local key files for local nodes |
| Records | Attestations | Closed-vocabulary, hash-only events; two ledgers (AI decisions, health evidence) |
| Verification | Rust crates | Rust crates plus a browser verifier that needs only the public key |
| Product surface | Signup, billing, dashboard, admin | None: no accounts, no logins, no demos; contact form only |
| Hosting | Container demo | Serverless: Cloud Run, Firebase Hosting, witness on Azure Container Apps |
| Source | Open core (MIT) | Production source private; this repo is docs, specs, vectors, and the original MIT crates |
| Pricing | $0.01 per attestation | Free for now; later either stops when credits end or about $0.01 per recorded decision; free to an organisation that hires the author, or purchasable outright (private) |
