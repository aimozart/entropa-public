# NIST FIPS-204 test vectors — provenance

`ml_dsa_65_acvp.json` contains **official NIST ACVP known-answer test vectors** for
**ML-DSA-65**, extracted **verbatim and unmodified** from NIST's own repository.

| | |
|---|---|
| **Source** | <https://github.com/usnistgov/ACVP-Server/tree/master/gen-val/json-files> |
| **Files** | `ML-DSA-keyGen-FIPS204/internalProjection.json`, `ML-DSA-sigGen-FIPS204/internalProjection.json` |
| **Retrieved** | 2026-08-12 |
| **Parameter set** | ML-DSA-65 (NIST security category 3 — the set Entropa uses) |

## What's committed

A representative subset, so the test suite stays fast:

| Group | Vectors | ACVP group | Exercises |
|---|---|---|---|
| `keyGen` | 12 | tgId 2 | `ML-DSA.KeyGen_internal` — seed ξ → (pk, sk) |
| `sigGenInternalDeterministic` | 8 | tgId 10 | `ML-DSA.Sign_internal` — deterministic (rnd = 0³²) |
| `sigGenExternalPureDeterministic` | 8 | tgId 3 | `ML-DSA.Sign` — full path incl. context string |

## Running against the *complete* official set

```bash
./fetch.sh          # pulls NIST's full keyGen / sigGen / sigVer files
```

The committed subset is for convenience and CI speed — it is not a curated "vectors we
happen to pass." Every vector in the upstream ML-DSA-65 groups is expected to pass;
`fetch.sh` exists so that claim is independently checkable.

## What this proves (and what it doesn't)

**Proves:** Entropa's post-quantum signing produces byte-for-byte identical output to the
NIST reference for these official vectors — key generation, both signing interfaces, and
signature verification (including rejection of tampered signatures).

**Does not prove:** FIPS certification. Entropa is **not** FIPS-validated and makes no such
claim. Formal validation requires an accredited CMVP/CAVP lab. This is a *correctness*
artifact: built to the standard and demonstrably matching it, not certified against it.
