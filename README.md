<div align="center">

# Entropa_

**AI probes reach post-quantum consensus, seeded by the entropy of space.**

*Entropa is boring. All we do is keep you safe.*

[![CI](https://github.com/aimozart/entropa-public/actions/workflows/ci.yml/badge.svg)](https://github.com/aimozart/entropa-public/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-57e0c4)](LICENSE-MIT)
![Rust](https://img.shields.io/badge/100%25-Rust-orange)
![PQC](https://img.shields.io/badge/signatures-ML--DSA%20(FIPS--204)-7c8cff)
![Consensus](https://img.shields.io/badge/consensus-Proof%20of%20Entropy-7c8cff)
![NIST KATs](https://img.shields.io/badge/NIST%20FIPS--204%20KATs-byte--exact-3ddc84)
![No mining](https://img.shields.io/badge/mining-none-lightgrey)

</div>

---

## What this is

Entropa is a **post-quantum trust layer for AI agents** — not a currency. There's no mining, no
staking, and no token to speculate on. It's infrastructure you'd use like an API: send a hash, get
back a tamper-proof, quantum-safe, timestamped receipt.

It's built around three ideas at once:

- **Post-quantum cryptography.** Every block is signed with **ML-DSA (NIST FIPS-204)** — the
  lattice-based signature scheme built to survive a future quantum adversary. Most chains today
  aren't quantum-safe. This one is, from block zero.
- **AI Probes.** The network's validators are AI agents ("Probes") that reason about what to
  record, rather than miners burning electricity on arbitrary puzzles.
- **Proof of Entropy (PoE).** Consensus with no mining and no staking plutocracy. Each round, the
  next proposer is chosen deterministically by a **cosmic entropy beacon** — unbiasable, and the
  same for every honest node, without coordination. Order, drawn from the entropy of space.

Deterministic agentic architecture: the AI is probabilistic, the ledger is not. Every Probe action
follows a fixed pipeline — decide → validate → post-quantum sign → append — so the result is always
a replayable, non-repudiable record, however the model reasoned to get there.

## Why you need Entropa

If you're deploying AI agents that take real actions — approving transactions, moving data,
making decisions on your behalf — you have a problem most teams haven't solved yet:

- **You can't prove what your agent did.** Logs can be edited. Databases can be modified after the
  fact. If an agent's decision is ever questioned — by a customer, an auditor, a regulator — "trust
  our logs" is not an answer. You need a record that's tamper-evident *by construction*, not by
  policy.
- **Your current crypto has an expiration date.** RSA and ECDSA — what almost every system uses
  today — will be broken by a sufficiently capable quantum computer. Adversaries are already
  harvesting encrypted data now to decrypt later ("harvest now, decrypt later"). If your agents'
  actions need to stay provably authentic for years, signing them with pre-quantum crypto is
  signing with a clock running out.
- **AI accountability is becoming a requirement, not a nice-to-have.** Regulators, enterprise
  customers, and your own risk team increasingly want to know: what did the agent decide, when, and
  can you prove it wasn't altered afterward. Entropa is built to answer that with math, not paperwork.
- **You shouldn't have to hand over your data to prove it.** Most audit/compliance tooling wants
  your raw data. Entropa only ever sees a hash — you get provable integrity without a new place for
  your data to leak from.

In short: **if an AI agent does something today, and someone asks you to prove it in five years,
Entropa is the layer that makes "yes, and here's the signed, quantum-safe proof" possible.**

## What it's for

Not moving money. **Proof.**

- **Post-quantum audit trails for AI agents** — an unforgeable record of what an agent did and
  decided, built to survive a quantum adversary.
- **Multi-agent coordination records** — shared, verifiable memory a swarm of agents can trust.
- **Provenance & attestation** — "this came from this agent, signed, at this time."
- **Verifiable randomness** — the Proof-of-Entropy beacon, usable standalone.

Entropa never holds your data — only a hash, a signature, and a timestamp. It's a "don't trust me,
let me prove it" primitive, not a data processor.

See [`POSITIONING.md`](POSITIONING.md) for the full pitch and [`ARCHITECTURE.md`](ARCHITECTURE.md)
for the system diagram.

## Verified against NIST's own vectors

The cryptography isn't "trust me, it's post-quantum." It's asserted **byte-for-byte** against
NIST's official ACVP known-answer test vectors for ML-DSA-65 (FIPS 204), committed verbatim
in [`crates/core/tests/vectors/`](crates/core/tests/vectors/):

| What's checked | Vectors |
|---|---|
| `ML-DSA.KeyGen_internal` — seed ξ → public key + expanded signing key | 12 |
| `ML-DSA.Sign_internal` — deterministic (rnd = 0³²) | 8 |
| `ML-DSA.Sign` — full external path, including context string | 8 |
| Verification of NIST's own reference signatures | 16 |
| Rejection of tampered signatures, messages, and contexts | 8 |

```bash
cargo test -p entropa-core --test nist_kat
```

A committed subset keeps CI fast; [`fetch.sh`](crates/core/tests/vectors/fetch.sh) pulls the
complete upstream set so the claim is independently checkable, not cherry-picked.

> **This is a correctness artifact, not a certification.** Entropa is **not** FIPS-validated
> and claims no such thing — formal validation requires an accredited CMVP/CAVP lab. What
> these tests show is that the implementation is built to the standard and demonstrably
> matches it.

## This repo (open core)

| Crate | What it is |
|---|---|
| [`entropa-core`](crates/core) | ML-DSA (FIPS-204) Probe identities, transactions, blocks, a fully verifiable chain |
| [`entropa-node`](crates/node) | Mempool + **Proof of Entropy** consensus engine |
| [`entropa-api`](crates/api) | axum JSON gateway + **Scryon**, the live star-map explorer |

The AI Probe intelligence (`entropa-agents` — the actual decision-making brain) is proprietary and
lives in a private repo. This repo is the open core: the cryptography, the consensus protocol, and
the explorer, all **MIT-licensed**. See [`OPEN_SOURCE.md`](OPEN_SOURCE.md) for the full split and why.

## Run it locally

```bash
git clone https://github.com/aimozart/entropa-public.git entropa
cd entropa
cargo run -p entropa-api
```

Open **http://localhost:8080** — a real single-node network starts immediately: genuine ML-DSA
signatures, genuine Proof-of-Entropy block production, live on the Scryon explorer. (The demo's
proposer is a small deterministic stand-in for the real AI Probe — see
[`crates/api/src/main.rs`](crates/api/src/main.rs).)

```bash
cargo test --workspace   # 18 tests, all real crypto and consensus logic
```

## Why Rust

Systems-grade safety and performance for cryptographic infrastructure — memory-safe by
construction, no GC pauses, a first-class ML-DSA implementation, one language from the chain core
to the node to the gateway.

---

<div align="center">

Built by **[aimozart](https://github.com/aimozart)** · [entropa.space](https://entropa.space)

</div>
