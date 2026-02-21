interface SourceBadgeProps {
    source: string;
}

interface SourceStyle {
    label: string;
    color: string;
    bg: string;
}

function classifySource(source: string): SourceStyle {
    const s = source.toLowerCase();

    // Official distro repos
    if (s === "official") {
        return { label: "Official", color: "#4ade80", bg: "#4ade8018" };
    }

    // COPR
    if (s.startsWith("copr:")) {
        const repo = source.slice(5); // strip "copr:"
        return { label: `COPR: ${repo}`, color: "#7A65FF", bg: "#7A65FF18" };
    }

    // RPM Fusion
    if (s === "rpmfusion-free") {
        return { label: "RPM Fusion", color: "#fb923c", bg: "#fb923c18" };
    }
    if (s === "rpmfusion-nonfree") {
        return { label: "RPM Fusion (NF)", color: "#fbbf24", bg: "#fbbf2418" };
    }
    if (s.startsWith("rpmfusion")) {
        return { label: "RPM Fusion", color: "#fb923c", bg: "#fb923c18" };
    }

    // Third-party with known vendor
    if (s.startsWith("third-party:")) {
        const vendor = source.slice(12);
        const vendorMap: Record<string, [string, string]> = {
            google: ["Google", "#4285f4"],
            microsoft: ["Microsoft", "#00a4ef"],
            brave: ["Brave", "#fb542b"],
            slack: ["Slack", "#4a154b"],
            spotify: ["Spotify", "#1db954"],
            dropbox: ["Dropbox", "#0061ff"],
            zoom: ["Zoom", "#2d8cff"],
            discord: ["Discord", "#5865f2"],
            nvidia: ["NVIDIA", "#76b900"],
        };
        const lower = vendor.toLowerCase();
        const match = vendorMap[lower];
        if (match) {
            return { label: match[0], color: match[1], bg: `${match[1]}18` };
        }
        return { label: vendor, color: "#f59e0b", bg: "#f59e0b18" };
    }
    // Generic third-party
    if (s === "third-party") {
        return { label: "Third-party", color: "#f59e0b", bg: "#f59e0b18" };
    }

    // APT areas
    if (s === "community") return { label: "Community", color: "#00DDB8", bg: "#00DDB818" };
    if (s === "proprietary") return { label: "Proprietary", color: "#a855f7", bg: "#a855f718" };
    if (s === "restricted") return { label: "Restricted", color: "#f97316", bg: "#f9731618" };

    // Flatpak remotes
    if (s === "flathub") return { label: "Flathub", color: "#00D4B8", bg: "#00D4B818" };
    if (s.includes("gnome")) return { label: "GNOME", color: "#4a86cf", bg: "#4a86cf18" };
    if (s.includes("kde")) return { label: "KDE", color: "#1d99f3", bg: "#1d99f318" };

    // Cargo sources
    if (s === "crates.io") return { label: "crates.io", color: "#FF5030", bg: "#FF503018" };
    if (s.startsWith("git:")) {
        const url = source.slice(4);
        if (url.includes("github.com")) return { label: "GitHub", color: "#a855f7", bg: "#a855f718" };
        if (url.includes("gitlab.com")) return { label: "GitLab", color: "#e2432a", bg: "#e2432a18" };
        return { label: "Git", color: "#a855f7", bg: "#a855f718" };
    }
    if (s === "local") return { label: "Local", color: "#94a3b8", bg: "#94a3b818" };

    // npm
    if (s === "npm") return { label: "npm", color: "#CB3837", bg: "#CB383718" };

    // Nix channels
    if (s === "nixpkgs") return { label: "nixpkgs", color: "#7EBAE4", bg: "#7EBAE418" };
    if (s === "nixpkgs-unstable") return { label: "nixpkgs-unstable", color: "#5b9bd5", bg: "#5b9bd518" };
    if (s.startsWith("nur")) return { label: "NUR", color: "#7EBAE4", bg: "#7EBAE418" };

    // Pacman / Arch repos
    if (s === "core") return { label: "Core", color: "#1793D1", bg: "#1793D118" };
    if (s === "extra") return { label: "Extra", color: "#2ea7e7", bg: "#2ea7e718" };
    if (s === "multilib") return { label: "Multilib", color: "#3bb8f0", bg: "#3bb8f018" };
    if (s === "aur") return { label: "AUR", color: "#1793D1", bg: "#1793D118" };

    // Locally installed / unknown
    if (s === "locally-installed") return { label: "Local", color: "#94a3b8", bg: "#94a3b818" };

    // Fallback
    return { label: source, color: "#6b7280", bg: "#6b728018" };
}

export function SourceBadge({ source }: SourceBadgeProps) {
    const { label, color, bg } = classifySource(source);

    return (
        <span
            className="text-xs px-1.5 py-0.5 rounded-md font-medium shrink-0 max-w-[140px] truncate"
            style={{
                background: bg,
                color: color,
                border: `1px solid ${color}33`,
            }}
            title={source}
        >
            {label}
        </span>
    );
}
