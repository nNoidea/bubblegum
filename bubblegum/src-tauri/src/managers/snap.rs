use super::{run_cmd, Package};

/// List installed snaps
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    // snap list columns: Name, Version, Rev, Tracking, Publisher, Notes
    let out = run_cmd("snap", &["list", "--color=never"]);

    out.lines()
        .skip(1) // header line
        .filter_map(parse_snap_line)
        .collect()
}

fn parse_snap_line(line: &str) -> Option<Package> {
    // Split by whitespace to handle variable-width columns
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }

    let name = words[0].to_string();
    let version = words[1].to_string();
    // Rev is words[2], tracking is words[3], publisher is words[4]
    let publisher = words.get(4).map(|s| s.to_string());

    // Skip base snaps like "core", "snapd"
    if name == "core" || name == "snapd" || name.starts_with("core") {
        return None;
    }

    Some(Package {
        id: format!("snap:{}", name),
        name: name.clone(),
        version,
        description: None,
        manager: "snap".into(),
        source: publisher,
        is_user_installed: true,
        icon_name: Some(name),
        category: None,
        size_bytes: None,
    })
}

/// Get pending snap refreshes
pub fn get_updates() -> Vec<super::Update> {
    // snap refresh --list shows available updates
    let out = run_cmd("snap", &["refresh", "--list", "--color=never"]);

    out.lines()
        .skip(1) // header
        .filter_map(parse_update_line)
        .collect()
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }
    let name = words[0].to_string();
    let new_version = words[1].to_string();

    Some(super::Update {
        package_id: format!("snap:{}", name),
        name,
        current_version: "installed".into(),
        new_version,
        manager: "snap".into(),
        source: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_snap_normal() {
        let line = "spotify  1.2.3.456  1234  latest/stable  spotify✓  -";
        let pkg = parse_snap_line(line);
        assert!(pkg.is_some());
        let pkg = pkg.unwrap();
        assert_eq!(pkg.name, "spotify");
        assert_eq!(pkg.version, "1.2.3.456");
    }

    #[test]
    fn parse_snap_core_filtered() {
        assert!(parse_snap_line("core22 20240101 1234 latest/stable canonical✓ base").is_none());
        assert!(parse_snap_line("snapd  2.63  12345 latest/stable canonical✓ snapd").is_none());
    }

    #[test]
    fn parse_snap_short() {
        assert!(parse_snap_line("lonely").is_none());
    }

    #[test]
    fn parse_snap_update() {
        let line = "firefox  131.0  5678  latest/stable  mozilla✓  -";
        let u = parse_update_line(line);
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "firefox");
        assert_eq!(u.new_version, "131.0");
    }
}
