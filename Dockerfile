# bubblegum build image
# Fully self-contained: Ubuntu 24.04 + Tauri system deps + Rust + Node.js + Tauri CLI.
# Build once with: podman build -t bubblegum-dev .
# Then use run.sh via Distrobox to spin up a container for each build.

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# ── System dependencies & Node.js ─────────────────────────────────────────────
RUN apt-get update -qq && \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        curl \
        wget \
        file \
        libssl-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.1-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libjavascriptcoregtk-4.1-dev \
        libsoup-3.0-dev \
        libglib2.0-dev \
        libdbus-1-dev \
        patchelf \
        xdg-utils \
        ca-certificates && \
    # Install Node.js LTS (22.x) system-wide via NodeSource
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# ── Rust ──────────────────────────────────────────────────────────────────────
# Install Rust system-wide so it's available regardless of the user's home dir
ENV RUSTUP_HOME=/opt/rust
ENV CARGO_HOME=/opt/cargo
ENV PATH="/opt/cargo/bin:$PATH"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --default-toolchain stable && \
    rustup target add x86_64-unknown-linux-gnu && \
    chmod -R a+rwX /opt/cargo /opt/rust

# ── Tauri CLI ─────────────────────────────────────────────────────────────────
RUN cargo install tauri-cli --version "^2" --locked && \
    chmod -R a+rwX /opt/cargo /opt/rust

WORKDIR /src
