# Entropa

### Verify, don't trust. Free, open AI security and post-quantum audit trails, for public safety.

**Our mission.** AI systems are being trusted with decisions about people's money, health, work and safety, and almost no one outside the vendor can check what they did or why. We think that is dangerous. Entropa exists to make AI and LLM security, and tamper-evident, post-quantum-signed audit trails, free and open for everyone: tools that limit what an AI can read and send, and a signed record of every decision that anyone can verify for themselves, today and after quantum computers. We publish our results, including every failure, because public safety needs proof, not promises.

## What it is
**Entropa Gate** is a free app for Linux (x86_64) that sits between your AI and the people and systems around it:
1. **Limits what the AI can read**: each person or agent gets only the access their role needs (access tiers and short-lived signed tokens).
2. **Checks what leaves**: secrets, personal data and your do-not-send terms are blocked or removed before anything goes to a model or tool (Egress Guard).
3. **Needs a person for big actions**: high-impact actions wait for a signed human approval (a default-deny Cedar policy gate).
4. **Keeps proof**: every decision becomes a hash-only, post-quantum-signed (ML-DSA-65), tamper-evident record that you or an auditor can check offline, without trusting us.

You run it on your own machine with your own keys and your own AI model. We never see your data.

## Where it stands (2026-11-17), honestly
- **Built and running for our own systems:** the signed, witnessed ledger with an offline verifier; the policy gate; the public explorers; a control-by-control self-assessment of our own estate against DORA, PCI DSS, NIST SP 800-53, the EU AI Act and HIPAA, with the gaps listed first.
- **Built and measured, not finished:** Egress Guard still leaks secrets in our own sealed tests (44.9% of 1,042 new cases in the latest round: we say so); access tiers and tokens (one real hole found and fixed); standards retrieval (two of five bars failed); the "try the gate" server and its container (442 of 448 hostile scenarios passed, one genuine minor gap found).
- **In progress:** the single installable package, a self-hosted explorer, a browser-side verifier.
- **Not done:** an independent review, any pilot, any certification. Nothing here is a claim that it is safe to rely on.

Every number, including the failures, is on the results page: **https://results.entropa.space**. Test sets are written by someone other than the author, run once and never reused. The sealed sets themselves are never published, or the results would mean nothing.

## Read and verify
- Site: https://entropa.space
- Documentation: https://docs.entropa.space (install, threat model, how to verify a record yourself, known limits)
- Results of record (failures first): https://results.entropa.space
- Explorer (check signatures and proofs in your browser): https://scryon.entropa.space
- Security policies: https://github.com/aimozart/sec-procedures

## Why Rust, and where other languages are used

Rust is the key to this project. Everything that makes a security decision or signs a record (the ledger, signing and verification, the policy gate, the Egress Guard, access tokens, standards retrieval, the servers) is written in Rust, for four reasons: memory safety by construction (no buffer overflows or use-after-free in safe code), one small static binary that we rebuild reproducibly so you can check it against the source, a strict compiler that pushes whole classes of mistakes to compile time and makes "fail closed" the natural way to write a guard, and predictable speed with no garbage-collector pauses. Other languages play supporting roles only: Python for test runners and analysis scripts, shell for build and packaging, a little JavaScript for the in-browser verifier. By GitHub's count about 84 percent of the Entropa code (entropa-gate) is Rust. Safe Rust does not remove logic errors, bad design or weak cryptography choices, and this is a research prototype with no independent review yet; our own sealed tests still find real bugs, and we publish them.

## Licence and contributing
**Apache-2.0** for the code and **CC-BY-4.0** for the documentation: a full giveaway. Use it, change it, sell it; keep the copyright notice; provided as is, no warranty, use at your own risk. The crates `entropa-core` and `entropa-node` were published earlier under MIT and stay MIT (`LICENSE-MIT`). The name "Entropa" is not licensed: forks must rename. Contributions (DCO sign-off, no CLA) open after the first full release; forks are welcome now. See `OPEN_SOURCE.md`.

## What this repository holds
The architecture, the scope, the record and verification specification, test vectors and the first open crates (`crates/`, with the earlier history on the `rust-archive` branch). The application source is published at [entropa-gate](https://github.com/aimozart/entropa-gate). Never keys, credentials or real data; all data is synthetic.

| Read | For |
|---|---|
| [OPEN_SOURCE.md](OPEN_SOURCE.md) | The licence and open-source model |
| [ARCHITECTURE.md](ARCHITECTURE.md) | How the system fits together |
| [docs/SCOPE.md](docs/SCOPE.md) | Scope |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability |
| [TERMS_OF_SERVICE.md](TERMS_OF_SERVICE.md), [PRIVACY_POLICY.md](PRIVACY_POLICY.md) | Terms of use (no warranty) and privacy |

## Download an earlier node (Linux)
An earlier node and checkpoint-witness build is published as a free binary: [node-v0.2.0](https://github.com/aimozart/entropa-public/releases/tag/node-v0.2.0) (x86_64 Linux, glibc 2.34 or newer, with a `SHA256SUMS` file). Install commands and what was tested (Ubuntu 24.04.5 and Rocky Linux 10.2, run on 2026-11-05): https://docs.entropa.space/install-linux . Linux only. The full source is being released in stages under Apache-2.0.

## Method
Tests first (red, then green); fuzzing and mutation testing on parsing and verification; secret scanning (gitleaks) and a preflight before every push; signed, data-minimal records; sealed tests written by someone other than the author; failed bars reported as failed. Wording: evidence is "mapped to" standards and results are "tested against" our own sets, never "compliant", "certified" or "secure".

## Services
Custom work, on request after a free fact-finding meeting: setup, architecture and infrastructure review, attack tests built from your own data and use case, and training. Contact form: https://entropa.space/contact
