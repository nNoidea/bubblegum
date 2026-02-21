use super::{run_cmd, Package};
use std::collections::HashMap;

// ─── Repo classification ─────────────────────────────────────────────────────

/// Build a lookup table mapping package names to their sync-database repository.
/// Uses `pacman -Sl` which outputs lines like:  core linux 6.11.1-1
/// Packages not present in any sync repo are foreign (AUR, manual builds, etc.).
fn build_repo_map() -> HashMap<String, String> {
    let out = run_cmd("pacman", &["-Sl"]);
    let mut map = HashMap::new();
    for line in out.lines() {
        // Format: "repo name version [installed]"
        let mut parts = line.splitn(3, ' ');
        if let (Some(repo), Some(name)) = (parts.next(), parts.next()) {
            map.insert(name.trim().to_string(), repo.trim().to_string());
        }
    }
    map
}

/// Get the set of foreign (non-sync) packages — these are AUR or manually built.
/// `pacman -Qm` lists them.
fn get_foreign_packages() -> std::collections::HashSet<String> {
    let out = run_cmd("pacman", &["-Qm"]);
    out.lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

/// Classify a package's source based on its repository in the sync database.
fn classify_source(name: &str, repo_map: &HashMap<String, String>, foreign: &std::collections::HashSet<String>) -> String {
    if foreign.contains(name) {
        return "aur".into();
    }
    match repo_map.get(name).map(|s| s.as_str()) {
        Some("core")      => "core".into(),
        Some("extra")     => "extra".into(),
        Some("multilib")  => "multilib".into(),
        // Arch merged community into extra in 2023, but some repos still use it
        Some("community") => "community".into(),
        Some(other)       => other.to_string(),
        None              => "official".into(), // installed but not in sync = likely just updated
    }
}

/// Detect the available AUR helper binary. Prefers paru, then yay.
pub fn detect_aur_helper() -> Option<&'static str> {
    if super::cmd_exists("paru") {
        Some("paru")
    } else if super::cmd_exists("yay") {
        Some("yay")
    } else {
        None
    }
}

// ─── Package listing ─────────────────────────────────────────────────────────

/// List all installed pacman packages with proper Arch repo classification.
pub fn get_packages(user_mode: bool) -> Vec<Package> {
    let repo_map = build_repo_map();
    let foreign = get_foreign_packages();
    let explicit = get_explicit_packages();

    if super::cmd_exists("expac") {
        get_packages_expac(user_mode, &repo_map, &foreign, &explicit)
    } else {
        get_packages_pacman_q(user_mode, &repo_map, &foreign, &explicit)
    }
}

fn get_packages_expac(
    user_mode: bool,
    repo_map: &HashMap<String, String>,
    foreign: &std::collections::HashSet<String>,
    explicit: &std::collections::HashSet<String>,
) -> Vec<Package> {
    // expac -H M '%n\t%v\t%d\t%G\t%m' — name, version, desc, groups, size
    let out = run_cmd("expac", &["-H", "M", "%n\t%v\t%d\t%G\t%m"]);

    out.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            if parts.len() < 2 {
                return None;
            }
            let name = parts[0].trim().to_string();
            let version = parts[1].trim().to_string();
            let description = parts.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            let groups = parts.get(3).map(|s| s.trim().to_string()).unwrap_or_default();
            let size: Option<u64> = parts.get(4).and_then(|s| s.trim().parse().ok());

            let is_explicit = explicit.contains(&name);

            if user_mode && !is_explicit && is_system_package_pacman(&name, &groups) {
                return None;
            }

            let source = classify_source(&name, repo_map, foreign);

            Some(Package {
                id: format!("pacman:{}", name),
                name: name.clone(),
                version,
                description,
                manager: "pacman".into(),
                source: Some(source),
                is_user_installed: is_explicit,
                icon_name: Some(name),
                category: Some(map_pacman_groups(&groups)),
                size_bytes: size,
            })
        })
        .collect()
}

fn get_packages_pacman_q(
    user_mode: bool,
    repo_map: &HashMap<String, String>,
    foreign: &std::collections::HashSet<String>,
    explicit: &std::collections::HashSet<String>,
) -> Vec<Package> {
    let out = run_cmd("pacman", &["-Q"]);

    out.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let name = parts.next()?.trim().to_string();
            let version = parts.next().unwrap_or("").trim().to_string();
            let is_explicit = explicit.contains(&name);

            if user_mode && !is_explicit && is_system_package_pacman(&name, "") {
                return None;
            }

            let source = classify_source(&name, repo_map, foreign);

            Some(Package {
                id: format!("pacman:{}", name),
                name: name.clone(),
                version,
                description: None,
                manager: "pacman".into(),
                source: Some(source),
                is_user_installed: is_explicit,
                icon_name: Some(name),
                category: None,
                size_bytes: None,
            })
        })
        .collect()
}

fn get_explicit_packages() -> std::collections::HashSet<String> {
    let out = run_cmd("pacman", &["-Qe", "--noconfirm"]);
    out.lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

fn is_system_package_pacman(name: &str, groups: &str) -> bool {
    let n = name.to_lowercase();
    if n.starts_with("lib") || n.ends_with("-devel") {
        return true;
    }
    let g = groups.to_lowercase();
    g.contains("base") || g.contains("base-devel")
}

fn map_pacman_groups(groups: &str) -> String {
    let g = groups.to_lowercase();
    if g.is_empty() {
        "Other".into()
    } else if g.contains("base") {
        "System".into()
    } else if g.contains("gnome") || g.contains("kde") || g.contains("xfce") {
        "Desktop".into()
    } else {
        "Other".into()
    }
}

// ─── Updates ─────────────────────────────────────────────────────────────────

/// Get pending updates from official repos + AUR.
/// Official: `checkupdates` (pacman-contrib) or `pacman -Qu`.
/// AUR: `paru -Qua` or `yay -Qua` if an AUR helper is installed.
pub fn get_updates() -> Vec<super::Update> {
    let mut updates: Vec<super::Update> = Vec::new();

    // ── Official repo updates ────────────────────────────────────────────────
    let repo_map = build_repo_map();
    let out = if super::cmd_exists("checkupdates") {
        run_cmd("checkupdates", &[])
    } else {
        run_cmd("pacman", &["-Qu"])
    };

    for line in out.lines() {
        if let Some(mut u) = parse_update_line(line) {
            // Classify source based on sync DB
            u.source = Some(classify_source(&u.name, &repo_map, &std::collections::HashSet::new()));
            updates.push(u);
        }
    }

    // ── AUR updates ──────────────────────────────────────────────────────────
    if let Some(helper) = detect_aur_helper() {
        // -Qua: query AUR packages with available updates
        let aur_out = run_cmd(helper, &["-Qua"]);
        for line in aur_out.lines() {
            if let Some(mut u) = parse_update_line(line) {
                u.source = Some("aur".into());
                updates.push(u);
            }
        }
    }

    updates
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    // checkupdates / pacman -Qu / yay -Qua: "name old -> new"
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 {
        return None;
    }
    let name = parts[0].trim().to_string();
    let current = parts[1].trim().to_string();
    // parts[2] should be "->"
    let new_version = parts[3].trim().to_string();

    Some(super::Update {
        package_id: format!("pacman:{}", name),
        name,
        current_version: current,
        new_version,
        manager: "pacman".into(),
        source: Some("pacman".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_update_checkupdates() {
        let u = parse_update_line("firefox 130.0-1 -> 131.0-1");
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "firefox");
        assert_eq!(u.current_version, "130.0-1");
        assert_eq!(u.new_version, "131.0-1");
    }

    #[test]
    fn parse_update_short() {
        assert!(parse_update_line("firefox 130.0-1").is_none());
    }

    #[test]
    fn system_package_lib() {
        assert!(is_system_package_pacman("libpng", ""));
    }

    #[test]
    fn system_package_base_group() {
        assert!(is_system_package_pacman("glibc", "base"));
    }

    #[test]
    fn user_package() {
        assert!(!is_system_package_pacman("firefox", "network"));
    }

    #[test]
    fn classify_official_repos() {
        let mut repo_map = HashMap::new();
        repo_map.insert("linux".into(), "core".into());
        repo_map.insert("firefox".into(), "extra".into());
        repo_map.insert("lib32-mesa".into(), "multilib".into());
        let foreign = std::collections::HashSet::new();

        assert_eq!(classify_source("linux", &repo_map, &foreign), "core");
        assert_eq!(classify_source("firefox", &repo_map, &foreign), "extra");
        assert_eq!(classify_source("lib32-mesa", &repo_map, &foreign), "multilib");
    }

    #[test]
    fn classify_aur_package() {
        let repo_map = HashMap::new();
        let mut foreign = std::collections::HashSet::new();
        foreign.insert("yay-bin".to_string());

        assert_eq!(classify_source("yay-bin", &repo_map, &foreign), "aur");
    }
}
