#!/usr/bin/env bash
set -euo pipefail

HA_HOST="aleks@192.168.0.89"
HA_CONFIG_DIR="/mnt/moosefs/configs/homeassistant"
LOCAL_DIR="$(cd "$(dirname "$0")" && pwd)"
SNAPSHOT="${LOCAL_DIR}/ha-config-snapshot"

if [ ! -d "${SNAPSHOT}" ]; then
    echo "ERROR: No config snapshot found at ${SNAPSHOT}"
    echo "Run ha-download.sh first, make your changes, then run this script."
    exit 1
fi

if [ ! -f "${SNAPSHOT}/configuration.yaml" ]; then
    echo "ERROR: ${SNAPSHOT}/configuration.yaml not found — snapshot looks incomplete."
    exit 1
fi

echo "=== Validating local config ==="
echo "Checking configuration.yaml includes..."

for f in input_helpers.yaml mqtt.yaml; do
    if ! grep -q "$f" "${SNAPSHOT}/configuration.yaml" 2>/dev/null; then
        echo "WARNING: ${f} may not be included in configuration.yaml"
    fi
done

echo ""
echo "=== Uploading config to ${HA_HOST}:${HA_CONFIG_DIR}/ ==="

rsync -avz \
    --include='*.yaml' \
    --include='*/' \
    --exclude='*' \
    "${SNAPSHOT}/" \
    "${HA_HOST}:${HA_CONFIG_DIR}/" || {
    rc=$?
    if [ $rc -eq 23 ]; then
        echo "(rsync code 23: some attrs not transferred — OK for docker ownership)"
    else
        echo "rsync failed with code $rc"
        exit $rc
    fi
}

echo ""
echo "=== Restarting Home Assistant via Nomad ==="
ssh aleks@192.168.0.23 "NOMAD_ADDR=http://192.168.0.23:4646 nomad job restart -on-error=fail -task homeassistant homeassistant"

echo ""
echo "Waiting for HA to come back online..."
for i in $(seq 1 60); do
    if ssh "${HA_HOST}" "curl -sf http://localhost:8123/api/ -H 'Authorization: Bearer \$(<~/.ha_token)'" >/dev/null 2>&1; then
        echo "HA is up after ~$((i * 2))s"
        exit 0
    fi
    sleep 2
done

echo "HA did not respond within 120s — check logs with:"
echo "  nomad alloc logs -job homeassistant"
exit 1
