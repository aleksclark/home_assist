#!/usr/bin/env bash
# L1 repository-local static contract for minisplit deploy/nomad (plan 03 §3.6).
# Monorepo path: services/minisplit-otel-poller/deploy/nomad/
# No Nomad credentials. No secret values. No cluster mutation. Fail closed.
set -euo pipefail

NOMAD_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_ROOT="$(cd "${NOMAD_DIR}/../.." && pwd)"
REPO_ROOT="$(cd "${SERVICE_ROOT}/../.." && pwd)"
JOB="${NOMAD_DIR}/jobs/minisplit-otel-poller.nomad.hcl"
MANIFEST="${NOMAD_DIR}/deployment.yaml"
ENV_FILE="${NOMAD_DIR}/env/home.nomadvars.hcl"
LOCK="${NOMAD_DIR}/images.lock.hcl"
README="${NOMAD_DIR}/README.md"
EXPECTED="${NOMAD_DIR}/tests/expected-services.json"
CODEOWNERS="${REPO_ROOT}/.github/CODEOWNERS"
LEGACY="${SERVICE_ROOT}/minisplit-otel-poller.nomad.hcl"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "OK: $*"; }

need_file() {
  [[ -f "$1" ]] || fail "missing required file: $1"
}

echo "==> minisplit plan-03 contract @ ${NOMAD_DIR}"
echo "    repo_root=${REPO_ROOT}"

need_file "$JOB"
need_file "$MANIFEST"
need_file "$ENV_FILE"
need_file "$LOCK"
need_file "$README"
need_file "$EXPECTED"
need_file "$CODEOWNERS"
need_file "$LEGACY"
pass "layout files present"

# Relative monorepo path sanity (fleet points here, not repo-root deploy/nomad)
case "${NOMAD_DIR}" in
  */services/minisplit-otel-poller/deploy/nomad) ;;
  *) fail "NOMAD_DIR must end with services/minisplit-otel-poller/deploy/nomad (got ${NOMAD_DIR})" ;;
esac
pass "monorepo deploy path"

# --- deployment.yaml required fields (plan 03 §3.1) ---
for needle in \
  'schema_version: 1' \
  'project: home-assist-minisplit' \
  'owner: aleks-clark' \
  'repository: https://github.com/aleksclark/home_assist' \
  'ref_policy: signed-default-branch-commit' \
  'namespace: default' \
  'datacenters: [home]' \
  'name: minisplit' \
  'id: minisplit-otel-poller' \
  'spec: jobs/minisplit-otel-poller.nomad.hcl' \
  'env: env/home.nomadvars.hcl' \
  'images: images.lock.hcl' \
  'nomad/jobs/minisplit-otel-poller' \
  'rollout: serial' \
  'prune: explicit-only'
do
  grep -Fq "$needle" "$MANIFEST" || fail "deployment.yaml missing ${needle}"
done
# Forbid absolute / traversal specs
if grep -E '^\s*spec:\s*/' "$MANIFEST"; then
  fail "absolute spec path forbidden in deployment.yaml"
fi
if grep -Fq '..' "$MANIFEST"; then
  fail "path traversal forbidden in deployment.yaml"
fi
# No leftover pre-plan03 keys as authority
for bad in 'job_id:' 'canonical_path:' 'classification:' 'secret_variable_path:'; do
  grep -Fq "$bad" "$MANIFEST" && fail "deployment.yaml still has pre-plan03 key ${bad}" || true
done
pass "deployment.yaml required fields"

# --- CODEOWNERS (monorepo paths) ---
grep -Eq '^/services/minisplit-otel-poller/deploy/nomad/[[:space:]]+@aleksclark' "$CODEOWNERS" \
  || fail "CODEOWNERS must own /services/minisplit-otel-poller/deploy/nomad/ @aleksclark"
grep -Eq '^/\.github/workflows/release' "$CODEOWNERS" \
  || fail "CODEOWNERS must own release workflows"
pass "CODEOWNERS"

# --- images.lock.hcl digest-only ---
LOCK_BODY="$(cat "$LOCK")"
echo "$LOCK_BODY" | grep -Eq 'image_minisplit_otel_poller[[:space:]]*=[[:space:]]*"ghcr\.io/aleksclark/minisplit-otel-poller@sha256:[0-9a-f]{64}"' \
  || fail "images.lock.hcl must set image_minisplit_otel_poller to repo@sha256:<64hex>"
echo "$LOCK_BODY" | grep -Eiq ':(latest|main|master)"' && fail "images.lock.hcl forbids mutable tags as authority" || true
echo "$LOCK_BODY" | grep -Eq 'PLACEHOLDER' && fail "images.lock.hcl still has PLACEHOLDER" || true
DIGEST="$(echo "$LOCK_BODY" | sed -n 's/.*@sha256:\([0-9a-f]\{64\}\).*/\1/p' | head -1)"
[[ -n "$DIGEST" ]] || fail "could not parse lock digest"
# Forbid multi-field tag/digest split as authority (old shape)
echo "$LOCK_BODY" | grep -Eq '^[[:space:]]*tag[[:space:]]*=' \
  && fail "images.lock.hcl must be digest-only (no separate tag= authority)" || true
pass "images.lock.hcl digest-only (${DIGEST:0:12}…)"

# --- jobspec: job id, digest pin, durable endpoints ---
JOB_BODY="$(cat "$JOB")"
echo "$JOB_BODY" | grep -Fq 'job "minisplit-otel-poller"' || fail 'jobspec missing job "minisplit-otel-poller"'
echo "$JOB_BODY" | grep -Fq "@sha256:${DIGEST}" \
  || fail "jobspec image digest must match images.lock.hcl"
echo "$JOB_BODY" | grep -Eiq 'image[[:space:]]*=[[:space:]]*"[^"]*:latest"' \
  && fail "jobspec must not use :latest" || true
if echo "$JOB_BODY" | grep -E 'image[[:space:]]*=' | grep -v '@sha256:' | grep -Eq 'image[[:space:]]*='; then
  fail "jobspec image must include @sha256 digest (no mutable tag-only pin)"
fi
echo "$JOB_BODY" | grep -Fq 'MQTT_BROKER = "tcp://mqtt.fleet.clark.team:1883"' \
  || fail "MQTT_BROKER must be durable mqtt.fleet.clark.team:1883"
echo "$JOB_BODY" | grep -Fq 'OTEL_EXPORTER_OTLP_ENDPOINT = "http://${attr.unique.network.ip-address}:4328"' \
  || fail "OTLP must be per-node otel-agent :4328 with attr interpolation"
echo "$JOB_BODY" | grep -Fq 'otel-collector.fleet.clark.team' \
  && fail "stale otel-collector FQDN forbidden" || true
if echo "$JOB_BODY" | grep 'OTEL_EXPORTER_OTLP_ENDPOINT' | grep -q ':4318'; then
  fail "central collector :4318 forbidden"
fi
echo "$JOB_BODY" | grep -Fq 'nomadVar "nomad/jobs/minisplit-otel-poller"' \
  || fail 'must reference nomadVar "nomad/jobs/minisplit-otel-poller"'
echo "$JOB_BODY" | grep -Fq '.mqtt_username' || fail "must template mqtt_username"
echo "$JOB_BODY" | grep -Fq '.mqtt_password' || fail "must template mqtt_password"
echo "$JOB_BODY" | grep -Eq 'max_parallel[[:space:]]*=[[:space:]]*1' \
  || fail "jobspec must set max_parallel = 1"
echo "$JOB_BODY" | grep -Eq 'canary[[:space:]]*=[[:space:]]*0' \
  || fail "jobspec must set canary = 0"
echo "$JOB_BODY" | grep -Eq 'count[[:space:]]*=[[:space:]]*1' \
  || fail "jobspec group count must be 1"
echo "$JOB_BODY" | grep -Fq 'observe_only' || fail "jobspec missing observe_only"
echo "$JOB_BODY" | grep -Fq 'static = 9105' || fail "jobspec missing health port 9105"
pass "jobspec digest pin + durable endpoints"

# --- secrets: no literals ---
SECRET_SCAN_FILES=("$JOB" "$MANIFEST" "$ENV_FILE" "$LOCK" "$README" "$EXPECTED" "$CODEOWNERS")
for f in "${SECRET_SCAN_FILES[@]}"; do
  if grep -Eiq 'BEGIN (RSA |OPENSSH )?PRIVATE KEY' "$f"; then
    fail "private key material in $f"
  fi
  if grep -Eiq 'postgres(ql)?://[^[:space:]"]+:[^[:space:]"]+@' "$f"; then
    fail "credentialed DSN shape in $f"
  fi
done
if echo "$JOB_BODY" | grep -Eiq 'MQTT_PASSWORD[[:space:]]*=[[:space:]]*"[^"]{8,}"'; then
  fail "inline MQTT password forbidden"
fi
pass "no secret literals; nomadVar present"

# --- env overlay: non-secret only ---
ENV_BODY="$(cat "$ENV_FILE")"
echo "$ENV_BODY" | grep -Eiq 'mqtt_password[[:space:]]*=' \
  && fail "env overlay must not assign mqtt_password" || true
echo "$ENV_BODY" | grep -Eiq 'password[[:space:]]*=' \
  && fail "env overlay must not assign passwords" || true
for needle in \
  'mqtt.fleet.clark.team' \
  'per_node_otel_agent' \
  '4328' \
  '9105' \
  'device_count' \
  'observe_only'
do
  grep -Fq "$needle" "$ENV_FILE" || fail "env missing ${needle}"
done
pass "env overlay non-secret site policy"

# --- expected-services.json sanity ---
command -v python3 >/dev/null 2>&1 || fail "python3 required for expected-services.json parse"
python3 - "$EXPECTED" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    data = json.load(f)
assert data.get("release_set") == "minisplit", data.get("release_set")
assert data.get("project") == "home-assist-minisplit", data.get("project")
jobs = data.get("jobs") or []
assert len(jobs) == 1, jobs
j = jobs[0]
assert j.get("id") == "minisplit-otel-poller"
assert j.get("observe_only") is True
assert j.get("secret_variable_path") == "nomad/jobs/minisplit-otel-poller"
assert j.get("secret_keys") == ["mqtt_username", "mqtt_password"]
assert j.get("image_var") == "image_minisplit_otel_poller"
assert j.get("image", {}).get("authority") == "digest"
assert j.get("device_inventory", {}).get("expected_count") == 3
eps = j.get("endpoints") or {}
assert eps.get("otlp_port") == 4328
assert "mqtt.fleet.clark.team" in (eps.get("mqtt_broker") or "")
assert j.get("groups", [{}])[0].get("count") == 1
upd = j["groups"][0].get("update", {})
assert upd.get("max_parallel") == 1
assert upd.get("canary") == 0
print("OK: expected-services.json structure")
PY

# --- README operator notes ---
for needle in \
  'observe-only' \
  'mqtt.fleet.clark.team' \
  '4328' \
  '4318' \
  'images.lock.hcl' \
  'nomad/jobs/minisplit-otel-poller' \
  'services/minisplit-otel-poller/deploy/nomad/deployment.yaml' \
  'explicit-only' \
  'metric_ok'
do
  grep -Fiq "$needle" "$README" || fail "README missing operator note: ${needle}"
done
pass "README operator notes"

# --- legacy MOVED pointer ---
LEGACY_BODY="$(cat "$LEGACY")"
echo "$LEGACY_BODY" | grep -Fq 'MOVED' || fail "legacy jobspec must be MOVED pointer"
echo "$LEGACY_BODY" | grep -Fq 'deploy/nomad/jobs/minisplit-otel-poller.nomad.hcl' \
  || fail "legacy pointer must reference canonical jobspec path"
# Mentions of retired local image are OK only as "retired" documentation alongside MOVED.
if echo "$LEGACY_BODY" | grep -Eq 'image[[:space:]]*='; then
  fail "legacy jobspec must not retain image= authority"
fi
if echo "$LEGACY_BODY" | grep -Fq 'job "minisplit-otel-poller"'; then
  fail "legacy path must not remain a runnable job block"
fi
pass "legacy MOVED pointer"

# --- optional Go contract tests ---
if command -v go >/dev/null 2>&1; then
  (cd "$SERVICE_ROOT" && go test ./deploy/nomad/jobs/ -count=1) \
    || fail "go test ./deploy/nomad/jobs/ failed"
  pass "go test ./deploy/nomad/jobs/"
else
  echo "WARN: go not installed; skipped go test"
fi

echo
echo "PASS: minisplit deploy/nomad contract checks"
