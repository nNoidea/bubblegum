# 🫧 Bubblegum

![Bubblegum](img/app.png)

A unified package manager GUI for Linux. Bubblegum gives you a single dashboard to browse, search, update, and uninstall packages across every package manager on your system.

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue)
![License](https://img.shields.io/badge/license-AGPL--3.0-red)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)

## Features

- **Unified view** — See every installed package in one place, grouped by source
- **Multi-manager support** — apt, dnf, flatpak, pacman, snap, nix, cargo, npm
- **Arch Linux & AUR** — Classifies packages by repo (core/extra/multilib/AUR); uses paru/yay for AUR updates
- **Live streaming** — Package lists and updates stream in progressively (no frozen UI)
- **Bulk updates** — Update all packages per manager with a single click; firmware updates via fwupd
- **Batch uninstall** — Stage packages for removal and review the command before running it
- **Icon detection** — Automatically resolves application icons from `.desktop` files and icon themes
- **Distrobox-aware** — Works transparently inside distrobox containers by querying the host
- **Built-in terminal** — Watch command output in a live terminal panel without leaving the app

## Supported Package Managers

| Manager | Packages | Updates | Uninstall | Notes                                |
| ------- | :------: | :-----: | :-------: | ------------------------------------ |
| APT     |    ✅    |   ✅    |    ✅     | Debian / Ubuntu                      |
| DNF/RPM |    ✅    |   ✅    |    ✅     | Fedora / RHEL                        |
| Pacman  |    ✅    |   ✅    |    ✅     | Arch Linux (core / extra / multilib) |
| AUR     |    ✅    |   ✅    |    ✅     | Via paru or yay                      |
| Flatpak |    ✅    |   ✅    |    ✅     |                                      |
| Snap    |    ✅    |   ✅    |    ✅     |                                      |
| Nix     |    ✅    |   ✅    |    ✅     |                                      |
| Cargo   |    ✅    |    —    |     —     | Read-only listing                    |
| npm     |    ✅    |    —    |     —     | Read-only listing                    |

## Tech Stack

- **Backend** — Rust + [Tauri v2](https://v2.tauri.app/)
- **Frontend** — React 19 · TypeScript · TailwindCSS v4 · Zustand · React Router
- **Privilege escalation** — `pkexec` (Polkit) for operations that need root
- **Build tool** — Vite 7

## Screenshots

<!-- Add screenshots here -->

## Getting Started

### Prerequisites

- Linux (x86_64)
- [Podman](https://podman.io/) and [Distrobox](https://distrobox.it/) (for the dev environment)
- Or: Rust 1.70+, Node.js 20+, and the [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/#linux)

### Quick Start (Distrobox)

The recommended development workflow uses a distrobox container so your host stays clean.

```bash
# 1. Create the container (Ubuntu 24.04, named "bubblegum")
distrobox create --name bubblegum --image ubuntu:24.04 --home ./box

# 2. Enter the container and run the setup script
distrobox enter bubblegum
./box/setup-dev-env.sh

# 3. Back on the host — launch dev mode
./run.sh --dev
```

### Manual Setup (no container)

If you already have the Tauri v2 prerequisites installed:

```bash
cd bubblegum
npm install
cargo tauri dev
```

### Building a Release Binary

```bash
./run.sh --build
# Output: dist/bubblegum
```

## Project Structure

```
bubblegum/                  ← Tauri project root
├── src/                    ← React frontend
│   ├── pages/              ← Overview · Search · Updates
│   ├── components/         ← PackageCard · TerminalPanel · …
│   ├── store.ts            ← Zustand state management
│   └── types.ts            ← Shared TypeScript types
├── src-tauri/              ← Rust backend
│   ├── src/lib.rs          ← Tauri commands (stream, update, uninstall, …)
│   └── src/managers/       ← Per-manager modules (apt, dnf, flatpak, …)
├── public/                 ← Static assets
box/                        ← Distrobox container home (gitignored)
run.sh                      ← Dev launcher script
```

## Development

| Command             | Description                        |
| ------------------- | ---------------------------------- |
| `./run.sh --dev`    | Hot-reload dev mode via distrobox  |
| `./run.sh --build`  | Release build via distrobox        |
| `cargo tauri dev`   | Dev mode (inside container/manual) |
| `cargo tauri build` | Release build (inside container)   |
| `npm run dev`       | Frontend-only dev server           |

## Security

- All subprocess calls use argument arrays — no shell interpretation
- stdin is nulled on every spawned process to prevent interactive prompts
- Package names are validated to prevent argument injection (`--` separators, reject names starting with `-`)
- CSP restricts frontend to `self` origins only
- Tauri capabilities are locked to the minimum required set
- Privilege escalation uses `pkexec` — the app never handles passwords directly

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

## License

[AGPL-3.0](LICENSE)
