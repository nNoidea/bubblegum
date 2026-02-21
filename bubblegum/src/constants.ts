// ─── Shared manager display config ────────────────────────────────────────────
// Single source of truth for manager colors and emojis across all components.
// Rust detect.rs also defines these per ManagerInfo — this is the frontend
// fallback used before managers are loaded or for static lookups.

export const MANAGER_COLORS: Record<string, string> = {
    apt: "#c4402e",
    dnf: "#3584e4",
    flatpak: "#2190a4",
    pacman: "#1c71d8",
    snap: "#c7561e",
    nix: "#5e81ac",
    brew: "#a86e00",
    cargo: "#b5390e",
    pip: "#3a75b5",
    npm: "#a01010",
};

export const MANAGER_EMOJIS: Record<string, string> = {
    apt: "🔴",
    dnf: "🎩",
    flatpak: "📦",
    pacman: "🏹",
    snap: "🟠",
    nix: "❄️",
    brew: "🍺",
    cargo: "🦀",
    pip: "🐍",
    npm: "📦",
};
