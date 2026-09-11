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
              │                            ├─→ transparency-service → Postgres
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
  breaks every later hash), persists via JPA/Postgres, serves
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

## What's built vs. what's next

**Built and verified**: all 6 services compile and pass their tests (real
hash-chain tamper-evidence logic, Kafka publish behavior), full Docker
Compose stack including observability, Helm chart skeleton.

**Not yet built**: Spring Security/OAuth2 (no auth on any endpoint yet —
do not expose this publicly as-is), Resilience4j actually wired into
inter-service calls (dependency is present, circuit breakers aren't applied
to any real call path yet), a live GKE deployment, Grafana dashboards
(datasources are provisioned, no dashboards built yet).
