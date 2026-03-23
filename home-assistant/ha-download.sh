#!/usr/bin/env bash
set -euo pipefail

HA_HOST="aleks@192.168.0.3"
HA_CONFIG_DIR="homeassistant_config"
LOCAL_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Downloading HA config from ${HA_HOST}:~/${HA_CONFIG_DIR}/ ..."

rsync -avz --delete \
    --exclude '.storage/' \
    --exclude '*.log' \
    --exclude '*.db' \
    --exclude '*.db-shm' \
    --exclude '*.db-wal' \
    --exclude 'tts/' \
    --exclude 'backups/' \
    --exclude '__pycache__/' \
    --exclude 'deps/' \
    --exclude '.cloud/' \
    "${HA_HOST}:~/${HA_CONFIG_DIR}/" \
    "${LOCAL_DIR}/ha-config-snapshot/"

echo ""
echo "Downloaded to: ${LOCAL_DIR}/ha-config-snapshot/"
echo "Key files:"
ls -la "${LOCAL_DIR}/ha-config-snapshot/configuration.yaml" 2>/dev/null && true
ls -la "${LOCAL_DIR}/ha-config-snapshot/automations.yaml" 2>/dev/null && true
