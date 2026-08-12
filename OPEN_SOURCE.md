# Entropa — Open-Core License Split

**Rule of thumb: the protocol and crypto are open; the AI that drives it is proprietary.**

| Crate / asset | License | Why |
|---|---|---|
| `entropa-core` | 🔓 MIT (see `LICENSE-MIT`) | PQC identities + chain primitives — public good, builds credibility |
| `entropa-node` | 🔓 MIT | Mempool + **Proof of Entropy** consensus engine, the beacon spec |
| Scryon (`crates/api/scryon/`) | 🔓 MIT | Explorer frontend — transparency + marketing |
| `entropa-agents` | 🔒 Proprietary | Probe decision logic / prompts — the actual secret sauce |
| Hosted API / managed service | 🔒 Proprietary | The commercial offering |

Each open crate's `Cargo.toml` `license` field should read `"MIT"`. `entropa-agents/Cargo.toml` should read
`"UNLICENSED"` (proprietary, all rights reserved) before the public GitHub push.
