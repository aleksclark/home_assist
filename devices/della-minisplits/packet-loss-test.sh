#!/usr/bin/env bash
# Measure packet loss to the 3 OpenBeken mini-split devices over 15 minutes.
# Pings each device once per second (900 pings total) in parallel,
# then prints a summary.

set -euo pipefail

declare -A DEVICES=(
  [livingroom]=192.168.0.21
  [amos]=192.168.0.25
  [kitchen]=192.168.0.4
)

DURATION=900
OUTDIR=$(mktemp -d)

echo "Pinging 3 mini-split devices for 15 minutes (${DURATION} pings each)..."
echo "Results will be saved to ${OUTDIR}"
echo ""

for name in "${!DEVICES[@]}"; do
  ip=${DEVICES[$name]}
  echo "  ${name} → ${ip}"
  ping -c "$DURATION" -i 1 -W 2 "$ip" > "${OUTDIR}/${name}.txt" 2>&1 &
done

echo ""
echo "Running until $(date -d "+${DURATION} seconds" '+%H:%M:%S' 2>/dev/null || date -v+${DURATION}S '+%H:%M:%S') ..."
wait

echo ""
echo "=== Results ==="
echo ""

for name in livingroom amos kitchen; do
  ip=${DEVICES[$name]}
  stats=$(tail -2 "${OUTDIR}/${name}.txt")
  loss=$(echo "$stats" | grep -oP '\d+(\.\d+)?% packet loss' || echo "parse error")
  rtt=$(echo "$stats" | grep -oP 'rtt .+' || true)
  printf "%-12s (%s): %s\n" "$name" "$ip" "$loss"
  if [[ -n "$rtt" ]]; then
    printf "%-12s            %s\n" "" "$rtt"
  fi
done

echo ""
echo "Raw logs: ${OUTDIR}/"
