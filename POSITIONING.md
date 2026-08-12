# Entropa — Positioning

> **Entropa is a post-quantum trust layer for AI agents.** Written entirely in Rust.
> It is not a currency and has no mining — it is verifiable infrastructure you use like an API.

## The tagline
> **"Entropa is boring. All we do is keep you safe."**

## The one-liner
**AI probes reach post-quantum consensus, seeded by the entropy of space.**

## What it actually is
A tamper-evident, **post-quantum-signed ledger** whose validators are **AI agents (Probes)**
that reason about what to record, and whose ordering is decided by **Proof of Entropy** — an
unbiasable, energy-free consensus seeded by a cosmic randomness beacon.

Blockchain is the *mechanism*, never the pitch. The pitch is **trust and accountability for AI**.

## Three rising waves, one repo
| Wave | Why it matters in 2026 | Entropa's piece |
|---|---|---|
| **Post-quantum cryptography** | NIST-mandated migration; "harvest-now-decrypt-later" is a live threat. Most chains are *not* quantum-safe. | Native **ML-DSA / FIPS-204** signatures on every block. |
| **AI agents** | The most-funded frontier in software. | **Probes that reason** — validators that think, not miners that burn. |
| **Verifiable randomness** | Quietly in demand (leader election, gaming, sharding, lotteries). | The **Proof-of-Entropy beacon** — usable as a standalone service. |

Nobody else stands at the intersection of all three. That is the moat.

## Proof of Entropy (PoE)
- **No mining** — nothing to burn; zero energy consensus.
- **No staking plutocracy** — you can't buy your way to more blocks.
- **Unbiasable** — proposer selection is seeded by an external entropy beacon through a VRF,
  committed on-chain and independently verifiable by every node.

PoE sidesteps every standard criticism of crypto consensus (PoW wastes energy; PoS is rule-by-whales)
in a single stroke.

## Deterministic agentic architecture
The Probes use an LLM to *decide*, but the system around them is **deterministic and auditable** — the
model is probabilistic, the trust envelope is not. Every agent action follows a fixed pipeline:
**grounded context → structured decision → validate → post-quantum sign → append to the ledger.** The chain
is fully replayable (same inputs reproduce the same hashes), and PoE selection is a *verifiable function*
of the beacon (a VRF), not per-run randomness. The result: **probabilistic brains, deterministic
guarantees** — which is exactly what makes AI agents safe to rely on.

## What it's good for (not moving money)
1. **Post-quantum audit trails for AI agents** — an unforgeable record of what agents did and agreed,
   built to survive a quantum adversary. An emerging, unmet enterprise/compliance need.
2. **Multi-agent coordination records** — the shared, verifiable memory a swarm of agents can trust.
3. **Provenance & attestation** — "this output came from this agent, signed, at this time."
4. **Verifiable randomness as a service** — the PoE beacon, sold standalone.

## How it's worth something — utility, not speculation
Entropa is monetized like **Stripe or Twilio**, not like a memecoin:
- **Metered service.** Pay (in plain currency) per PQC attestation, per verifiable-randomness draw,
  per agent-audit record. The ledger is the trust mechanism, not an investment vehicle.
- **No mining, no speculative token.** If a usage unit ever exists, it is a **prepaid credit** minted
  when you pay for real work — worth exactly what it buys, pegged to utility, never mined or pumped.
- Result: it survives regulation, wastes no energy, and is honest infrastructure.

## Data-minimal & token-less (the regulatory moat)
We never hold customer data — only a **hash + PQC signature + timestamp**. So we're not a data processor:
no PII, nothing to breach, and the heavy data-privacy regime largely doesn't apply to us. It's a pure
**"don't trust me, prove it"** primitive — commit to data by its hash, then prove it existed and was signed
at time T **without ever revealing it**.

And **no fungible token.** Billing is plain USD (metered); at most a *non-transferable* prepaid credit — never
a tradeable coin. That keeps Entropa out of securities regulation and out of the crypto-speculation story. Our
only on-chain assets are **unique, verifiable receipts** (non-fungible).

**Net: you sell proof — not data, not money.** Almost no regulatory surface, cheap to run, safe to adopt.

## Why Rust
Systems-grade safety and performance for cryptographic infrastructure: no GC pauses, memory-safe by
construction, a first-class ML-DSA implementation, and a single language from the chain core to the
node to the agents to the gateway. Entropa is **100% Rust**.

## Architecture (all Rust, cargo workspace)
- `entropa-core` — Probe identities (ML-DSA/FIPS-204), transactions, blocks, verifiable chain.
- `entropa-node` — mempool + **Proof of Entropy** consensus engine.
- `entropa-agents` — the AI Probes (Claude-driven, with an offline fallback).
- `entropa-api` — axum JSON gateway + the **Scryon** explorer.
- `entropa-veyl` *(planned)* — **Veyl**, a smart-contract DSL.

Product surface: **Scryon** (explorer), **Veyl** (contract language), **Entrove** (wallet).
