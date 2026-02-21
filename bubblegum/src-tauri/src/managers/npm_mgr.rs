use super::Package;

/// List globally installed npm packages.
/// Handles nvm-based installs by looking inside ~/.nvm/versions/node/
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    let home = std::env::var("HOME").unwrap_or_default();

    let npm = find_npm_binary(&home);
    let npm_ref = npm.as_deref().unwrap_or("npm");

    // `npm list -g --depth=0` lists top-level global packages
    let out = super::run_cmd(npm_ref, &["list", "-g", "--depth=0"]);
    if out.is_empty() {
        return vec![];
    }

    parse_npm_list(&out)
}

/// Find the npm binary, preferring nvm's latest installed version over system npm.
fn find_npm_binary(home: &str) -> Option<String> {
    // If npm is already in PATH, use it
    if super::cmd_exists("npm") {
        return Some("npm".to_string());
    }

    // Look inside ~/.nvm/versions/node/<version>/bin/npm
    let nvm_dir = format!("{}/.nvm/versions/node", home);
    let mut versions: Vec<String> = std::fs::read_dir(&nvm_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();

    // Sort descending to get latest first (v24.x > v22.x, etc.)
    versions.sort_by(|a, b| b.cmp(a));

    for ver in versions {
        let npm_path = format!("{}/{}/bin/npm", nvm_dir, ver);
        if std::path::Path::new(&npm_path).exists() {
            return Some(npm_path);
        }
    }

    None
}

/// Parse `npm list -g --depth=0` output.
///
/// Format:
///   /home/user/.nvm/versions/node/v24.13.1/lib
///   ├── npm@11.8.0
///   ├── typescript@5.7.2
///   └── pnpm@9.1.0
fn parse_npm_list(output: &str) -> Vec<Package> {
    let mut packages = Vec::new();

    for line in output.lines().skip(1) {
        // Strip tree drawing characters
        let clean = line
            .replace("├── ", "")
            .replace("└── ", "")
            .replace("│   ", "")
            .replace("─", "")
            .replace("├", "")
            .replace("└", "");
        let clean = clean.trim();

        if clean.is_empty() || clean.starts_with("(empty)") {
            continue;
        }

        // Take only the first token (rest might be "(deduped)" etc.)
        let token = clean.split_whitespace().next().unwrap_or(clean);

        if let Some((name, version)) = split_npm_name_version(token) {
            // Skip npm itself
            if name == "npm" {
                continue;
            }

            packages.push(Package {
                id: format!("npm:{}", name),
                name: name.clone(),
                version,
                description: None,
                manager: "npm".into(),
                source: Some("npm".into()),
                is_user_installed: true,
                icon_name: None,
                category: Some("Development".into()),
                size_bytes: None,
            });
        }
    }

    packages
}

/// Split a `name@version` token, handling scoped packages like `@scope/pkg@1.0.0`.
fn split_npm_name_version(s: &str) -> Option<(String, String)> {
    if let Some(rest) = s.strip_prefix('@') {
        // Scoped: @scope/name@version
        // Strip the leading @, find the next @
        let at_pos = rest.find('@')?;
        let name = format!("@{}", &rest[..at_pos]);
        let version = rest[at_pos + 1..].to_string();
        return Some((name, version));
    }
    // Regular: name@version
    let at_pos = s.rfind('@')?;
    let name = s[..at_pos].to_string();
    let version = s[at_pos + 1..].to_string();
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some((name, version))
}

/// npm packages don't have a built-in outdated-check command that's straightforward.
/// `npm outdated -g` could be used but it exits non-zero when there are updates.
pub fn get_updates() -> Vec<super::Update> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_npm_list_basic() {
        let output = "/home/user/.nvm/versions/node/v24.13.1/lib\n\
├── npm@11.8.0\n\
├── typescript@5.7.2\n\
└── pnpm@9.1.0\n";
        let pkgs = parse_npm_list(output);
        // npm is skipped
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "typescript");
        assert_eq!(pkgs[0].version, "5.7.2");
        assert_eq!(pkgs[1].name, "pnpm");
        assert_eq!(pkgs[1].version, "9.1.0");
    }

    #[test]
    fn parse_npm_list_empty() {
        let output = "/home/user/.nvm/versions/node/v24.13.1/lib\n└── (empty)\n";
        let pkgs = parse_npm_list(output);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn split_npm_scoped() {
        let (name, ver) = split_npm_name_version("@angular/cli@18.2.0").unwrap();
        assert_eq!(name, "@angular/cli");
        assert_eq!(ver, "18.2.0");
    }

    #[test]
    fn split_npm_regular() {
        let (name, ver) = split_npm_name_version("typescript@5.7.2").unwrap();
        assert_eq!(name, "typescript");
        assert_eq!(ver, "5.7.2");
    }

    #[test]
    fn split_npm_no_at() {
        assert!(split_npm_name_version("noversion").is_none());
    }
}
