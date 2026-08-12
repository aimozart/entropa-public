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

Both auto-close after 30 minutes of the condition clearing, so a transient blip doesn't leave a stale
"firing" alert sitting around.

## Why not Prometheus/Grafana

Considered and deliberately skipped, for the same reason [Pulumi was dropped](ARCHITECTURE.md#build--infrastructure):
one Cloud Run service doesn't justify running a separate metrics stack. Cloud Run already ships request count,
latency, error rate, and container CPU/memory to Cloud Monitoring for free with zero setup, and the two alert
policies above sit directly on top of that plus one log-based metric — no scraping, no exporters, no
dashboards to maintain. Real observability infrastructure earns its keep once there's enough surface area to
need correlating dashboards across services; a single-service demo isn't that yet.

## Watching it yourself

```
gcloud run services logs read entropa-scryon --region=us-central1 --project=entropa-testnet
```

Every brain decision, every persistence save/fail, and every request is in there — the same logs the alert
policies watch.
