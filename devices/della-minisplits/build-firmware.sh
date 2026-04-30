#!/usr/bin/env bash
#
# Build patched OpenBeken firmware for the Della mini-split RTL87X0C modules.
#
# Clones the OpenBK7231T_App repo, applies the heat_cool + publish-throttle
# patch, and compiles the OpenRTL87X0C target via Docker.  Outputs land in
#   devices/della-minisplits/firmware/
#
# Prerequisites: git, docker
#
# Usage:
#   ./build-firmware.sh            # uses default version tag
#   ./build-firmware.sh 1.18.242   # custom version string
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PATCH_FILE="$SCRIPT_DIR/openbeken-heat-cool.patch"
FW_OUT="$SCRIPT_DIR/firmware"
BUILD_DIR="$SCRIPT_DIR/.build"
REPO_URL="https://github.com/openshwprojects/OpenBK7231T_App.git"
DOCKER_IMAGE="openbk_rtl87x0c"

VERSION="${1:-$(date +%Y%m%d_%H%M%S)}"

if [ ! -f "$PATCH_FILE" ]; then
    echo "ERROR: patch not found: $PATCH_FILE" >&2
    exit 1
fi

command -v git   >/dev/null || { echo "ERROR: git not found"   >&2; exit 1; }
command -v docker >/dev/null || { echo "ERROR: docker not found" >&2; exit 1; }

# ── 1. Clone / update source ────────────────────────────────────────────────
echo "==> Preparing source in $BUILD_DIR"
if [ -d "$BUILD_DIR/.git" ]; then
    echo "    Existing clone found — fetching latest"
    git -C "$BUILD_DIR" fetch --depth=1 origin main
    git -C "$BUILD_DIR" reset --hard origin/main
    git -C "$BUILD_DIR" clean -fdx -e sdk/
else
    echo "    Cloning $REPO_URL (shallow)"
    rm -rf "$BUILD_DIR"
    git clone --depth=1 "$REPO_URL" "$BUILD_DIR"
fi

# ── 2. Apply patch ──────────────────────────────────────────────────────────
echo "==> Applying patch: $(basename "$PATCH_FILE")"
cd "$BUILD_DIR"
git checkout -- .

# Filter out the submodule hunk (sdk/OpenRTL87X0C) that only exists in dirty
# working trees, not fresh clones.  Then apply with --recount to tolerate
# hunk line-count drift from upstream changes.
filterpatch=$(mktemp)
awk '
    /^diff --git a\/sdk\// { skip=1; next }
    /^diff --git /          { skip=0 }
    !skip                   { print }
' "$PATCH_FILE" > "$filterpatch"

git apply --recount --ignore-whitespace "$filterpatch"
rm -f "$filterpatch"
echo "    Patch applied successfully"

# ── 3. Build Docker image (cached after first run) ──────────────────────────
echo "==> Building Docker image: $DOCKER_IMAGE"
docker build \
    --platform linux/amd64 \
    -t "$DOCKER_IMAGE" \
    --build-arg UID="$(id -u)" \
    --build-arg USERNAME="$(id -un)" \
    docker/

# ── 4. Compile firmware ────────────────────────────────────────────────────
echo "==> Compiling OpenRTL87X0C (version: $VERSION)"
docker run --rm \
    --platform linux/amd64 \
    -v "$BUILD_DIR":/OpenBK7231T_App \
    -e APP_VERSION="$VERSION" \
    -e TARGET_SDKS="OpenRTL87X0C" \
    "$DOCKER_IMAGE"

# ── 5. Copy outputs ────────────────────────────────────────────────────────
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
