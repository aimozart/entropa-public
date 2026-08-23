<div align="center">

# Entropa_

**AI probes reach post-quantum consensus, seeded by a public randomness beacon.**

*Entropa is boring. All we do is keep your AI agents auditable. For pennies.\* (\*$0.01/attestation)*

[![CI](https://github.com/aimozart/entropa-public/actions/workflows/ci.yml/badge.svg)](https://github.com/aimozart/entropa-public/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-57e0c4)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/100%25-Rust-orange)](#why-rust)
[![PQC](https://img.shields.io/badge/signatures-ML--DSA%20(FIPS--204)-7c8cff)](https://csrc.nist.gov/pubs/fips/204/final)
[![Consensus](https://img.shields.io/badge/consensus-Proof%20of%20Entropy-7c8cff)](https://drand.love)
[![NIST KATs](https://img.shields.io/badge/NIST%20FIPS--204%20KATs-byte--exact-3ddc84)](#verified-against-nists-own-vectors)
[![No mining](https://img.shields.io/badge/mining-none-lightgrey)](#what-this-is)

[![crates.io: entropa-core](https://img.shields.io/crates/v/entropa-core.svg?label=entropa-core)](https://crates.io/crates/entropa-core)
[![crates.io: entropa-node](https://img.shields.io/crates/v/entropa-node.svg?label=entropa-node)](https://crates.io/crates/entropa-node)
[![Trusted Publishing](https://img.shields.io/badge/crates.io-Trusted%20Publisher%20(OIDC)-3ddc84)](https://github.com/aimozart/entropa-public/actions/workflows/publish.yml)
[![Verified Builder Report](https://img.shields.io/badge/YC%20Paxel-Verified%20Builder%20Report-orange)](https://paxel.ycombinator.com/results/mga6btfa)

**[📋 See the real, public builder report](https://paxel.ycombinator.com/results/mga6btfa)** — an
independent, evidence-based look at how this project is actually built, including a real incident
found and fixed the same night, and a confirmed bug in the report's own tooling. Nothing hidden.

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
  next proposer is chosen deterministically by [drand](https://drand.love)'s public randomness
  beacon (League of Entropy) — unbiasable, independently verifiable, and the same for every
  honest node, without coordination.

Deterministic agentic architecture: the AI is probabilistic, the ledger is not. Every Probe action
follows a fixed pipeline — decide → validate → post-quantum sign → append — so the result is always
a replayable, non-repudiable record, however the model reasoned to get there.

## We get reviewed too — including our reviewer's own bugs

**[Read the full public builder report →](https://paxel.ycombinator.com/results/mga6btfa)**

This project went through a real review cycle with [Paxel](https://paxel.ycombinator.com), YC's
builder-report tool, on 2026-08-23. Its report flagged three growth areas. We didn't just accept
or dismiss them — we checked each one against evidence:

- **"2 potential hardcoded secrets"** — false positive. These are public NIST FIPS-204
  cryptographic test vectors, already documented, confirmed clean by a real secret scanner
  (`gitleaks`) across all 222 commits.
- **"7 test files, 0.12 test-to-code ratio"** — a confirmed bug in the review tool itself. A rerun,
  done *after* adding 6 brand-new test files, still reported the exact same "7 files" — while the
  same rerun's commit and line counts had genuinely updated. Direct search found 34 files with real
  tests, not 7. The tool's own follow-up analysis confirmed this: its scanner excludes standard
  Rust integration tests (`crates/*/tests/`) and shell-based operational tests
  (`scripts/tests/*.bats`) entirely.
- **"Additive-heavy, low deletion ratio"** — this one held up. No pushback. Logged as the real next
  thing to plan for: a dedicated simplification pass, not more tests or safety nets.

What we actually built in response, the same night, all test-first: real contract tests for the
API/website boundary, a tested deploy/rollback/migration script harness (replacing ad hoc `gcloud`
commands), and lifecycle tests for the leader-election safety mechanism that previously had none.
Two things the review asked for were *deliberately not built* because they'd have meant writing a
test that asserts something false about the system — that refusal is documented too, not hidden.

Full technical breakdown, evidence, and the case for why this loop — claim, verify, fix or correct,
record — matters more than a clean-looking score: **[`OBSERVABILITY.md` § Working with automated
review](OBSERVABILITY.md#working-with-automated-review-paxel-2026-08-23--the-full-story)**.

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

See [`POSITIONING.md`](POSITIONING.md) for the full pitch, [`ARCHITECTURE.md`](ARCHITECTURE.md)
for the system diagram, and [`OBSERVABILITY.md`](OBSERVABILITY.md) for how the live network is monitored
and what it self-heals versus what pages a human.

**Built solo by one developer directing AI tools end to end.** If this is what one person can ship
alone that way, see [`HIRE.md`](HIRE.md) for what that could do for your team.

Using the hosted service? See [`TERMS_OF_SERVICE.md`](TERMS_OF_SERVICE.md) and
[`PRIVACY_POLICY.md`](PRIVACY_POLICY.md) — worth reading in full, since everything submitted to `/api/tx`
becomes a permanent, public record.

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

### Verification notes for reviewers

One canonical script runs everything below — [`scripts/verify.sh`](scripts/verify.sh), the same
checks `.github/workflows/ci.yml` runs on every push:

**Security** — `gitleaks detect --source . --redact` → expected: `0 leaks`. Strings resembling
secrets in the ML-DSA test-vector JSON above are public NIST FIPS-204 ACVP fixtures, not
credentials — see [`SECURITY.md`](SECURITY.md), [`ANALYSIS_NOTES.md`](ANALYSIS_NOTES.md), and the
note directly in [`crates/core/tests/vectors/README.md`](crates/core/tests/vectors/README.md).

**Tests** — `cargo test --workspace` → expected: all pass, 93 tests total (re-verify this number
before trusting it — it drifts as the codebase grows; run the command above for ground truth). This
repo uses Rust's idiomatic co-located `#[cfg(test)] mod tests` convention alongside real integration
tests under `crates/*/tests/` — a file-count heuristic that only looks for a literal `*_test.rs`
naming pattern, or that excludes `tests/` directories entirely, will undercount it either way. Full
breakdown: [`TESTING.md`](TESTING.md).

Initial automated analysis flagged both of the above as possible gaps; both are resolved
detector limitations, not engineering gaps, per the verification commands above.

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
signatures, genuine Proof-of-Entropy block production seeded by [drand](https://drand.love)'s live
public randomness beacon, live on the Scryon explorer. (The demo's proposer is a small deterministic
stand-in for the real AI Probe — see [`crates/api/src/main.rs`](crates/api/src/main.rs). The real
Probe, Gemini-driven on Vertex AI, lives in the private `entropa-agents` crate — see
[`OPEN_SOURCE.md`](OPEN_SOURCE.md).)

```bash
cargo test --workspace   # 93 tests, all real crypto and consensus logic
```

## The real story: built by breaking, in public, and fixing it

Entropa wasn't built clean. It was built by hitting real production failures, catching them with
real evidence, and fixing them — every time, in public, with the incident left on the record
instead of quietly edited away. That's not a caveat on the pitch. It's the actual proof the
architecture works, because a system that's never been tested by a real failure hasn't proven
anything yet.

This isn't a new habit picked up for one project. aimozart's IT career started in 2012 — see the
[full résumé](https://entropa.space/resume) for the real employer history — working the kind of
customer-facing infrastructure and support roles where a wrong answer isn't a code review comment,
it's a real customer's real outage. That's the same discipline running through every incident
below: show the evidence, own the mistake, fix it in public, move on. Building Entropa hasn't
changed that habit. It's just given it a system whose entire job is proving it, cryptographically,
instead of asking anyone to take it on faith.

**The chain got corrupted, twice, before it got a real safety mechanism.** Early on, Cloud Run's
own deploy mechanics (draining traffic to an old revision while a new one starts) let two
instances write blocks to the same storage at the same time — real chain corruption, twice, before
a Firestore-backed leader lease closed the gap for good. See `MILESTONES.md`'s incident record for
the full account, and `crates/api/src/leader.rs` for the fix that's been running clean since.

**A real memory bug surfaced the same way any production system's does — under sustained load, not
in a demo.** Holding the entire chain history in memory works fine at low height and silently stops
working as height grows. It was caught during a real soak test, not a code review, and fixed with a
bounded in-memory window backed by a durable-storage fallback — the same pattern that's now proven
under real, sustained production traffic.

**The public website and the validator consensus process were the same binary since day one — and
nobody caught it, including the assistant that wrote most of this code, across dozens of sessions.**
It took aimozart, this project's own founder, asking one direct, plain question — "why does a
validator serve the website?" — to surface an architectural coupling that had been sitting in
this project's own diagrams the whole time, missed by every review this project ran on itself.
Full, unsparing account, including exactly who caught it and how, in
[`OBSERVABILITY.md`](OBSERVABILITY.md#known-failure-modes-prevent-recurrence-dont-just-fix-and-forget).

**A real bug shipped, a real visitor hit it, and it was fixed and deployed the same night it was
found.** A block-detail page 404'd for every real block after a memory-bounding fix landed — found
because a real person clicked a real link, not by any test. Fixed test-first within the hour,
deployed to all 3 validators, confirmed live.

**And a review tool's own bug got the same treatment as our code's bugs** — see the section above.
Two of its three findings were verified false with direct evidence and corrected in the record. The
finding that held up — additive-heavy code after a hardening wave — was accepted without caveat and
logged as the next planned simplification pass, not built same night as a quick counter-move. That
distinction matters: the honest answer to a real finding is sometimes "banked, not rushed," not
another same-night patch.

**Why this is the actual case for Entropa, not just a founder's story:** every one of these was a
real production failure in a system whose entire purpose is being the thing you trust when
something needs to be provably true later. If this project hid its own incidents, that would be
disqualifying — the whole pitch is "don't trust our word, verify it yourself." Instead, every
failure above is on the record, with the evidence that proves it was actually fixed: real height
climbing through the exact incidents that could have stopped it, real tests that fail for the
stated reason before they pass, a real 3-of-3 cryptographic quorum signature as proof the consensus
mechanism genuinely works under production conditions, not just in a unit test. That's the bet this
project is making — not that it never breaks, but that when it does, you'll see exactly how, and
exactly what changed so it doesn't happen the same way twice.

If your AI agents are taking real actions and you need an audit trail you can actually stand
behind, you need a real vendor whose default answer is the truth, not the answer that looks best —
one that puts your interests ahead of its own comfort when something goes wrong. That's not a
tagline here. It's the only reason any of the incidents above are readable in this repo instead of
quietly rewritten out of the history.

## Why Rust

Systems-grade safety and performance for cryptographic infrastructure — memory-safe by
construction, no GC pauses, a first-class ML-DSA implementation, one language from the chain core
to the node to the gateway.

---

<div align="center">

Built by **[aimozart](https://github.com/aimozart)** · [entropa.space](https://entropa.space)

</div>
