use super::{run_cmd, Package};

/// List all installed pacman packages
pub fn get_packages(user_mode: bool) -> Vec<Package> {
    // -Qi gives detailed info; -Q just lists. Use -Qi for full info.
    // For speed, parse `pacman -Q` first then enrich with `pacman -Qi` if needed.
    // Here we use `expac` if available, else fall back to `pacman -Q`.
    if super::cmd_exists("expac") {
        get_packages_expac(user_mode)
    } else {
        get_packages_pacman_q(user_mode)
    }
}

fn get_packages_expac(user_mode: bool) -> Vec<Package> {
    // expac -H M '%n\t%v\t%d\t%G\t%m' — name, version, desc, groups, size
    let out = run_cmd("expac", &["-H", "M", "%n\t%v\t%d\t%G\t%m"]);
    let explicit = get_explicit_packages();

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

            Some(Package {
                id: format!("pacman:{}", name),
                name: name.clone(),
                version,
                description,
                manager: "pacman".into(),
                source: Some("pacman".into()),
                is_user_installed: is_explicit,
                icon_name: Some(name),
                category: Some(map_pacman_groups(&groups)),
                size_bytes: size,
            })
        })
        .collect()
}

fn get_packages_pacman_q(user_mode: bool) -> Vec<Package> {
    let out = run_cmd("pacman", &["-Q"]);
    let explicit = get_explicit_packages();

    out.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let name = parts.next()?.trim().to_string();
            let version = parts.next().unwrap_or("").trim().to_string();
            let is_explicit = explicit.contains(&name);

            if user_mode && !is_explicit && is_system_package_pacman(&name, "") {
                return None;
            }

            Some(Package {
                id: format!("pacman:{}", name),
                name: name.clone(),
                version,
                description: None,
                manager: "pacman".into(),
                source: Some("pacman".into()),
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

/// Get pending updates (requires `checkupdates` from `pacman-contrib`)
pub fn get_updates() -> Vec<super::Update> {
    // checkupdates prints: name old_version -> new_version
    let out = if super::cmd_exists("checkupdates") {
        run_cmd("checkupdates", &[])
    } else {
        run_cmd("pacman", &["-Qu"])
    };

    out.lines()
        .filter_map(parse_update_line)
        .collect()
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    // checkupdates: "name old -> new"
    // pacman -Qu: "name old -> new"
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
}
