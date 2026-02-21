use super::{cmd_exists, host_cmd_exists, ManagerInfo};

/// Canonical list of all manager IDs that have full backend implementations.
/// This list is the single source of truth — it must stay in sync with
/// the match arms in `lib.rs` (stream_packages / stream_updates).
pub const SUPPORTED_MANAGER_IDS: &[&str] =
    &["apt", "dnf", "flatpak", "pacman", "snap", "nix", "cargo", "npm", "local"];

/// Detect all package managers available on this system.
/// Only managers with full implementations are included here.
pub fn detect_managers() -> Vec<ManagerInfo> {
    vec![
        // ── DNF / RPM ────────────────────────────────────────────────────────
        // "dnf" is the manager ID for all RPM-based distros.
        // The backend uses `rpm -qa` under the hood (via distrobox-host-exec
        // when running inside a container), so a separate "rpm" entry is
        // redundant and would only cause confusion.
        ManagerInfo {
            id: "dnf".into(),
            name: "DNF / RPM".into(),
            available: host_cmd_exists("rpm"), // rpm is the actual query tool
            version: get_host_version("rpm", &["--version"]),
            color: "#51A2DA".into(),
            emoji: "🎩".into(),
        },
        // ── APT ───────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "apt".into(),
            name: "APT".into(),
            available: cmd_exists("apt"),
            version: get_version("apt", &["--version"]),
            color: "#EA4F53".into(),
            emoji: "🔴".into(),
        },
        // ── Flatpak ───────────────────────────────────────────────────────────
        ManagerInfo {
            id: "flatpak".into(),
            name: "Flatpak".into(),
            available: cmd_exists("flatpak"),
            version: get_version("flatpak", &["--version"]),
            color: "#00B4A0".into(),
            emoji: "📦".into(),
        },
        // ── Pacman ────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "pacman".into(),
            name: {
                // Show AUR helper in the display name if detected
                let base = "Pacman";
                match super::pacman::detect_aur_helper() {
                    Some("paru") => format!("{} + paru (AUR)", base),
                    Some("yay")  => format!("{} + yay (AUR)", base),
                    _            => base.to_string(),
                }
            },
            available: cmd_exists("pacman"),
            version: get_version("pacman", &["--version"]),
            color: "#1793D1".into(),
            emoji: "🏹".into(),
        },
        // ── Snap ──────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "snap".into(),
            name: "Snap".into(),
            available: cmd_exists("snap"),
            version: get_version("snap", &["version"]),
            color: "#E95420".into(),
            emoji: "🟠".into(),
        },
        // ── Nix ───────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "nix".into(),
            name: "Nix".into(),
            available: cmd_exists("nix"),
            version: get_version("nix", &["--version"]),
            color: "#7EBAE4".into(),
            emoji: "❄️".into(),
        },
        // ── Cargo ─────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "cargo".into(),
            name: "Cargo".into(),
            available: cmd_exists("cargo"),
            version: get_version("cargo", &["--version"]),
            color: "#CE422B".into(),
            emoji: "🦀".into(),
        },
        // ── npm ───────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "npm".into(),
            name: "npm".into(),
            available: {
                let home = std::env::var("HOME").unwrap_or_default();
                cmd_exists("npm")
                    || std::path::Path::new(&format!("{}/.nvm/versions/node", home)).exists()
            },
            version: get_version("npm", &["--version"]),
            color: "#CB3837".into(),
            emoji: "📦".into(),
        },
        // ── Local ─────────────────────────────────────────────────────────────
        ManagerInfo {
            id: "local".into(),
            name: "Local Apps".into(),
            available: true,
            version: None,
            color: "#999999".into(),
            emoji: "🏠".into(),
        },
    ]
}

fn get_version(cmd: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.lines().next().map(|l| l.trim().to_string())
        })
}

fn get_host_version(cmd: &str, args: &[&str]) -> Option<String> {
    if super::is_in_distrobox() {
        let mut full_args = vec![cmd];
        full_args.extend_from_slice(args);
        std::process::Command::new("distrobox-host-exec")
            .args(&full_args)
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                s.lines().next().map(|l| l.trim().to_string())
            })
    } else {
        get_version(cmd, args)
    }
}
