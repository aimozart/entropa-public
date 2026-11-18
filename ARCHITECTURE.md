# Architecture

```
 producers                      Entropa node (Rust, serverless)                    verifiers
 ---------                      -------------------------------                    ---------
 AI assistants  --decision-->   validate (closed vocabularies,       -->  append-only Merkle log  -->  explorers (read only)
 pipelines      --evidence-->   identifier-shape refusal, hash-only)      signed checkpoints          in-browser verifier
 CI / deploys   --receipt--->   sign (ML-DSA-65 via cloud KMS)            consistency proofs          independent witnesses
                                                                                   ^                    (co-sign checkpoints)
                                                                                   |
                                                                       witnesses on other clouds
```

## Pieces
- **Records.** Event types for AI decisions (`platinum.decision`) and for healthcare data-engineering evidence (pipeline run, control check, access, change, policy). Fields are closed vocabularies plus hashes of inputs and outputs. Identifier-shaped text is refused. Plain-English descriptions are generated from codes and counts by fixed templates.
- **Log.** BLAKE3 Merkle tree (leaf and node prefixes per RFC 6962/9162 style). The checkpoint (root, size, time) is signed *before* any record is written.
- **Signing.** ML-DSA-65. Managed key in Google Cloud KMS for the cloud node; AWS KMS signer also implemented; a local key file for local nodes. Signer is pluggable.
- **Node.** Rust, runs on Cloud Run (scale to zero). HTTP: decision, evidence, verify, export, public key. Local CLI with the same operations.
- **Witness.** `entropa-witnessd` co-signs a checkpoint only if the log signature is valid and a consistency proof links it to the last size it saw; forks and rollbacks are refused and logged. Runs on a different cloud from the log (Azure now, AWS planned). Statements travel in the export bundle.
- **Verifier.** A 32 KB self-contained JavaScript bundle (ML-DSA-65 and BLAKE3, no CDN) checks every event signature, leaf hash, Merkle root, checkpoint signature and witness statement using only the public key. Tamper tests: flipped bytes are detected.
- **Explorers and docs.** Static sites on Firebase Hosting reading exported bundles.
- **Guardrails around the agents that build it.** An MCP server (read-only tools), a command hook that denies destructive actions, a Cedar policy gate and an approvals ledger. Enforcement is in code, not in prompts.

## Limits (stated plainly)
The log proves recorded events are unaltered; it cannot prove nothing was omitted. Witnesses reduce but do not remove trust in the operator. Synthetic data only; not a HIPAA-compliant or certified system.
