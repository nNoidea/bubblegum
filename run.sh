#!/usr/bin/env bash
# run.sh — Build Bubblegum inside a throwaway Distrobox container.
#
# Usage:
#   ./run.sh --build      Build release binary  → dist/bubblegum
#   ./run.sh --appimage   Build AppImage         → dist/bubblegum.AppImage
#   ./run.sh --rebuild    Rebuild the base image (after Dockerfile changes)
#   ./run.sh --dev        Enter container interactively (for cargo tauri dev)
#
# Caches (Cargo, npm) are mapped to ./box/ by Distrobox.

set -e

IMAGE="bubblegum-dev"
BOX_NAME="bubblegum-box"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT="$SCRIPT_DIR/bubblegum"
OUT_DIR="$SCRIPT_DIR/dist"
BOX_DIR="$SCRIPT_DIR/box"

# ── 1. Ensure the base image exists ───────────────────────────────────────────
if [[ "${1:-}" == "--rebuild" ]] || ! podman image exists "$IMAGE" 2>/dev/null; then
    echo "🐳  Building base image '$IMAGE' (using Podman)…"
    podman build --tag "$IMAGE" "$SCRIPT_DIR"
    echo "✅  Image ready."
    [[ "${1:-}" == "--rebuild" ]] && exit 0
fi

# ── 2. Ensure Distrobox container is created & running ────────────────────────
mkdir -p "$BOX_DIR" "$OUT_DIR"

if ! distrobox list | grep -q "$BOX_NAME"; then
    echo "📦  Creating Distrobox container '$BOX_NAME'…"
    # --home maps the container's ~ to our local ./box/
    distrobox create --image "$IMAGE" --name "$BOX_NAME" --home "$BOX_DIR" --yes
fi

# Ensure it is running
if ! podman container inspect -f '{{.State.Status}}' "$BOX_NAME" | grep -q "running"; then
    echo "▶️   Starting Distrobox container '$BOX_NAME'…"
    distrobox enter "$BOX_NAME" -- bash -c "echo Container started."
fi

# ── 3. Execute requested command ──────────────────────────────────────────────
MODE="${1:---build}"

# Ensure npm packages are installed first
echo "📦  Installing npm dependencies…"
distrobox enter "$BOX_NAME" -- bash -c "cd '$PROJECT' && npm ci"

case "$MODE" in
  --build)
    echo "🔨  Building Bubblegum (release binary)…"
    distrobox enter "$BOX_NAME" -- bash -c "
      cd '$PROJECT'
      cargo tauri build --no-bundle
      if [[ -f src-tauri/target/release/bubblegum ]]; then
          cp src-tauri/target/release/bubblegum '$OUT_DIR'/bubblegum
          echo '✅  Binary → dist/bubblegum'
      else
          echo '⚠️   Binary not found'
      fi
    "
    ls -lh "$OUT_DIR"
    echo "🛑  Stopping Distrobox container '$BOX_NAME'…"
    distrobox stop "$BOX_NAME" --yes
    ;;

  --appimage)
    echo "📦  Building Bubblegum AppImage…"
    distrobox enter "$BOX_NAME" -- bash -c "
      cd '$PROJECT'
      # Tauri uses appimagetool which might need this inside Distrobox
      export APPIMAGE_EXTRACT_AND_RUN=1
      cargo tauri build --bundles appimage
      if ls src-tauri/target/release/bundle/appimage/*.AppImage >/dev/null 2>&1; then
          cp src-tauri/target/release/bundle/appimage/*.AppImage '$OUT_DIR'/bubblegum.AppImage
          echo '✅  AppImage → dist/bubblegum.AppImage'
      else
          echo '⚠️   No AppImage found'
      fi
    "
    ls -lh "$OUT_DIR"
    echo "🛑  Stopping Distrobox container '$BOX_NAME'…"
    distrobox stop "$BOX_NAME" --yes
    ;;

  --dev)
    echo "🧑‍💻 Entering Dev Mode…"
    distrobox enter "$BOX_NAME" -- bash -c "cd '$PROJECT' && exec bash"
    ;;

  *)
    echo "Usage: $0 [--build | --appimage | --dev | --rebuild]"
    exit 1
    ;;
esac
