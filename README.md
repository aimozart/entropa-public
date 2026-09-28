# Entropa Microservices

A post-quantum-adjacent audit-trail log — the same core idea as the original
[entropa-chain](https://github.com/aimozart) Rust project — rebuilt as a real
Java 21 / Spring Boot / Spring Cloud microservices system, following the
Config Server → Eureka → Gateway → event-driven-services pattern.

Built as a hands-on portfolio project alongside a Spring Boot microservices
course, targeting GKE/Kubernetes/Helm deployment.

## Architecture

```
Client → api-gateway (Spring Cloud Gateway)
              │
              ├─→ ingest-service    → Kafka (entropa.attestations)
              │                            │
              │                            ├─→ transparency-service → MongoDB Atlas
              │                            └─→ notification-service
              │
   eureka-server (service discovery)
   config-server (centralized config, backed by ./config-repo)
```

- **ingest-service** — accepts `POST /api/tx`, hashes the payload (SHA-256),
  publishes an `AttestationEvent` to Kafka, returns `202 Accepted` with a
  tracking ID immediately. Never touches the log directly.
- **transparency-service** — the single-writer sequencer. Consumes from
  Kafka, computes a real hash chain (each block commits to the previous
  block's hash + its own index + content hash — tampering with any record
  breaks every later hash), persists to MongoDB Atlas (Spring Data MongoDB), serves
  `GET /api/receipt/:trackingId` and `GET /api/chain`. Deliberately **not**
  horizontally scaled (replicas: 1 everywhere in this repo) — same
  single-writer-by-design tradeoff the original Rust system made, for the
  same reason (one writer means no reorder/race between concurrent appends).
- **notification-service** — event-driven, reacts to every new attestation.
  Structured logging today; the natural seam for real webhook/email delivery
  later, without touching ingest or transparency at all.
- **eureka-server** / **config-server** / **api-gateway** — standard Spring
  Cloud infrastructure: service discovery, centralized config
  (`./config-repo`, one YAML per service), and a single routed entry point.

## Running it locally

First, create your own local `.env` (never committed — see `.gitignore`):
```
cp .env.example .env
# then edit .env and set your own local-only values
```
Every value in `.env` protects only throwaway containers running on your own
machine (a local Postgres, a local Keycloak instance) — nothing real, and
nothing ever reachable outside your own Docker network. No default/example
password value is ever checked into this repo, on principle, not just
because these particular ones happen to be low-stakes.

```
docker compose up --build
```

This brings up all 6 services plus Kafka, Postgres, and the full
observability stack (Prometheus, Loki, Tempo, Grafana). First run downloads
a lot (Spring dependencies get re-resolved inside each service's Docker
build) — subsequent runs are much faster.

Once it's up:
- `http://localhost:8761` — Eureka dashboard, confirm all 6 services registered
- `http://localhost:8080/api/tx` — POST an attestation through the gateway
- `http://localhost:3000` — Grafana (anonymous admin access, local only)
- `http://localhost:9090` — Prometheus

Example submission:
```
curl -X POST http://localhost:8080/api/tx \
  -H "Content-Type: application/json" \
  -d '{"payload": "hello-entropa", "label": "test"}'
```

Then check the receipt with the returned `trackingId`:
```
curl http://localhost:8080/api/receipt/<trackingId>
```

## Running the whole thing down cleanly

```
docker compose down -v
```

`-v` also drops the Postgres volume — this is a demo/portfolio project, not
something holding real data worth preserving between sessions.

## Kubernetes / Helm

`helm/entropa/` — a real Helm chart, one Deployment+Service template pair
templated over a `services` map in `values.yaml` (not 12 near-duplicate
files). `transparency-service` is explicitly excluded from autoscaling in
`templates/hpa.yaml`, matching its single-writer design.

**Real cost note, stated plainly**: unlike the original Cloud-Run-based
system, this architecture does **not** scale to zero. JVM/Spring Boot cold
starts are too slow for request-triggered scaling, and Kafka/Postgres are
always-on stateful services by nature. Expect a real baseline cost for as
long as a GKE cluster running this is up — spin it up when actively
developing/demoing, tear it down (`helm uninstall` + delete the cluster)
when not in use, same discipline as the AWS/Terraform practice work.

Before installing the chart for real: create the DB credentials Secret
yourself (`kubectl create secret generic entropa-db-credentials
--from-literal=password=<real-value>`) — it's deliberately not templated
from a plaintext Helm value.

## Live demo

This is a portfolio/demo project — **zero real customers, no real billing**.
The live system runs on GKE at `entropa.space`: a real Stripe **test-mode**
checkout ($0, no real charge) unlocks a dashboard that starts real mock
AI-agent decisions flowing through the actual ingest → Kafka →
transparency-service → MongoDB Atlas pipeline, so the architecture is visibly
real and working without anyone needing to hand over real payment info or
trust an unproven product.

## What's built vs. what's next

**Built and verified**: all 7 services compile and pass their tests (real
hash-chain tamper-evidence logic, Kafka publish behavior), full Docker
Compose stack including observability, a real Helm chart, a live GKE
deployment behind a Google-managed-SSL load balancer, Spring
Security/OAuth2 (Keycloak-issued JWTs validated at the gateway on every
route except the public demo/signup paths), Resilience4j circuit breakers
wired into the gateway's routes to ingest-service and transparency-service,
and a real Stripe test-mode signup flow feeding a live demo dashboard.

**Not yet built**: Grafana dashboards (datasources are provisioned, no
dashboards built yet), a real contact-form backend (the public site's
contact page currently falls back to a `mailto:` link).

## Real incidents found and fixed

Kept here honestly, the same way the codebase itself is — every one of
these was a real bug in a real deployment, found with actual evidence
(logs, `curl`, `kubectl describe`), not a hypothetical.

- **Kafka's `bitnami/kafka` image was removed from Docker Hub's free
  tier** mid-project (a real 2025 industry change) — every pinned version
  tag started returning `ImagePullBackOff`. Fixed by switching to
  `bitnamilegacy/kafka`, Bitnami's actual free-tier replacement repo.
- **`config-server`'s own Dockerfile never copied `config-repo/` into its
  image** — every service was silently running on completely empty remote
  configuration the whole time (confirmed via `propertySources: []` from
  the config server's own API). Fixed by adding the missing `COPY` step.
- **The default `file:///${CONFIG_REPO_PATH:../config-repo}` search-location
  mis-parsed as an FTP hostname lookup** inside the container (`Caused by:
  java.net.UnknownHostException: config-repo`, routed through
  `sun.net.www.protocol.ftp`) — fixed with an explicit `file:/config-repo`
  path instead of the fragile relative-path default.
- **`config-server` bakes `config-repo/` into its Docker image at build
  time — it is not live-reloaded.** Editing a config file and expecting
  the running system to pick it up does nothing until `config-server`
  itself is rebuilt and redeployed. Cost real debugging time more than
  once before this was internalized.
- **A trailing newline byte baked into a Kubernetes Secret** (created via
  `echo` instead of `printf`) caused every Postgres authentication attempt
  to fail — the password value and the actual database password matched
  character-for-character except for that one invisible byte. Found via
  `xxd` on the decoded secret value.
- **Kafka requires `SASL_PLAINTEXT` authentication on its client listener**
  (a Bitnami chart default), but none of the Spring services had any SASL
  configuration at all — every produce/consume attempt looped forever on
  `Node -1 disconnected` instead of a clear auth error. Fixed by wiring
  real `security.protocol`/`sasl.mechanism`/`sasl.jaas.config` properties
  from a Kubernetes Secret into every Kafka-connected service.
- **Spring Cloud Gateway's route predicate `/api/signup/**` never matched
  the bare `/api/signup` path** the frontend actually calls — a real
  Spring `PathPattern` gotcha, not the older, more permissive
  `AntPathMatcher` behavior some documentation still assumes. Fixed by
  matching both the exact path and the wildcard.
- **Spring Security blocked the CORS `OPTIONS` preflight request itself**,
  before Spring Cloud Gateway's own `globalcors` filter ever got a chance
  to run — real `403`s on every cross-origin browser call despite CORS
  being correctly configured. Fixed by explicitly permitting `OPTIONS` on
  every path in the security filter chain.
- **JVM startup probes were tuned for an unconstrained environment** —
  under this cluster's real CPU budgets, cold starts regularly took
  90–125 seconds (once even climbing past a 300-second probe budget for a
  JPA/Hibernate + Postgres service). Fixed with a proper Kubernetes
  `startupProbe` sized to the real, measured cold-start time instead of a
  guess.

## Storage: MongoDB Atlas (migrated from Postgres, 2026-09-28)

`transparency-service` stores the hash chain in **MongoDB Atlas** (M0, Google Cloud `us-central1`). The Atlas
project, cluster, database user (`readWrite` on the `entropa` database only) and network allow list are
provisioned with **Pulumi (TypeScript)**. The connection string reaches the pod only as `SPRING_DATA_MONGODB_URI`,
read from a Kubernetes Secret; it is never in a file or in `config-repo`.

**What changed.** One service. The hashing (`ChainHasher`), the single writer (`TransparencyLogService`), the
controller and their existing tests are untouched: they depend on the repository interface, not the database.

| | Before | After |
|---|---|---|
| Dependency | `spring-boot-starter-data-jpa` + PostgreSQL driver | `spring-boot-starter-data-mongodb` |
| Record | `@Entity` row in table `attestation_records` | `@Document` in collection `attestation_records`; `@Field` names match the old columns |
| Uniqueness | `UNIQUE` constraints | unique indexes on `leaf_index` and `tracking_id` (`auto-index-creation: true`) |
| Repository | `JpaRepository<AttestationRecord, Long>` | `MongoRepository<AttestationRecord, String>` |
| Tests | none for the repository | `AttestationRepositoryMongoTest` (4 tests), written first; CI runs a `mongo:8` service container |

**Cutover with no lost events.**

1. Scaled `transparency-service` to 0. The ingest service kept accepting decisions, and they waited in Kafka.
2. Exported the now-frozen table as Extended JSON (`$numberLong`, `$date`), so 64-bit integers and dates keep their BSON types.
3. Imported into Atlas with `mongoimport` and recomputed every block hash from scratch.
4. Merged; CD deployed the new version. It rejoined the same Kafka consumer group and resumed from its committed offset.

**Proof.**

- Hash chain recomputed over every record, across the join between migrated and newly written records:
  `35 records checked, 0 broken` (records 0–24 from Postgres, 25–34 written by the MongoDB version).
- `explain("executionStats")` on Atlas for the two queries the service runs constantly:

| Query | Winning plan | Docs examined |
|---|---|---|
| Receipt by `tracking_id` | `EXPRESS_IXSCAN` on `tracking_id` | 1 |
| Newest record (the writer asks this on every append) | `LIMIT ← FETCH ← IXSCAN` on `leaf_index`, backward: no in-memory sort | 1 |

Without the indexes, both queries were `COLLSCAN`s that examined every record (measured on a local copy), so each
append would get slower as the chain grew. With them, the cost stays at one document.

**Design notes.**

- The new test failed first for the right reason: `@DataMongoTest` only builds MongoDB repositories, so the JPA
  repository couldn't be injected.
- Spring Boot 3 creates no indexes unless `spring.data.mongodb.auto-index-creation` is on; `@Indexed` alone does
  nothing. Turning it off lets duplicate tracking IDs in, and a test catches that.
- `@Field` keeps the stored field names identical to the old columns. The API's JSON comes from the getters, so it's
  unchanged (`leafIndex`, `trackingId`…) and no consumer noticed the move.

**Known limitations.**

- Atlas allows the GKE nodes' public IPs, and Autopilot can replace nodes. Production would send traffic out through
  one reserved address (Cloud NAT) or use a private endpoint (Atlas M10+).
- `docker-compose.yml` still starts Postgres for local runs; the local stack needs a MongoDB service before
  `transparency-service` runs locally again.
- Postgres stays in the cluster as the rollback path until it's retired.
