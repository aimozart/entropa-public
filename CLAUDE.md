# CLAUDE.md — entropa-microservices

> Static rules for this repo, loaded every session. Deep working state lives in
> `SESSION_STATE.md` (overwritten/appended each session) — this file is only
> for rules that must hold across every future session regardless of what
> happened last time.

## Identity

- **Java 21 / Spring Boot / Spring Cloud microservices** — a rebuild of the
  original Entropa (post-quantum audit-trail log) concept, pivoted from Rust
  after a full decision process (see `SESSION_STATE.md` for the history).
- **This is a portfolio/skills-demonstration project, not a commercial
  product.** No live customers, no real payment processing tied to this repo.
  The old Rust build's live GCP infrastructure and Stripe live-mode
  configuration have been decommissioned separately (see `SESSION_STATE.md`).
- Public home: `github.com/aimozart/entropa-public` (main branch = this
  codebase; the original Rust build is preserved unchanged on the
  `rust-archive` branch, never deleted).
- Local dev: 6 Spring Boot services + Keycloak + Kafka + Postgres + full
  observability stack (Prometheus/Loki/Tempo/Grafana), all via
  `docker compose up --build`.

## Laws (hard constraints — never break)

- **L1 — No password-shaped literal ever goes into a tracked file, including
  "dev only" placeholders.** This isn't about real risk (a `_dev_only`
  suffixed local Postgres password protects nothing real) — it's that a
  human reviewer skimming a public portfolio repo will read any
  password-shaped string as a red flag regardless of whether it's technically
  safe, and for a job-search artifact, optics on this specific point matter
  as much as substance. All local secrets live in `.env` (gitignored, never
  committed), sourced from a documented `.env.example` with placeholder
  names like `change-me-locally`, never real-looking values. *(Real incident,
  2026-09-10: shipped `entropa_dev_only`/`admin_dev_only` in a first push,
  had to rewrite public history to remove it. Don't repeat this — get it
  right before the first push, not after being told.)*
- **L2 — Secrets never flow through Spring Cloud Config as `${PLACEHOLDER}`
  values, even ones meant to be resolved client-side.** Route secrets
  directly as container env vars matching Spring's own relaxed-binding
  property names (e.g. `SPRING_DATASOURCE_PASSWORD`), which take priority
  over remote config automatically. `${VAR}` placeholders inside a value
  served by Config Server are not reliably re-resolved against the local
  environment for framework-autoconfigured properties like
  `spring.datasource.password` — confirmed as a real bug, not a hypothetical,
  2026-09-10 (see `SESSION_STATE.md`).
- **L3 — Every claim about "it works" needs a real command's output as
  proof**, not just "the container is up." A container in `Up` status can
  still be crash-looping on the next request, stuck waiting on Eureka
  registration, or failing silently. The standing verification sequence for
  any change: rebuild → bring up → check Eureka registration → real curl
  through the gateway (both unauthenticated-should-401 and
  authenticated-should-200 cases) → check receipt/read-back, not just
  submission.
- **L4 — Docker networking: never hardcode `localhost` as another service's
  address.** Every cross-service URL (Config Server, Eureka, Keycloak,
  Kafka, Postgres) must come from an environment variable with a
  `localhost`-defaulting fallback only for genuinely-standalone local runs
  outside Docker. `spring.config.import: optional:configserver:http://localhost:8888`
  hardcoded directly in the import string overrides
  `spring.cloud.config.uri` entirely and silently breaks in Docker — this
  bit us for real on 2026-09-10; the fix is `import: "optional:configserver:"`
  (quoted, no URL) + a separate `spring.cloud.config.uri` property that
  environment variables can actually override.
- **L5 — GKE/Kubernetes never scales to zero for this architecture, and
  that's not a bug to fix.** JVM/Spring Boot cold starts are too slow for
  request-triggered scaling; Kafka/Postgres/Keycloak are stateful, always-on
  by nature. Real baseline cost accrues for as long as a cluster is up.
  Standing discipline: spin up when actively developing/demoing, tear down
  (`helm uninstall` + delete the cluster) when not — same as the AWS/
  Terraform practice discipline elsewhere in this user's work. Don't leave a
  GKE cluster running unattended.
- **L6 — `transparency-service` stays single-writer (replicas: 1), by
  design, in every environment.** Never horizontally scale it or include it
  in an HPA — this mirrors the same single-writer-by-design tradeoff the
  original Rust system made deliberately, not an oversight to fix later.
- **L7 — Resume/GitHub copy about Java experience must stay precise and
  honest.** Real basis: JVM ecosystem work via a Databricks-certified Apache
  Spark (Scala) background, plus a completed "Apache Spark for Java
  Developers" course, described as intermittent/exploratory over several
  years — never framed as continuous paid professional Java employment,
  which didn't happen. This project + the accompanying Spring Boot
  microservices course are the deliberate solidification point. See
  `SESSION_STATE.md` for the exact language already agreed on.
- **L8 — Real security, even in local/mock/demo contexts, by default —
  never something to be asked for.** OAuth2/JWT auth on every real endpoint,
  actuator/health endpoints the only exception. This was already built
  (Keycloak + Spring Security Resource Server on the gateway) before this
  law was written down — the law exists so the *next* addition (a new
  service, a new route) gets the same treatment automatically, not as an
  afterthought.

## Key references

`SESSION_STATE.md` — cockpit, read first every session, current build status
and what's still open (GKE deployment, Grafana dashboards, demo-traffic
generator for the test-mode-Stripe showcase idea).
