#!/usr/bin/env bash
# run.sh — Launch Bubblegum inside the distrobox container.
# Usage: ./run.sh [--dev | --build | --appimage]
#   --dev       (default) Hot-reload dev mode (cargo tauri dev)
#   --build     Build a release binary (no bundle) and copy it to dist/
#   --appimage  Build a distributable AppImage and copy it to dist/
#
# If the container doesn't exist it is bootstrapped automatically:
#   1. Build base image from box/Dockerfile  (system apt deps — done once)
#   2. Create the distrobox container with home dir = ./box/
#   3. Run box/setup-dev-env.sh inside it   (Rust, nvm, Node, Tauri CLI)

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

# ── Auto-bootstrap ────────────────────────────────────────────────────────────
IMAGE=bubblegum-dev
DOCKERFILE="$BOX_HOME/Dockerfile"

bootstrap_container() {
    # 1. Build the base image only if not already present
    if ! podman image exists "$IMAGE" 2>/dev/null; then
        echo "🐳  Building base image '$IMAGE' from box/Dockerfile…"
        echo "    (This only happens once — subsequent runs skip this step)"
        podman build --tag "$IMAGE" --file "$DOCKERFILE" "$BOX_HOME"
        echo "✅  Image '$IMAGE' built."
    else
        echo "✔   Image '$IMAGE' already present, skipping build."
    fi

    # 2. Create the distrobox container with home = ./box/
    echo "📦  Creating distrobox container '$CONTAINER'…"
    distrobox create \
        --name "$CONTAINER" \
        --image "$IMAGE" \
        --home "$BOX_HOME" \
        --yes

    # 3. First-time distrobox initialisation (installs entrypoint, etc.)
    echo "⚙️   Initialising container (distrobox first-run)…"
    distrobox enter "$CONTAINER" -- true

    # 4. Install Rust / nvm / Node.js / Tauri CLI into box/
    echo "🦀  Running setup-dev-env.sh inside the container…"
    echo "    (Installing Rust, nvm, Node.js, Tauri CLI — grab a coffee ☕)"
    podman exec \
        --user "$CURRENT_USER" \
        --env "HOME=$BOX_HOME" \
        "$CONTAINER" \
        bash "$BOX_HOME/setup-dev-env.sh"

    echo ""
    echo "✅  Container '$CONTAINER' is ready!"
}

# ── Ensure the container exists and is running ────────────────────────────────
STATUS=$(podman inspect --format '{{.State.Status}}' "$CONTAINER" 2>/dev/null || echo "missing")

if [[ "$STATUS" == "missing" ]]; then
    echo "⚠️   Container '$CONTAINER' not found — bootstrapping…"
    echo ""
    bootstrap_container
    STATUS=$(podman inspect --format '{{.State.Status}}' "$CONTAINER" 2>/dev/null || echo "missing")
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

  --appimage)
    echo "📦  Building Bubblegum AppImage…"
    "${EXEC_PREFIX[@]}" "$SHELL_INIT && cargo tauri build --bundles appimage 2>&1"

    OUT_DIR="$(dirname "$0")/dist"
    mkdir -p "$OUT_DIR"

    APPIMAGE=$(find "$PROJECT/src-tauri/target/release/bundle/appimage" -name "*.AppImage" 2>/dev/null | head -1)
    if [[ -n "$APPIMAGE" ]]; then
        cp "$APPIMAGE" "$OUT_DIR/bubblegum.AppImage"
        echo "✅  AppImage → dist/bubblegum.AppImage"
    else
        echo "⚠️  No AppImage found in target/release/bundle/appimage/"
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
