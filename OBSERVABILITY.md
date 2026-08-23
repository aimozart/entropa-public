# Entropa — Observability & Self-Healing

*How the live network is watched, and what it does on its own before a human ever gets paged.*

Entropa runs as a single Cloud Run service with an AI Probe making real decisions on a timer. Small footprint,
but it's live, unattended, and calling an external model — worth being deliberate about what happens when
something breaks.

## What self-heals without anyone looking

Most failure modes here are already handled by the code and the platform, not by a human reacting to an alert:

- **A failed Gemini call doesn't crash anything.** `Proposer::craft` returns a `Result`; on error the round is
  logged and skipped — the Probe just tries again on its next heartbeat (every 120s). No retry loop needed to
  write, because the heartbeat *is* the retry loop.
- **A failed persistence write doesn't lose data.** `persistence::save_block` is best-effort: a failed
  Firestore write logs and returns; only that one block's save is missed, not the whole chain's (each block is
  its own document). The in-memory chain is always the source of truth for the running process regardless of
  whether the last save landed.
- **A crashed container restarts itself.** This is Cloud Run's job, not ours — if the process dies, Cloud Run
  brings up a fresh instance, and the chain resumes from Firestore instead of resetting to height zero
  (`crates/api/src/persistence.rs`).
- **A bad deploy doesn't take down the live site.** Cloud Run only routes traffic to a revision once it passes
  its own health check; a build that doesn't boot never gets traffic.

None of this is exotic — it's just "fail soft, retry on the next natural cycle, let the platform handle process
death" — but it means the alerting below exists for the failures that *don't* self-heal, not for routine
hiccups.

## What actually pages a human

Two Cloud Monitoring alert policies, both notifying by email, both created directly against the GCP APIs (no
extra monitoring stack — see [why not Prometheus/Grafana](#why-not-prometheusgrafana) below):

| Alert | Trigger | What it catches |
|---|---|---|
| **AI Probe decision failures** | 3+ failed Gemini calls in a 15-minute window (log-based metric over `brain: decide failed` in Cloud Run's own logs) | Auth/IAM breakage, Vertex AI quota exhaustion, a model response format change the parser can't handle — anything that will keep failing every cycle until someone looks |
| **Cloud Run 5xx errors** | 2+ server errors in a 5-minute window | The service itself misbehaving, independent of the AI path |
| **Heartbeat check failed** (added 2026-08-12) | A 15-minute uptime check hits `GET /api/health`, expects `"status":"ok"` in the body; alerts on the first failure | Anything that takes the whole service down (crash loop, bad deploy, out-of-memory instance stuck restarting) — fires faster and more broadly than the other two, which both need a specific *kind* of failure to accumulate first. Purpose-built so an outage doesn't sit unnoticed burning resources; see `entropa-health-heartbeat-a8YrpQoz4ug` uptime check config. |

Both auto-close after 30 minutes of the condition clearing, so a transient blip doesn't leave a stale
"firing" alert sitting around.

## Why not Prometheus/Grafana

Considered and deliberately skipped, for the same reason [Pulumi was dropped](ARCHITECTURE.md#build--infrastructure):
one Cloud Run service doesn't justify running a separate metrics stack. Cloud Run already ships request count,
latency, error rate, and container CPU/memory to Cloud Monitoring for free with zero setup, and the two alert
policies above sit directly on top of that plus one log-based metric — no scraping, no exporters, no
dashboards to maintain. Real observability infrastructure earns its keep once there's enough surface area to
need correlating dashboards across services; a single-service demo isn't that yet.

## Known failure modes (prevent recurrence, don't just fix and forget)

Every incident below left behind a permanent guardrail — a test, a structural fix, or a standing rule — not
just a one-off patch. Check this table before assuming a new symptom is novel; the full narrative for each is
in `MILESTONES.md` (search the date).

| Date | Failure | Root cause | Guardrail that now prevents it |
|---|---|---|---|
| 2026-08-12 | Chain corrupted at two points after ~12 rapid redeploys | Cloud Run runs old+new revisions concurrently during every deploy transition; two in-memory `Node`s both wrote blocks to the same Firestore collection | Firestore-backed leader lease (`crates/api/src/leader.rs`, 20s TTL) — only the lease holder writes. **Standing rule**: after every deploy, `gcloud run revisions list` and delete anything that isn't current — Cloud Run doesn't reliably reclaim an old revision on its own when it's still actively writing. **Standing rule**: no design partner onboarded without a 3-day soak test first (`SESSION_STATE.md` § soak criteria) — this class of bug only shows up across *repeated* live redeploys, never in a single build/test/verify pass. |
| 2026-08-12 | `GeminiBrain` failed ~100% of heartbeats (`invalid type: map, expected a string`) | Gemini occasionally returns `payload` as a nested JSON object instead of the plain string the prompt asks for; `Decision.payload: String` had no tolerance for that | `flexible_string` deserializer (`crates/agents/src/brain.rs`) accepts either shape. Regression test `decision_tolerates_object_payload`. **General principle**: never trust an LLM's output to match its requested schema exactly — deserialize defensively, don't just format a prompt and hope. |
| 2026-08-12 | `/api/chain` 500ing for every visitor, silently breaking the explorer + `/flow` | The endpoint dumped the entire unpaginated chain; once past a few thousand blocks it exceeded Cloud Run's response-size limit | `?limit=`/`?offset=` pagination, default most recent 1000 blocks (`crates/api/src/lib.rs`). **General principle**: any endpoint returning a collection that grows without bound over the service's lifetime needs pagination from day one, not once it breaks — audited the rest of `lib.rs` for the same pattern (mempool length is returned as a count, not the collection; every other route reads a single block), nothing else currently at risk. |
| 2026-08-12 | Explorer/`/flow` visibly hung ("checking…"/"loading chain…") right after the fix above shipped | The pagination fix's default (1000 blocks) was still ~10.8MB of JSON — 2.4s+ server-side alone, before the browser even parsed/rendered it, on a page that live-refreshes every few seconds | Both pages now request `?limit=50` explicitly (`explorer.html`, `flow.html`) — confirmed 542KB / ~0.3s. **General principle**: "paginated" isn't the same as "fast enough for the actual default view" — check what the default request really costs a real client, not just whether it's under a hard server limit. |
| 2026-08-12 | GCP budget alerts were blending Entropa's spend with 4 unrelated projects on the same billing account | The two pre-existing budgets (`Entropa ~/day`, `$20 card guard`) had no `projects` filter, so they tracked the whole billing account | Added `Entropa (entropa-testnet only)`, a $60/month budget filtered to `projects/1032137727494`. **General principle**: a shared billing account needs at least one project-scoped budget per project that actually matters, or a "cost is fine"/"cost spiked" reading could be about a completely different project. |
| 2026-08-12 → 2026-08-23 | The public website and the validator's consensus/block-production logic were **the same Cloud Run binary** for the entire life of the project — every deploy, including a plain landing-page copy change, rebuilt and redeployed the whole crypto/consensus stack | Marketing routes (landing page, Scryon explorer, `/flow`, `/hire`, `/resume`, `/glossary`) were bundled into `crates/api/src/lib.rs` alongside the actual product API and consensus code, routed by HTTP `Host` header, since the first live deploy. Documented in this project's own architecture diagram the whole time — never treated as something to actually fix. When N=3 quorum consensus shipped (2026-08-22/23), this tripled: two more validator services, each also a full copy of the public website. **Not caught by this project's own review process** — not the 2026-08-22 production-readiness review, not repeated architecture read-throughs, not the session that designed the quorum rewrite itself. Caught by aimozart, the project's own founder — the assistant working on this project said, in passing, that "validator 1 serves entropa.space," and aimozart's reaction to that one plain sentence is what surfaced the whole problem. Not caught by the assistant itself flagging it, and not caught by any internal review process. Slow Cloud Build times had been attributed, out loud, to Rust's build speed — that attribution was wrong; the real cause was three unrelated services fused into one deployable unit. | Full duty-based separation: the website now deploys to Firebase Hosting with zero blockchain/crypto dependencies; each validator's binary carries only consensus + product API routes. See `ARCHITECTURE.md` § Repo split for the current, corrected topology. **Standing lesson**: "documented" is not "fixed" — a known architectural shortcut that nobody revisits on a schedule will eventually get worse, not better, especially when new work (like standing up more validators) multiplies it instead of questioning it first. |
| 2026-08-23 | `GET /block/:index` returned 404 for essentially every live block, on every visitor who clicked a real block-detail link | `Chain::bound_to_window` (shipped 2026-08-22) trims the in-memory chain to the most recent 20,000 blocks, whose Vec positions are *relative* to a floor index — but the block-detail page still looked blocks up by their raw absolute chain index, an out-of-bounds miss for nearly everything. Four other routes got the relative-index + storage-fallback fix when bounded memory shipped; this one was missed. Found by a real person clicking a real link, not by any test or review. | Regression test built first (evict a small chain to a 10-block window, request a block inside the window but not at that Vec position, confirm it fails for exactly that reason), then the fix: translate to a relative offset, fall back to durable storage below the floor, same pattern as the other four routes. Deployed to all validators same day, confirmed live. **Separately confirmed, with real evidence, not assumed**: this bug was read-only and never touched the write path — zero chain-rollback events and continuous, monotonic height growth were verified across the entire incident window, through this fix, a DNS cutover, and a load-balancer build happening in the same session. The bug was real and visible; the chain's actual integrity was never at risk. |

**Why this table exists**: alerts had been firing on both of the bottom two bugs for a while before they were
actually investigated (see the mailbox-routing note above) — the alert *worked*, but nothing forced a look at
root cause vs. just clearing the noise. This table is the forcing function for the next incident: land a row
here, not just a fix.

## Working with automated review (Paxel), 2026-08-23

Same night as the incidents above, this project went through a real back-and-forth review cycle
with [Paxel](https://paxel.ycombinator.com), YC's builder-report tool. Documenting both sides
honestly: what its first pass got wrong, what got fixed for real in response, and a confirmed bug
in its own analysis — transparency cuts both ways, on our code and on the tools reviewing it.

**Confirmed remediation, built and shipped the same session:**
- **API/Firebase contract tests** (`crates/api/tests/firebase_contract.rs`) — the boundary between
  the Rust validator API and the newly-separated Firebase-hosted website had zero coverage; now
  pins the JSON shape of `/api/receipt/:id` and `/block/:index`, plus CORS behavior, via `insta`
  snapshots with random crypto fields redacted.
- **Deploy/rollback/migration script harness** (`scripts/lib/gcloud-runner.sh` + 3 `.bats` suites,
  25 tests) — real ad hoc `gcloud` invocations extracted into tested scripts with required-env
  validation, dry-run mode, and revision/service-existence safety checks against a fake `gcloud`.
- **Leader-lease lifecycle tests** — the actual acquire/renew/expiry/stale-owner-rejection logic
  behind this project's single-writer safety guarantee had zero coverage (only URL construction
  was tested); now has real trait-based seams (`LeaseStore`/`Clock`) and 8 integration tests
  exercising the exact production code path via a fake store/clock.

Two things were *not* built even under an explicit push to close every named item, and that's
recorded here deliberately: a CORS test asserting unknown origins are rejected (the real policy is
permissive to all origins — a test asserting rejection would encode a false claim about the
system) and an `IMAGE`-based deploy path (this project deploys via `--source=.`, not a pre-built
image tag — adding an unused required variable to match a wrong assumption would be dead weight).

**A confirmed bug in Paxel's own test-file discovery, not just our usual blind-spot list:**
after building all of the above, a rerun still reported "7 test files" — identical to a report run
*before* 6 brand-new test files were added. Commit/line-count totals *did* change between the two
runs, ruling out a full stale cache; the bug is specifically in test-file discovery. Direct grep for
real test code (`#[test]`, `#[tokio::test]`, bats shebangs) found 34 files, not 7 — most likely
because the scanner excludes standard Rust integration-test layout (`crates/*/tests/*.rs`) and
shell-based operational tests (`scripts/tests/*.bats`) entirely. Paxel's own follow-up confirmed
this reading. Full technical detail in `ANALYSIS_NOTES.md § 5`.

**The one finding from that review kept without caveat**: a dedicated simplification pass. This
session was almost entirely additive (new tests, new scripts, new trait seams) — the honest next
step isn't more safety net, it's coming back and removing what's now obsolete.

## Watching it yourself

```
gcloud run services logs read entropa-scryon --region=us-central1 --project=entropa-testnet
```

Every brain decision, every persistence save/fail, and every request is in there — the same logs the alert
policies watch.
