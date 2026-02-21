use super::{run_host_cmd, run_cmd, Package};
use std::fs;
use std::path::Path;

/// List found .desktop apps that are not necessarily managed by a package manager.
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    let mut packages = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    
    let desktop_dirs = vec![
        format!("{}/.local/share/applications", home),
        "/usr/share/applications".to_string(),
        "/usr/local/share/applications".to_string(),
    ];

    // NOTE: We specifically avoid directories managed by other backends:
    // Flatpak: ~/.local/share/flatpak/exports/share/applications, /var/lib/flatpak/exports/share/applications
    // Snap: /var/lib/snapd/desktop/applications
    // This reduces duplicates for apps already tracked by bubblegum.

    // Check for Flatpak and Snap directories to potentially filter them if they are already handled,
    // but the user wants "found .desktop apps". To avoid too much noise, we'll focus on the main ones.
    // If we want to be very thorough, we could add more, but let's stick to these.

    let mut seen_ids = std::collections::HashSet::new();

    for dir in desktop_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "desktop") {
                    let id = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if id.is_empty() || seen_ids.contains(&id) {
                        continue;
                    }

                    if is_file_owned(&path) {
                        continue;
                    }

                    if let Some(pkg) = parse_desktop_file(&path, &id) {
                        seen_ids.insert(id);
                        packages.push(pkg);
                    }
                }
            }
        }
    }

    packages
}

fn parse_desktop_file(path: &Path, id: &str) -> Option<Package> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut icon = None;
    let mut comment = None;
    let mut categories = None;
    let mut version = "1.0".to_string(); // Desktop files often don't have a version for the app itself
    let mut no_display = false;

    // Only parse the [Desktop Entry] section
    let mut in_section = false;
    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_section = true;
            continue;
        } else if line.starts_with('[') && line.ends_with(']') {
            in_section = false;
            continue;
        }

        if in_section {
            if let Some(val) = line.strip_prefix("Name=") {
                if !line.contains('[') { // Avoid translated names for simplicity or take the first one
                    name = Some(val.to_string());
                } else if name.is_none() {
                    name = Some(val.to_string());
                }
            } else if let Some(val) = line.strip_prefix("Icon=") {
                icon = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("Comment=") {
                if !line.contains('[') {
                    comment = Some(val.to_string());
                }
            } else if let Some(val) = line.strip_prefix("Categories=") {
                categories = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("Version=") {
                version = val.to_string();
            } else if let Some(val) = line.strip_prefix("NoDisplay=") {
                if val.to_lowercase() == "true" {
                    no_display = true;
                }
            }
        }
    }

    // Skip hidden apps (like helper tools)
    if no_display {
        return None;
    }

    let name = name.or_else(|| Some(id.to_string()))?;
    
    Some(Package {
        id: format!("local:{}", id),
        name,
        version,
        description: comment,
        manager: "local".into(),
        source: Some("Local .desktop file".into()),
        is_user_installed: path.to_string_lossy().contains("/.local/"),
        icon_name: icon,
        category: categories.and_then(|c| c.split(';').next().map(|s| s.to_string())),
        size_bytes: None,
    })
}

fn is_file_owned(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // 1. Check RPM (Fedora, RedHat, openSUSE)
    // rpm -qf /path/to/file returns 0 if owned
    if super::host_cmd_exists("rpm") {
        let out = run_host_cmd("rpm", &["-qf", &path_str]);
        if !out.is_empty() && !out.contains("is not owned") {
            return true;
        }
    }

    // 2. Check Pacman (Arch)
    // pacman -Qo /path/to/file returns 0 if owned
    if super::cmd_exists("pacman") {
        let out = run_cmd("pacman", &["-Qo", &path_str]);
        if !out.is_empty() && !out.contains("No package owns") {
            return true;
        }
    }

    // 3. Check DPKG (Ubuntu, Debian)
    // dpkg -S /path/to/file returns 0 if owned
    if super::cmd_exists("dpkg") {
        let out = run_cmd("dpkg", &["-S", &path_str]);
        if !out.is_empty() && !out.contains("no path found matching") {
            return true;
        }
    }

    false
}
