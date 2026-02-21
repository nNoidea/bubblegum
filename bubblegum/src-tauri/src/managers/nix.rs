use super::{run_cmd, Package};

/// List installed nix packages (user profile)
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    // nix-env -q lists user-installed packages
    // Format: attribute_name-version
    let out = run_cmd("nix-env", &["-q", "--no-name"]);

    // Try nix profile list for Nix 2.x flakes-based installs
    let profile_out = run_cmd(
        "nix",
        &["profile", "list"],
    );

    let mut packages: Vec<Package> = Vec::new();

    if !profile_out.trim().is_empty() && !profile_out.contains("error") {
        packages.extend(parse_nix_profile_list(&profile_out));
    }

    if !out.trim().is_empty() && !out.contains("error") {
        packages.extend(parse_nix_env_q(&out));
    }

    // Deduplicate by id
    packages.dedup_by(|a, b| a.id == b.id);
    packages
}

fn parse_nix_env_q(output: &str) -> Vec<Package> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let name = line.trim().to_string();
            // Extract version: last hyphen-delimited segment that starts with digit
            let (pkg_name, version) = split_nix_name_version(&name);

            Package {
                id: format!("nix:{}", pkg_name),
                name: pkg_name.clone(),
                version,
                description: None,
                manager: "nix".into(),
                source: Some("nixpkgs".into()),
                is_user_installed: true,
                icon_name: Some(pkg_name),
                category: None,
                size_bytes: None,
            }
        })
        .collect()
}

fn parse_nix_profile_list(output: &str) -> Vec<Package> {
    // Nix 2.x: each entry has multiple lines, starts with "Index:"
    let mut packages = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_version: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Name:") {
            current_name = Some(line.trim_start_matches("Name:").trim().to_string());
        } else if line.starts_with("Version:") {
            current_version = Some(line.trim_start_matches("Version:").trim().to_string());
        } else if line.is_empty() {
            if let Some(name) = current_name.take() {
                let version = current_version.take().unwrap_or_else(|| "unknown".into());
                packages.push(Package {
                    id: format!("nix:{}", name),
                    name: name.clone(),
                    version,
                    description: None,
                    manager: "nix".into(),
                    source: Some("nixpkgs".into()),
                    is_user_installed: true,
                    icon_name: Some(name),
                    category: None,
                    size_bytes: None,
                });
            }
        }
    }

    // Flush the last entry if the output doesn't end with a blank line
    if let Some(name) = current_name.take() {
        let version = current_version.take().unwrap_or_else(|| "unknown".into());
        packages.push(Package {
            id: format!("nix:{}", name),
            name: name.clone(),
            version,
            description: None,
            manager: "nix".into(),
            source: Some("nixpkgs".into()),
            is_user_installed: true,
            icon_name: Some(name),
            category: None,
            size_bytes: None,
        });
    }

    packages
}

fn split_nix_name_version(full: &str) -> (String, String) {
    // e.g., "firefox-120.0.1" or "hello-2.12"
    let parts: Vec<&str> = full.rsplitn(2, '-').collect();
    if parts.len() == 2 && parts[0].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        (parts[1].to_string(), parts[0].to_string())
    } else {
        (full.to_string(), "unknown".to_string())
    }
}

/// Get pending nix updates
pub fn get_updates() -> Vec<super::Update> {
    // nix-env -u --dry-run shows what would be updated
    let out = run_cmd("nix-env", &["-u", "--dry-run"]);
    out.lines()
        .filter(|l| l.contains("->") || l.starts_with("upgrading"))
        .filter_map(parse_update_line)
        .collect()
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    // "upgrading 'hello-2.10' to 'hello-2.12'"
    let stripped = line.trim_start_matches("upgrading").trim().to_string();
    let parts: Vec<&str> = stripped.split('\'').collect();
    // parts: ["", "hello-2.10", " to ", "hello-2.12", ""]
    if parts.len() < 4 {
        return None;
    }
    let old_full = parts[1];
    let new_full = parts.get(3).unwrap_or(&"");

    let (name, current_version) = split_nix_name_version(old_full);
    let (_n, new_version) = split_nix_name_version(new_full);

    Some(super::Update {
        package_id: format!("nix:{}", name),
        name,
        current_version,
        new_version,
        manager: "nix".into(),
        source: Some("nixpkgs".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── nix profile list (Nix 2.x) ──────────────────────────────────────────

    #[test]
    fn parse_profile_list_basic() {
        let output = "\
Name:               bluetuith
Flake reference:    nixpkgs#bluetuith
Store paths:        /nix/store/abc-bluetuith-0.2.3

Name:               firefox
Flake reference:    nixpkgs#firefox
Store paths:        /nix/store/xyz-firefox-131.0
";
        let pkgs = parse_nix_profile_list(output);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "bluetuith");
        assert_eq!(pkgs[1].name, "firefox");
    }

    #[test]
    fn parse_profile_list_no_trailing_blank() {
        // Some nix versions don't end with a blank line
        let output = "\
Name:               htop
Version:            3.3.0
Store paths:        /nix/store/abc-htop-3.3.0";
        let pkgs = parse_nix_profile_list(output);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "htop");
        assert_eq!(pkgs[0].version, "3.3.0");
    }

    #[test]
    fn parse_profile_list_with_version() {
        let output = "\
Name:               hello
Version:            2.12
Flake reference:    nixpkgs#hello
Store paths:        /nix/store/abc-hello-2.12

Name:               cowsay
Version:            3.7.0
Flake reference:    nixpkgs#cowsay
Store paths:        /nix/store/xyz-cowsay-3.7.0

";
        let pkgs = parse_nix_profile_list(output);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "hello");
        assert_eq!(pkgs[0].version, "2.12");
        assert_eq!(pkgs[1].name, "cowsay");
        assert_eq!(pkgs[1].version, "3.7.0");
    }

    #[test]
    fn parse_profile_list_empty() {
        let pkgs = parse_nix_profile_list("");
        assert!(pkgs.is_empty());
    }

    // ── nix-env -q ───────────────────────────────────────────────────────────

    #[test]
    fn parse_nix_env_q_basic() {
        let output = "firefox-120.0.1\nhello-2.12\n";
        let pkgs = parse_nix_env_q(output);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "firefox");
        assert_eq!(pkgs[0].version, "120.0.1");
        assert_eq!(pkgs[1].name, "hello");
        assert_eq!(pkgs[1].version, "2.12");
    }

    #[test]
    fn parse_nix_env_q_no_version() {
        let output = "some-tool\n";
        let pkgs = parse_nix_env_q(output);
        assert_eq!(pkgs.len(), 1);
        // "some-tool" → name="some", version="tool" (splits on last hyphen where
        // the segment starts with a digit... but "tool" doesn't start with digit)
        // So it should be name="some-tool", version="unknown"
        assert_eq!(pkgs[0].name, "some-tool");
        assert_eq!(pkgs[0].version, "unknown");
    }

    // ── split_nix_name_version ───────────────────────────────────────────────

    #[test]
    fn split_name_version_normal() {
        let (name, ver) = split_nix_name_version("firefox-120.0.1");
        assert_eq!(name, "firefox");
        assert_eq!(ver, "120.0.1");
    }

    #[test]
    fn split_name_version_hyphenated_name() {
        let (name, ver) = split_nix_name_version("nix-output-monitor-2.1.2");
        assert_eq!(name, "nix-output-monitor");
        assert_eq!(ver, "2.1.2");
    }

    #[test]
    fn split_name_version_no_version() {
        let (name, ver) = split_nix_name_version("bluetuith");
        assert_eq!(name, "bluetuith");
        assert_eq!(ver, "unknown");
    }

    // ── update parsing ───────────────────────────────────────────────────────

    #[test]
    fn parse_update_basic() {
        let u = parse_update_line("upgrading 'hello-2.10' to 'hello-2.12'");
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "hello");
        assert_eq!(u.current_version, "2.10");
        assert_eq!(u.new_version, "2.12");
    }

    #[test]
    fn parse_update_malformed() {
        assert!(parse_update_line("some random text").is_none());
    }
}
