#!/usr/bin/env bash
#
# Build patched OpenBeken firmware for the Della mini-split RTL87X0C modules.
#
# Clones the aleksclark/OpenBK7231T_App fork (della-heat-cool branch) and
# compiles the OpenRTL87X0C target via Docker.  Outputs land in
#   devices/della-minisplits/firmware/
#
# Prerequisites: git, docker
#
# Usage:
#   ./build-firmware.sh            # uses default version tag
#   ./build-firmware.sh 1.18.244   # custom version string
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FW_OUT="$SCRIPT_DIR/firmware"
BUILD_DIR="$SCRIPT_DIR/.build"
REPO_URL="git@github.com:aleksclark/OpenBK7231T_App.git"
BRANCH="della-heat-cool"
DOCKER_IMAGE="openbk_rtl87x0c"

VERSION="${1:-$(date +%Y%m%d_%H%M%S)}"

command -v git   >/dev/null || { echo "ERROR: git not found"   >&2; exit 1; }
command -v docker >/dev/null || { echo "ERROR: docker not found" >&2; exit 1; }

# ── 1. Clone / update source ────────────────────────────────────────────────
echo "==> Preparing source in $BUILD_DIR"
if [ -d "$BUILD_DIR/.git" ]; then
    echo "    Existing clone found — fetching latest"
    git -C "$BUILD_DIR" remote set-url origin "$REPO_URL"
    git -C "$BUILD_DIR" fetch --depth=1 origin "$BRANCH"
    git -C "$BUILD_DIR" checkout -B "$BRANCH" FETCH_HEAD
    git -C "$BUILD_DIR" clean -fdx -e sdk/
else
    echo "    Cloning $REPO_URL (shallow, branch $BRANCH)"
    rm -rf "$BUILD_DIR"
    git clone --depth=1 --branch "$BRANCH" "$REPO_URL" "$BUILD_DIR"
fi

# ── 2. Build Docker image (cached after first run) ──────────────────────────
# The SDK's bundled ARM GCC needs GLIBC ≥2.32; upstream Dockerfile uses
# Ubuntu 20.04 (GLIBC 2.31) which is too old.  Patch it to 22.04 at build time.
echo "==> Building Docker image: $DOCKER_IMAGE"
DOCKER_CTX=$(mktemp -d)
sed 's/ubuntu:20.04/ubuntu:22.04/' "$BUILD_DIR/docker/Dockerfile" > "$DOCKER_CTX/Dockerfile"
cp "$BUILD_DIR/docker/build_target_platforms.sh" "$DOCKER_CTX/"
docker build \
    --platform linux/amd64 \
    -t "$DOCKER_IMAGE" \
    --build-arg UID="$(id -u)" \
    --build-arg USERNAME="$(id -un)" \
    "$DOCKER_CTX"
rm -rf "$DOCKER_CTX"

# ── 3. Compile firmware ────────────────────────────────────────────────────
echo "==> Compiling OpenRTL87X0C (version: $VERSION)"
docker run --rm \
    --platform linux/amd64 \
    -v "$BUILD_DIR":/OpenBK7231T_App \
    -e APP_VERSION="$VERSION" \
    -e TARGET_SDKS="OpenRTL87X0C" \
    "$DOCKER_IMAGE"

# ── 4. Copy outputs ────────────────────────────────────────────────────────
BIN="$BUILD_DIR/output/$VERSION/OpenRTL87X0C_${VERSION}.bin"
OTA="$BUILD_DIR/output/$VERSION/OpenRTL87X0C_${VERSION}_ota.img"

if [ ! -f "$BIN" ] || [ ! -f "$OTA" ]; then
    echo "ERROR: build outputs not found in $BUILD_DIR/output/$VERSION/" >&2
    echo "       Check Docker build log above for errors." >&2
    exit 1
fi

mkdir -p "$FW_OUT"
cp "$BIN" "$FW_OUT/OpenRTL87X0C_${VERSION}_heat_cool.bin"
cp "$OTA" "$FW_OUT/OpenRTL87X0C_${VERSION}_heat_cool_ota.img"

echo ""
echo "=== Build complete ==="
echo "  flash:  $FW_OUT/OpenRTL87X0C_${VERSION}_heat_cool.bin"
echo "  OTA:    $FW_OUT/OpenRTL87X0C_${VERSION}_heat_cool_ota.img"
echo ""
echo "Flash via OTA:"
echo "  curl 'http://<device_ip>/ota_exec?host=<http_server>&file=OpenRTL87X0C_${VERSION}_heat_cool_ota.img'"
