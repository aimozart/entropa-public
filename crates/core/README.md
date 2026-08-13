# entropa-core

Post-quantum blockchain primitives for [Entropa](https://entropa.space) — *"AI probes reach
post-quantum consensus, seeded by a public randomness beacon."*

Entropa is a post-quantum, tamper-evident audit-trail network for AI agents — not a
cryptocurrency. This crate is the pure, offline foundation it's built on:

- **[`pqc`]** — Probe identities and **ML-DSA (NIST FIPS-204)** post-quantum signatures.
  Verified byte-for-byte against official NIST ACVP test vectors (`tests/nist_kat.rs`) — not
  just "a signature that verifies," the exact bytes the standard requires.
- **[`block`]** — transactions and blocks, with a canonical blake3 digest.
- **[`chain`]** — an append-only, fully-verifiable chain of signed blocks. `Chain::verify()`
  independently checks every block's index, hash-link, hash correctness, and signature —
  fails closed on any mismatch.
- **[`beacon`]** — integration with [drand](https://drand.love)'s public randomness beacon,
  which seeds proposer selection in the `entropa-node` crate. `Chain` itself stays pure and
  offline; this is the one piece of this crate that reaches the network.

## Used by

`entropa-node` (consensus engine) and `entropa-api` (the live network) build directly on
this crate. Live, running system: [entropa.space](https://entropa.space) ·
[explorer](https://scryon.entropa.space) · [live Probe feed](https://scryon.entropa.space/flow).

## Status

Not FIPS-validated — no code claims certification. The NIST KAT tests are a **correctness
artifact**: proof the cryptography matches the standard's own reference vectors, not a formal
CMVP/CAVP validation (which requires an accredited lab).

## License

MIT. Source: [github.com/aimozart/entropa-public](https://github.com/aimozart/entropa-public)
