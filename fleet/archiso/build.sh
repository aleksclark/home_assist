#!/bin/bash
set -euo pipefail

# Build the fleet bootstrap Arch ISO
# Based on the official releng profile with SSH + fleet customizations.
#
# Usage:
#   ./build.sh [--inject-key ~/.ssh/id_rsa.pub]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="/tmp/fleet-archiso-build"
OUT_DIR="${SCRIPT_DIR}/out"
SSH_KEY=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --inject-key)
            SSH_KEY="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if ! command -v mkarchiso &>/dev/null; then
    echo "ERROR: archiso not installed. Run: pacman -S archiso"
    exit 1
fi

# Clean previous build
sudo rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}" "${OUT_DIR}"

# Copy the full profile to work dir
cp -a "${SCRIPT_DIR}"/airootfs "${WORK_DIR}/"
cp -a "${SCRIPT_DIR}"/efiboot "${WORK_DIR}/"
cp -a "${SCRIPT_DIR}"/grub "${WORK_DIR}/"
cp -a "${SCRIPT_DIR}"/syslinux "${WORK_DIR}/"
cp "${SCRIPT_DIR}"/profiledef.sh "${WORK_DIR}/"
cp "${SCRIPT_DIR}"/packages.x86_64 "${WORK_DIR}/"
cp "${SCRIPT_DIR}"/pacman.conf "${WORK_DIR}/"
cp "${SCRIPT_DIR}"/bootstrap_packages "${WORK_DIR}/" 2>/dev/null || true

# Inject SSH public key if provided
if [ -n "${SSH_KEY}" ]; then
    EXPANDED_KEY="${SSH_KEY/#\~/$HOME}"
    if [ -f "${EXPANDED_KEY}" ]; then
        echo "Injecting SSH key: ${EXPANDED_KEY}"
        mkdir -p "${WORK_DIR}/airootfs/root/.ssh"
        cp "${EXPANDED_KEY}" "${WORK_DIR}/airootfs/root/.ssh/authorized_keys"
    else
        echo "ERROR: SSH key file not found: ${SSH_KEY} (expanded: ${EXPANDED_KEY})"
        exit 1
    fi
else
    echo "WARNING: No SSH key injected. Root password 'fleet' will be the only access method."
    echo "  Use --inject-key ~/.ssh/id_rsa.pub to add your key."
fi

echo "=== Building fleet bootstrap ISO ==="
sudo mkarchiso -v -w "${WORK_DIR}/work" -o "${OUT_DIR}" "${WORK_DIR}"

echo ""
echo "=== Build complete ==="
echo "ISO: $(ls -1t ${OUT_DIR}/fleet-bootstrap-*.iso 2>/dev/null | head -1)"
echo ""
echo "Write to USB:"
echo "  sudo dd bs=4M if=<iso> of=/dev/sdX status=progress oflag=sync"
