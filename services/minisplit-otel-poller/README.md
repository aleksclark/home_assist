# minisplit-otel-poller

Observe-only OTEL/MQTT telemetry poller for Della minisplit units.

## Contract

- **Job ID:** `minisplit-otel-poller` (project-owned)
- **Canonical jobspec:** `deploy/nomad/jobs/minisplit-otel-poller.nomad.hcl`
- **Image:** `ghcr.io/aleksclark/minisplit-otel-poller` tag@digest (see `deploy/nomad/images.lock.hcl`)
- **MQTT:** subscribe-only to `*/get` telemetry topics; broker `tcp://mqtt.fleet.clark.team:1883` (VIP `.100`)
- **OTLP:** `http://otel-collector.fleet.clark.team:4318`
- **Health:** `:9105` `/healthz` `/readyz` `/metricsz`
- **Secrets:** optional `nomad/jobs/minisplit-otel-poller` keys `mqtt_username`, `mqtt_password`

## Observe-only guarantee

- No MQTT Publish
- No HTTP POST/PUT/PATCH/DELETE to devices
- Only GET `http://<device>/lograw` and MQTT subscribe on allowed `/get` suffixes
- Unit tests reject control topic classes and publish APIs

## Deploy

1. Merge code + release workflow → GHCR image + CalVer tag
2. Pin PR updates jobspec + `images.lock.hcl` digest
3. Ensure Nomad Variable keys exist (empty strings OK if anonymous MQTT)
4. Reviewed `nomad job plan` + `nomad job run -check-index` CAS (not fleet GHA wrappers)
5. Verify Stable, digest, readyz, metricsz counts, OTEL class/count, no control

## Tests

```bash
cd services/minisplit-otel-poller
go test ./... -count=1
go test ./deploy/nomad/jobs -count=1
```
