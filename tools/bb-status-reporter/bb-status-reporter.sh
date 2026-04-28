#!/usr/bin/env bash
# BB Status Reporter — publishes workstation status to Home Assistant
# Runs as a systemd user service, polls every 10 seconds.
#
# States:
#   away    — monitors are off (DPMS standby/suspend/off)
#   working — monitors on, BeyondAllReason is NOT running
#   playing — BeyondAllReason is running
#
# Publishes to HA via REST API: input_text.bb_status

set -euo pipefail

HA_HOST="${HA_HOST:-192.168.0.89}"
HA_PORT="${HA_PORT:-8123}"
HA_TOKEN="${HA_TOKEN:?BB_STATUS: HA_TOKEN must be set (long-lived access token)}"
HA_URL="http://${HA_HOST}:${HA_PORT}"
ENTITY_ID="input_text.bb_status"
POLL_INTERVAL="${POLL_INTERVAL:-10}"

prev_status=""

get_status() {
    # Check DPMS — if monitors are off, we're away
    local dpms_state
    dpms_state=$(DISPLAY=:0 xset q 2>/dev/null | grep -oP 'Monitor is \K\w+' || echo "unknown")

    if [[ "$dpms_state" != "On" ]]; then
        echo "away"
        return
    fi

    # Monitors are on — check if BeyondAllReason (recoil/spring engine) is running
    if pgrep -f 'recoil.*/spring' >/dev/null 2>&1 || pgrep -f 'beyond-all-reason' >/dev/null 2>&1; then
        echo "playing"
    else
        echo "working"
    fi
}

publish_status() {
    local status="$1"
    curl -sf -X POST "${HA_URL}/api/states/${ENTITY_ID}" \
        -H "Authorization: Bearer ${HA_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{\"state\": \"${status}\", \"attributes\": {\"friendly_name\": \"BB Status\", \"source\": \"workstation\"}}" \
        >/dev/null 2>&1 || echo "BB_STATUS: failed to publish to HA" >&2
}

echo "BB Status Reporter starting (polling every ${POLL_INTERVAL}s)"
echo "  HA: ${HA_URL}  Entity: ${ENTITY_ID}"

while true; do
    status=$(get_status)

    if [[ "$status" != "$prev_status" ]]; then
        echo "Status changed: ${prev_status:-<none>} -> ${status}"
        publish_status "$status"
        prev_status="$status"
    fi

    sleep "$POLL_INTERVAL"
done
