# Nomad deployment contract (minisplit-otel-poller)

Authoritative project-owned definition for the **minisplit** release set shipped
from `aleksclark/home_assist` (monorepo path under
`services/minisplit-otel-poller/`). Layout matches plan 03 §3.

Fleet reconciler source `project-home-assist` should point at this manifest:

```text
services/minisplit-otel-poller/deploy/nomad/deployment.yaml
```

Not a repo-root `deploy/nomad/deployment.yaml`.

## Layout

| Path | Role |
|---|---|
| `deployment.yaml` | Source/ownership/reconcile manifest (schema_version 1) |
| `jobs/minisplit-otel-poller.nomad.hcl` | Portable jobspec (one top-level job) |
| `env/home.nomadvars.hcl` | Non-secret home-fleet overlay |
| `images.lock.hcl` | Immutable image digest lock |
| `tests/` | Static contract tests (no secrets, no live Nomad) |

## Ownership

- Project: `home-assist-minisplit`
- Release set: `minisplit`
- Owner identity (manifest): `aleks-clark`
- GitHub CODEOWNERS: `@aleksclark` on this tree and release workflow
- Job ID: `minisplit-otel-poller` (must stay stable)
- Classification: project-owned, **stateless singleton**, **observe-only**
- Secret path: `nomad/jobs/minisplit-otel-poller` → keys `mqtt_username`, `mqtt_password`
- Fleet ledger tracks ownership; **do not** duplicate this jobspec in fleet-iac

## Runtime contract (non-secret)

| Item | Value |
|---|---|
| Image | `ghcr.io/aleksclark/minisplit-otel-poller` digest-pinned in `images.lock.hcl` + jobspec |
| MQTT broker | `tcp://mqtt.fleet.clark.team:1883` (VIP class `192.168.0.100:1883`) |
| OTLP | per-node otel-agent `http://${attr.unique.network.ip-address}:4328` |
| OTLP forbidden | `otel-collector.fleet.clark.team:4318`, direct node IP `:4318` |
| Health | host static `9105` — `/healthz`, `/readyz`, `/metricsz` |
| Devices | env `DEVICES` count **3** (kitchen / livingroom / amos) |
| Resources | cpu 100 / memory 64 |
| Update | `max_parallel = 1`, `canary = 0`, `auto_revert = true`, rollout **serial** |
| Prune | **explicit-only** (never auto-purge) |

## Observe-only guarantee

- MQTT subscribe-only on allowed `*/get` telemetry topics
- HTTP GET device `lograw` only — no POST/PUT/PATCH/DELETE to devices
- No device control publish paths
- Unit tests reject control topic classes and publish APIs

## OTEL / export proof

Endpoint health is verified externally:

1. Local node `:4328` accept (per-node otel-agent)
2. otel-agent export counters
3. SigNoz / collector metric class + counts

App `/metricsz` `metric_ok` is **local Record only** — not an export ACK. Do not
treat `metric_ok` alone as OTEL delivery proof.

## Secrets

Create Nomad Variable path `nomad/jobs/minisplit-otel-poller` with key names only:

- `mqtt_username`
- `mqtt_password`

Empty strings are OK when the broker allows anonymous subscribe. Values never
belong in git, overlays, image locks, plan output, or logs.

## Image authority

1. Release workflow publishes `ghcr.io/aleksclark/minisplit-otel-poller:<calver>` and emits a digest.
2. Pin PR updates `images.lock.hcl` and the jobspec image line to the **same** digest.
3. Never treat `:latest` or floating tags as deploy authority.
4. Lock form is digest-only: `image_minisplit_otel_poller = "ghcr.io/...@sha256:<64hex>"`.

## Validation (source-only)

```bash
# L1 contract (no Nomad credentials, no cluster mutation)
./services/minisplit-otel-poller/deploy/nomad/tests/contract.sh

# Existing Go jobspec contract tests
cd services/minisplit-otel-poller
go test ./deploy/nomad/jobs -count=1
```

Live enroll / CAS deploy / fleet reconciler flip is **out of scope** for this
source contract PR. Do not run `nomad job run` from this tree as part of CI.

## Legacy path

`services/minisplit-otel-poller/minisplit-otel-poller.nomad.hcl` is a MOVED
pointer only. Do not deploy it. Local image `minisplit-otel-poller:v1` is retired.

## Audit hygiene

Record endpoint class, HTTP status, digest, device_count, PR/SHA, observe_only.
Never MQTT passwords, Variable values, or device control payloads.
