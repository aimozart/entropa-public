# SESSION_STATE — entropa-microservices

## The pivot, in one paragraph

Started as a Terraform Associate (004) study site build, then AWS DevOps Pro
(CloudFormation), then a full career-direction pivot: dropping AWS/Terraform
certification pursuit entirely in favor of GCP + Java 21 + Spring Boot +
Spring Cloud microservices + Kubernetes/Helm, aimed at the same Dec 1, 2026
hard hire deadline. Entropa (the post-quantum audit-trail log concept,
previously a real, live Rust production system) was rebuilt from scratch in
Java as the flagship portfolio piece proving this new skillset — not a
literal port, a genuine architectural rebuild into real microservices
(Config Server, Eureka, Gateway, event-driven Kafka, OAuth2 security).

The original Rust codebase and its real production history are **not**
deleted — preserved in full on the `rust-archive` branch of
`github.com/aimozart/entropa-public`. Real live GCP infrastructure for the
Rust build (Cloud Run services) predates this repo and is a separate
decommissioning question from this codebase.

## Real business/account actions taken this session (2026-09-10/11)

- **Stripe live mode decommissioned for real**, via the GCP-stored
  `entropa-stripe-live-secret-key` secret and direct Stripe API calls (not
  just documentation): the `Entropa Attestation` product and its price set
  to `active: false`, the entropa live webhook endpoint disabled. Zero real
  subscriptions existed at the time (confirmed via the API before touching
  anything). Test-mode Stripe deliberately left fully active — see "Open
  idea" below for why.
- **A second, unrelated live product found on the same Stripe account**
  ("Ruby on Rails Mastery Course") — its product was already inactive; its
  webhook endpoint (still enabled, pointing at `rubyrailsmasterycourse.guru`)
  was deleted after explicit direct confirmation from the Captain that this
  was intentional and the site behind it no longer exists.
- **entropa-public repo replaced**: old Rust content removed from `main`,
  full Rust history archived unchanged on `rust-archive` (pushed to both
  GitHub and the private Gitea backup mirror before anything was removed
  from main). New Java codebase pushed to `main`, CI (Gradle build+test +
  gitleaks) added and confirmed green on GitHub Actions.
- **Public history rewritten once already** (2026-09-10) to remove commits
  that had shipped password-shaped placeholder strings
  (`entropa_dev_only`, `admin_dev_only`) — even though gitleaks confirmed
  these were never real secrets, they were bad optics for a job-search
  portfolio repo and got force-pushed out. See `CLAUDE.md` L1 — don't
  reintroduce this class of mistake.

## What's actually built and verified (not just "should work")

Real end-to-end verification performed via direct `curl`/`docker` commands
this session, not assumed:

- All 6 services (`eureka-server`, `config-server`, `api-gateway`,
  `ingest-service`, `transparency-service`, `notification-service`) build
  clean, pass their tests (10 tests: SHA-256 hashing correctness, Kafka
  publish behavior, tamper-evident hash-chain linking logic), and run
  together via `docker compose up --build` alongside Kafka, Postgres,
  Keycloak, and the full observability stack.
- **Real hash-chain tamper-evidence logic**, not a stub: each block commits
  to the previous block's hash + its own leaf index + its content hash.
  `ChainHasher` is a pure, independently-tested function.
- **Real OAuth2/JWT security**: Keycloak as the identity provider (realm
  `entropa`, a service-account client for machine-to-machine token
  issuance), Spring Security Resource Server on the gateway validating
  every request except `/actuator/**` and `/fallback/**`. Verified:
  unauthenticated request → `401`; valid JWT → `200` with correct data.
- **Real service discovery under an actual failure**: after eureka-server
  was restarted mid-session, all 5 client services correctly re-registered
  within seconds with zero manual intervention — proven, not assumed.
- **Real observability**: all 6 services expose live Prometheus metrics
  (confirmed via Prometheus's own `/api/v1/targets`, all showing `up`),
  Grafana provisioned with Prometheus/Loki/Tempo datasources.
- **Resilience4j actually wired into the gateway's routes** (circuit
  breaker + fallback URI on both the ingest and transparency routes,
  time-limiter configured), not just present as an unused dependency.
- **Real bugs found and fixed during verification, not hypothetical**:
  hardcoded `localhost` breaking Docker networking for Spring Cloud Config,
  a YAML quoting error, missing actuator metrics exposure on two services,
  and a Spring Cloud Config placeholder-resolution failure for
  `spring.datasource.password` specifically (fixed by routing that one
  property as a direct env var instead of through remote config — see
  `CLAUDE.md` L2).

## Still open

1. **Live GKE deployment** — Helm chart exists (`helm/entropa/`, templated
   over a `services` map, `transparency-service` correctly excluded from
   autoscaling), but nothing has actually been deployed to a real GKE
   cluster yet. This is the actual next milestone — a hiring manager can't
   reach `localhost`.
2. **Grafana dashboards** — datasources are provisioned, no actual dashboard
   JSON has been built yet.
3. **Demo-traffic / mock-showcase feature (real, good idea, not yet
   scoped)**: keep test-mode Stripe active, build a flow where a test-mode
   checkout triggers a generator producing simulated "AI attestation"
   traffic through the real architecture, landing on a public dashboard so
   a hiring manager can see the system actually working without needing
   real customer data. Needs real design (where does the generator run,
   what's the dashboard's own auth story) before building — explicitly
   deferred until the core rebuild + GKE deployment are settled.
4. **Automatic teardown tooling for the eventual GKE cluster** — no real
   cost risk yet since everything is still local Docker, but once a real
   cluster exists, build the automation (a scheduled job or manual
   reminder) rather than relying on remembering to `helm uninstall`
   manually every session.
5. **Resume and GitHub profile copy** — language already agreed on (see
   `CLAUDE.md` L7), not yet written into the actual resume file or GitHub
   profile README.

## Separate, parallel track (not this repo)

`/home/sbaker/gcp_microservices/java_files/spring-microservices-course/` —
a from-scratch study site (same pattern as the completed Terraform Associate
site) for the "Master Microservices with SpringBoot, Docker, Kubernetes"
Udemy course, built from real, paid transcripts. Scaffolded, content-rewrite
work in progress as of this session's end. Distinct from entropa-microservices:
this is the Captain's own hands-on, self-built practice work (explicitly "no
AI" for the actual coding, per the Captain's own instruction) with
progressively-building capstones per section culminating in one integrated
final capstone — not something this assistant builds end-to-end the way
entropa-microservices is.
