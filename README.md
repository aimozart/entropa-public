# entropa-public

This repository exists for two packages published on [crates.io](https://crates.io): **`entropa-core`** and **`entropa-node`**. Publishing is done from here by GitHub Actions with crates.io Trusted Publishing (no stored tokens).

## What the crates are for

- **`entropa-core`**: post-quantum signatures (ML-DSA, NIST FIPS-204, checked byte for byte against the official NIST test vectors), canonical hashing, and a verifiable, append-only chain of signed blocks. It is a pure, offline library.
- **`entropa-node`**: a mempool and a proposer-selection and consensus engine built on `entropa-core`.

## A brief history

Entropa began in 2026 as a post-quantum, tamper-evident network for recording what AI agents do, built as a small blockchain-style chain. These two crates are that first design, published for anyone to use or learn from. The project later moved to a simpler, stronger design: a signed, append-only Merkle log with independent witnesses and no consensus network. That work lives elsewhere and is not published from this repository.

## Licence

The crates are published under the MIT licence (`LICENSE-MIT`). Not FIPS-validated; the NIST test vectors prove correctness against the standard's reference values, not certification. Provided as is, no warranty, use at your own risk.
