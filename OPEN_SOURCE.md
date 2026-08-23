# Entropa — Open-Core License Split

**Rule of thumb: the protocol and crypto are open; the AI that drives it is proprietary.**

| Crate / asset | License | Why |
|---|---|---|
| `entropa-core` | 🔓 MIT (see `LICENSE-MIT`) | PQC identities + chain primitives — public good, builds credibility |
| `entropa-node` | 🔓 MIT | Mempool + **Proof of Entropy** proposer-selection primitive, the beacon spec |
| Scryon (`crates/api/scryon/`) | 🔓 MIT | Explorer frontend — transparency + marketing |
| `entropa-agents` | 🔒 Proprietary | Probe decision logic / prompts — the actual secret sauce |
| **N=3 quorum consensus layer** | 🔒 Proprietary | Real production multi-validator agreement + VRF-based leaderless fallback election, built on top of the open Proof of Entropy primitive — not in this repo, see below |
| Hosted API / managed service | 🔒 Proprietary | The commercial offering |

**On the quorum layer specifically**: the open `entropa-node` crate above gives you a real,
correct *proposer-selection* primitive (`select_proposer`, seeded by drand) — that part has
always been open and stays open. What's proprietary is the layer built on top of it that
actually runs the live network in production: real N=3 multi-party cryptographic quorum
(majority of independent validators must co-sign a block before it's final), plus a
VRF-based fallback election so the network keeps producing blocks even if a validator goes
down. That engineering is ours, built after the open-core split, and it stays private —
same "protocol is open, the intelligence layered on top is proprietary" rule this document
already applies to `entropa-agents`.

Each open crate's `Cargo.toml` `license` field should read `"MIT"`. `entropa-agents/Cargo.toml` should read
`"UNLICENSED"` (proprietary, all rights reserved) before the public GitHub push.

## Doesn't `git clone` just give the product away?

No — the moat survives the open source. Cloning `entropa-public` gets the plumbing, not the product:

1. **The AI Probe brain (`entropa-agents`) never leaves this private repo.** The consensus engine tells you
   *how* a proposer is picked; it says nothing about *what a Probe decides to record*. Without it, a clone can
   only run a dumb demo — a stand-in that submits a canned transaction every 3 seconds, exactly like the public
   repo's `demo_decision()` does today. **The intelligence is the product, and it's not in the repo.**
2. **A clone starts from genesis, alone.** No other validators, no shared history, nobody to cross-verify with.
   The actual value is being part of *our* live network with established trust, not the ability to spin up an
   empty toy chain. Same reason "you can clone Bitcoin's code" never meant "you get Bitcoin's network effect."
3. **There's no separate client SDK or deployable agent to clone in the first place.** A partner integrates
   with one HTTP call (`POST /api/tx`, a bearer key) — the entire "surface" a clone gets is a thin JSON API
   over a chain with no AI behind it. Cloning the plumbing doesn't get you a product; it gets you an empty
   pipe.
4. **Hosting, uptime, support, billing, SLAs.** Even a team that could reimplement the AI layer mostly won't
   want to run their own PQC infra — that's exactly the "boring, keep you safe" work customers pay to not
   think about.

Giving the engine away is the **credibility engine**, not a leak: it's how the crypto gets proven real
(auditable, not "trust me"), and it's why an engineer or investor reading the code believes the rest can be
built. Nobody pays for a black box; they pay for a black box they can verify the foundations of. **We gave
away the recipe for flour, not the bakery, the ovens, or the head baker.**
