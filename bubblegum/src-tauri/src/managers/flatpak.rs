use super::{run_cmd, Package};

/// List installed flatpaks
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    let out = run_cmd(
        "flatpak",
        &[
            "list",
            "--columns=application,name,version,branch,description,origin,installation",
        ],
    );

    out.lines()
        .filter_map(parse_flatpak_line)
        .collect()
}

fn parse_flatpak_line(line: &str) -> Option<Package> {
    let parts: Vec<&str> = line.splitn(7, '\t').collect();
    if parts.len() < 2 {
        return None;
    }

    let app_id = parts[0].trim().to_string();
    let name = parts[1].trim().to_string();
    let version = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_else(|| "unknown".into());
    // branch unused currently
    let description = parts.get(4).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let origin = parts.get(5).map(|s| s.trim().to_string());
    // installation: user or system
    let installation = parts.get(6).map(|s| s.trim().to_string()).unwrap_or_default();

    // Icon lives at ~/.local/share/flatpak/exports/share/icons or /var/lib/flatpak/exports/share/icons
    let icon_name = Some(app_id.clone());

    Some(Package {
        id: format!("flatpak:{}", app_id),
        name: if name.is_empty() { app_id.clone() } else { name },
        version,
        description,
        manager: "flatpak".into(),
        source: origin,
        is_user_installed: installation == "user",
        icon_name,
        category: guess_category_from_app_id(&app_id),
        size_bytes: None,
    })
}

fn guess_category_from_app_id(id: &str) -> Option<String> {
    // Flatpak IDs are like org.gnome.Calculator, com.brave.Browser
    // Guess from middle segment
    let lower = id.to_lowercase();
    if lower.contains("browser") || lower.contains("firefox") || lower.contains("thunderbird") || lower.contains("email") {
        Some("Internet".into())
    } else if lower.contains("music") || lower.contains("video") || lower.contains("player") || lower.contains("vlc") || lower.contains("spotify") {
        Some("Multimedia".into())
    } else if lower.contains("code") || lower.contains("studio") || lower.contains("idea") || lower.contains("editor") {
        Some("Development".into())
    } else if lower.contains("office") || lower.contains("writer") || lower.contains("calc") || lower.contains("libreoffice") {
        Some("Office".into())
    } else if lower.contains("game") || lower.contains("steam") {
        Some("Games".into())
    } else {
        None
    }
}

/// Get pending flatpak updates
pub fn get_updates() -> Vec<super::Update> {
    // list remotes that have updates
    let out = run_cmd(
        "flatpak",
        &[
            "remote-ls",
            "--updates",
            "--columns=application,name,version",
        ],
    );

    out.lines()
        .filter_map(parse_update_line)
        .collect()
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    let parts: Vec<&str> = line.splitn(3, '\t').collect();
    if parts.len() < 3 {
        return None;
    }
    let app_id = parts[0].trim().to_string();
    let name = parts[1].trim().to_string();
    let new_version = parts[2].trim().to_string();

    Some(super::Update {
        package_id: format!("flatpak:{}", app_id),
        name,
        current_version: "installed".into(),
        new_version,
        manager: "flatpak".into(),
        source: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flatpak_full() {
        let line = "com.brave.Browser\tBrave Browser\t1.73.104\tstable\tThe web browser from Brave\tflathub\tuser";
        let pkg = parse_flatpak_line(line);
        assert!(pkg.is_some());
        let pkg = pkg.unwrap();
        assert_eq!(pkg.id, "flatpak:com.brave.Browser");
        assert_eq!(pkg.name, "Brave Browser");
        assert_eq!(pkg.version, "1.73.104");
        assert_eq!(pkg.source, Some("flathub".into()));
        assert!(pkg.is_user_installed);
    }

    #[test]
    fn parse_flatpak_system() {
        let line = "org.gnome.Calculator\tCalculator\t46.1\tstable\tPerform calculations\tflathub\tsystem";
        let pkg = parse_flatpak_line(line);
        assert!(pkg.is_some());
        assert!(!pkg.unwrap().is_user_installed);
    }

    #[test]
    fn parse_flatpak_minimal() {
        // At minimum: app_id and name
        let line = "org.test.App\tTest App";
        let pkg = parse_flatpak_line(line);
        assert!(pkg.is_some());
        assert_eq!(pkg.unwrap().name, "Test App");
    }

    #[test]
    fn parse_flatpak_short() {
        assert!(parse_flatpak_line("single-field").is_none());
    }

    #[test]
    fn parse_flatpak_update() {
        let line = "com.brave.Browser\tBrave Browser\t1.74.0";
        let u = parse_update_line(line);
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "Brave Browser");
        assert_eq!(u.new_version, "1.74.0");
    }

    #[test]
    fn category_browser() {
        assert_eq!(guess_category_from_app_id("com.brave.Browser"), Some("Internet".into()));
    }

    #[test]
    fn category_unknown() {
        assert_eq!(guess_category_from_app_id("org.random.Thing"), None);
    }
}
