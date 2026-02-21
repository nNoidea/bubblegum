#!/usr/bin/env bash
# run.sh — Launch Bubblegum inside the distrobox container.
# Usage: ./run.sh [--dev | --build]
#   --dev    (default) Hot-reload dev mode (cargo tauri dev)
#   --build  Build a release binary/AppImage and copy it out

set -e

CONTAINER=bubblegum
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BOX_HOME="$SCRIPT_DIR/box"
PROJECT="$SCRIPT_DIR/bubblegum"
CURRENT_USER="$(whoami)"
USER_ID=$(id -u)

# ── Detect display ────────────────────────────────────────────────────────────
# Prefer Wayland; fall back to X11.
if [[ -n "$WAYLAND_DISPLAY" ]]; then
    DISPLAY_ARGS=(
        --env "WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
        --env "DISPLAY=${DISPLAY:-:0}"
        --env "GDK_BACKEND=wayland,x11"
    )
else
    DISPLAY_ARGS=(
        --env "DISPLAY=${DISPLAY:-:0}"
        --env "GDK_BACKEND=x11"
    )
fi

# ── Ensure the container is running ──────────────────────────────────────────
STATUS=$(podman inspect --format '{{.State.Status}}' "$CONTAINER" 2>/dev/null || echo "missing")

if [[ "$STATUS" == "missing" ]]; then
    echo "❌  Container '$CONTAINER' not found."
    echo "    Run the bootstrap steps in DEV_ENVIRONMENT.md first."
    exit 1
fi

if [[ "$STATUS" != "running" ]]; then
    echo "▶  Starting container $CONTAINER…"
    podman start "$CONTAINER"
    # Wait for distrobox entrypoint to finish
    for i in $(seq 1 20); do
        sleep 1
        if podman exec "$CONTAINER" test -e /.containersetupdone 2>/dev/null; then
            break
        fi
    done
fi

# ── Common exec preamble ──────────────────────────────────────────────────────
EXEC_PREFIX=(
    podman exec
    --user "$CURRENT_USER"
    --env "HOME=$BOX_HOME"
    "${DISPLAY_ARGS[@]}"
    --env "XDG_RUNTIME_DIR=/run/user/$USER_ID"
    --env "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$USER_ID/bus"
    "$CONTAINER"
    bash -c
)

SHELL_INIT="source \$HOME/.cargo/env && export NVM_DIR=\$HOME/.nvm && . \$NVM_DIR/nvm.sh && cd $PROJECT"

# ── Mode selection ────────────────────────────────────────────────────────────
MODE="${1:---dev}"

case "$MODE" in
  --dev)
    echo "🫧  Launching Bubblegum (dev mode)…"
    "${EXEC_PREFIX[@]}" "$SHELL_INIT && cargo tauri dev"
    ;;

  --build)
    echo "🔨  Building Bubblegum release binary…"
    # cargo tauri build --no-bundle: builds frontend, embeds it, compiles Rust
    # without creating any installer package (deb/rpm/appimage).
    "${EXEC_PREFIX[@]}" "$SHELL_INIT && cargo tauri build --no-bundle 2>&1"

    # Copy binary out to the repo root
    OUT_DIR="$(dirname "$0")/dist"
    mkdir -p "$OUT_DIR"

    if [[ -f "$PROJECT/src-tauri/target/release/bubblegum" ]]; then
        cp "$PROJECT/src-tauri/target/release/bubblegum" "$OUT_DIR/"
        echo "✅  Binary → dist/bubblegum"
    fi

    echo ""
    echo "Artifacts in: $OUT_DIR"
    ls -lh "$OUT_DIR" 2>/dev/null || true
    ;;

  *)
    echo "Usage: $0 [--dev | --build]"
    exit 1
    ;;
esac
