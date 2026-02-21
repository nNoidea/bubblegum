use super::Package;

/// List packages installed via `cargo install` by parsing ~/.cargo/.crates.toml
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return vec![];
    }

    // Primary: parse ~/.cargo/.crates.toml (works without cargo in PATH)
    let crates_path = format!("{}/.cargo/.crates.toml", home);
    if let Ok(content) = std::fs::read_to_string(&crates_path) {
        let pkgs = parse_crates_toml(&content);
        if !pkgs.is_empty() {
            return pkgs;
        }
    }

    // Fallback: cargo install --list (needs cargo in PATH or at ~/.cargo/bin/cargo)
    let cargo_bin = format!("{}/.cargo/bin/cargo", home);
    let out = super::run_cmd(&cargo_bin, &["install", "--list"]);
    if !out.is_empty() {
        return parse_cargo_install_list(&out);
    }
    let out = super::run_cmd("cargo", &["install", "--list"]);
    parse_cargo_install_list(&out)
}

/// Parse ~/.cargo/.crates.toml – no TOML crate needed, format is simple enough.
///
/// Example lines:
///   "jolt 0.3.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["jolt"]
///   "cargo-tauri 2.10.0 (git+https://github.com/tauri-apps/tauri.git#abc)" = ["cargo-tauri"]
fn parse_crates_toml(content: &str) -> Vec<Package> {
    let mut packages = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        // Each package line starts with `"name ver (source)" = [...]`
        if !line.starts_with('"') {
            continue;
        }
        // Find the closing `" =`
        let end = match line.find("\" =") {
            Some(pos) => pos,
            None => continue,
        };
        let key = &line[1..end]; // strip surrounding quotes

        // key = "name version" or "name version (source_url)"
        let mut parts = key.splitn(3, ' ');
        let name = match parts.next() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        let version = match parts.next() {
            Some(v) => v.trim_start_matches('v').to_string(),
            None => continue,
        };
        let source_url = parts.next().unwrap_or("").trim_matches(|c| c == '(' || c == ')');
        let source = classify_cargo_source(source_url);

        packages.push(Package {
            id: format!("cargo:{}", name),
            name: name.clone(),
            version,
            description: None,
            manager: "cargo".into(),
            source: Some(source),
            is_user_installed: true,
            icon_name: None,
            category: Some("Development".into()),
            size_bytes: None,
        });
    }

    packages
}

/// Parse `cargo install --list` output.
///
/// Format:
///   jolt v0.3.0:
///       jolt
///   cargo-tauri v2.10.0 (https://github.com/tauri-apps/tauri.git#abc):
///       cargo-tauri
fn parse_cargo_install_list(output: &str) -> Vec<Package> {
    let mut packages = Vec::new();

    for line in output.lines() {
        // Binary lines are indented; skip them
        if line.starts_with(' ') || line.starts_with('\t') || line.is_empty() {
            continue;
        }

        // Strip trailing colon
        let line = line.trim_end_matches(':');

        // Split into at most 3 parts: name, version, optional_source
        let mut parts = line.splitn(3, ' ');
        let name = match parts.next() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        let version = match parts.next() {
            Some(v) => v.trim_start_matches('v').to_string(),
            None => continue,
        };
        let source_url = parts.next().unwrap_or("").trim_matches(|c| c == '(' || c == ')');
        let source = classify_cargo_source(source_url);

        packages.push(Package {
            id: format!("cargo:{}", name),
            name: name.clone(),
            version,
            description: None,
            manager: "cargo".into(),
            source: Some(source),
            is_user_installed: true,
            icon_name: None,
            category: Some("Development".into()),
            size_bytes: None,
        });
    }

    packages
}

/// Classify a cargo source URL string into a human-readable source label.
fn classify_cargo_source(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() || url.contains("crates.io") || url.starts_with("registry+") {
        return "crates.io".into();
    }
    if url.starts_with("git+") || url.starts_with("https://") || url.starts_with("http://") {
        let clean = url
            .trim_start_matches("git+")
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        // Remove rev/query/fragment for display
        let clean = clean.split('?').next().unwrap_or(clean);
        let clean = clean.split('#').next().unwrap_or(clean);
        let clean = clean.trim_end_matches('/');
        return format!("git:{}", clean);
    }
    if url.starts_with("path+") || url.starts_with("file://") {
        return "local".into();
    }
    format!("registry:{}", url)
}

/// Cargo doesn't have a traditional "check for updates" mechanism via CLI.
pub fn get_updates() -> Vec<super::Update> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_crates_toml_registry() {
        let content = r#"[v1]
"jolt 0.3.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["jolt"]
"cargo-tauri 2.10.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["cargo-tauri"]
"#;
        let pkgs = parse_crates_toml(content);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "jolt");
        assert_eq!(pkgs[0].version, "0.3.0");
        assert_eq!(pkgs[0].source, Some("crates.io".into()));
        assert_eq!(pkgs[1].name, "cargo-tauri");
        assert_eq!(pkgs[1].version, "2.10.0");
    }

    #[test]
    fn parse_crates_toml_git() {
        let content = r#"[v1]
"my-tool 1.0.0 (git+https://github.com/user/my-tool.git#abc123)" = ["my-tool"]
"#;
        let pkgs = parse_crates_toml(content);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "my-tool");
        assert!(pkgs[0].source.as_deref().unwrap().starts_with("git:"));
    }

    #[test]
    fn parse_cargo_install_list_output() {
        let output = "jolt v0.3.0:\n    jolt\ncargo-tauri v2.10.0:\n    cargo-tauri\n";
        let pkgs = parse_cargo_install_list(output);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "jolt");
        assert_eq!(pkgs[0].version, "0.3.0");
        assert_eq!(pkgs[1].name, "cargo-tauri");
        assert_eq!(pkgs[1].version, "2.10.0");
    }

    #[test]
    fn parse_cargo_install_list_empty() {
        assert!(parse_cargo_install_list("").is_empty());
    }

    #[test]
    fn classify_source_crates_io() {
        assert_eq!(classify_cargo_source("registry+https://github.com/rust-lang/crates.io-index"), "crates.io");
    }

    #[test]
    fn classify_source_git() {
        assert_eq!(classify_cargo_source("git+https://github.com/user/repo.git#abc"), "git:github.com/user/repo.git");
    }

    #[test]
    fn classify_source_empty() {
        assert_eq!(classify_cargo_source(""), "crates.io");
    }
}
