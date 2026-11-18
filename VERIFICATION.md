# How to independently verify an Entropa receipt

Every submission to Entropa returns a **receipt** you can verify yourself — without
trusting Entropa's servers, our API responses, or our own claims. This document shows
you exactly how, with real, tested code. Everything below was run against a live
production receipt before being written down here, not copied from memory.

## What's in a receipt

```json
{
  "index": 253616,
  "record": "6f6e626f617264696e...",
  "proof": [
    { "sibling": "5b7dfa39e77d1825f...", "side": "Left" },
    { "sibling": "f0d1b362530029f86...", "side": "Left" }
  ],
  "checkpoint": {
    "root": "44ebe65cd24a970215...",
    "size": 253617,
    "timestamp": 1787736540,
    "proposer_pubkey": "5a0d3a031902ed86...",
    "signature": "f98ac8cad8ac9f07..."
  }
}
```

- **`record`** — your original submission (hex-encoded). **If you attached an optional
  label**, `record` is `payload + 0x00 (a single NUL byte) + label`, not just the
  payload alone — split on the first `0x00` byte to recover each half. This is
  deliberate: it means a label is exactly as tamper-evident as the payload, folded
  into the same leaf hash, and can never be silently changed or removed later without
  invalidating the proof below.
- **`proof`** — the sibling hashes needed to recompute the log's root from your record
  alone, without needing every other record in the log.
- **`checkpoint`** — a signed statement of the entire log's state at the moment your
  proof was issued: its Merkle root, how many records it covered (`size`), when
  (`timestamp`), who signed it (`proposer_pubkey`), and the signature itself.

Verifying a receipt means checking two independent things:

1. **Your record really produces this root, given this proof** (a pure hashing check —
   no cryptographic keys involved).
2. **The checkpoint's signature is a genuine, unforged ML-DSA (FIPS-204) signature**
   over that root, by the pubkey it claims.

If both hold, your record is provably in the log, and the log's signer really
committed to it — independent of anything Entropa's own API tells you.

## Step 1 — verify the inclusion proof (pure hashing, any language)

The hashing scheme is domain-separated (same design as Certificate Transparency's
RFC 6962): a leaf is `hash(0x00 || record)`, and combining two nodes is
`hash(0x01 || left || right)`. Entropa uses BLAKE3.

```python
import blake3

def leaf_hash(data: bytes) -> bytes:
    return blake3.blake3(b"\x00" + data).digest()

def node_hash(left: bytes, right: bytes) -> bytes:
    return blake3.blake3(b"\x01" + left + right).digest()

def verify_inclusion(record_hex: str, proof: list, expected_root_hex: str) -> bool:
    current = leaf_hash(bytes.fromhex(record_hex))
    for step in proof:
        sibling = bytes.fromhex(step["sibling"])
        if step["side"] == "Left":
            current = node_hash(sibling, current)
        else:
            current = node_hash(current, sibling)
    return current.hex() == expected_root_hex
```

Run this against your own receipt's `record`, `proof`, and `checkpoint.root`. If it
returns `True`, your record is definitely part of the tree that root represents — full
stop, no trust required. This works identically whether or not you used a label —
the whole `record` bytes (payload and label together, if present) are what's hashed.
To recover your original payload and label separately afterward:

```python
def decode_record(record: bytes) -> tuple[bytes, bytes | None]:
    if b"\x00" in record:
        payload, label = record.split(b"\x00", 1)
        return payload, label
    return record, None
```

This is a real, working example — `pip install blake3`, drop
in your receipt's fields, and it runs.

## Step 2 — verify the checkpoint signature (ML-DSA / FIPS-204)

The exact bytes that get signed (the "checkpoint digest") are:

```
root (32 bytes) || size (8 bytes, big-endian u64) || timestamp (8 bytes, big-endian u64)
```

```python
import struct

def checkpoint_digest(root_hex: str, size: int, timestamp: int) -> bytes:
    return bytes.fromhex(root_hex) + struct.pack(">Q", size) + struct.pack(">Q", timestamp)
```

Then verify the signature with any FIPS-204-conformant ML-DSA-65 library. We tested
this guide with the pure-Python [`dilithium-py`](https://pypi.org/project/dilithium-py/)
implementation:

```python
from dilithium_py.ml_dsa import ML_DSA_65

digest = checkpoint_digest(checkpoint["root"], checkpoint["size"], checkpoint["timestamp"])
pubkey = bytes.fromhex(checkpoint["proposer_pubkey"])
signature = bytes.fromhex(checkpoint["signature"])

is_valid = ML_DSA_65.verify(pubkey, digest, signature)
```

Any other FIPS-204-compliant library (Rust's `ml-dsa` crate, `liboqs`, etc.) will give
the identical answer — this is a public NIST standard, not an Entropa-specific
algorithm.

## What "no trust required" actually means

Notice neither step above ever calls Entropa's API, imports Entropa's code, or trusts
anything Entropa's server told you beyond the raw receipt data itself. You could run
this verification a year from now, offline, after Entropa ceased to exist, and it would
still give you a correct answer about whether your record was genuinely in the log at
that checkpoint. That's the actual guarantee — not a slogan.

**One thing to trust exactly once, out of band**: the signer's public key
(`proposer_pubkey`) itself. Verifying a signature only proves *a* key signed it — you
still need to know that key genuinely belongs to Entropa. Confirm the pubkey you're
checking against matches the one published at [entropa.space](https://entropa.space),
obtained separately from any individual receipt.

## Full worked example

A complete, runnable script combining both steps, tested against a real production
receipt, is available on request — the two snippets above are the entire algorithm;
there is no additional hidden logic.
