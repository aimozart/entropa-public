# entropa-node

The consensus engine behind [Entropa](https://entropa.space) — a post-quantum,
tamper-evident audit-trail network for AI agents.

- **[`Mempool`]** — pending transactions awaiting inclusion in a block.
- **[`select_proposer`]** — **Proof of Entropy** proposer selection, seeded by
  [drand](https://drand.love)'s public randomness beacon (via `entropa-core::beacon`) instead
  of mining or staking. No single party — including us — can predict or influence who
  proposes the next block.
- **[`Node`]** — produces and validates blocks on top of `entropa-core::Chain`.

Deliberately **transport-free**: networking (libp2p gossip, peer discovery) is meant to wrap
this crate, so the consensus core itself stays fully testable in isolation.

## Used by

`entropa-api` runs this as the consensus layer behind the live network:
[entropa.space](https://entropa.space) · [explorer](https://scryon.entropa.space) ·
[live Probe feed](https://scryon.entropa.space/flow).

## License

MIT. Source: [github.com/aimozart/entropa-public](https://github.com/aimozart/entropa-public)
