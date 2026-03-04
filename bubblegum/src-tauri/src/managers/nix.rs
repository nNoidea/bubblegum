use super::{run_cmd, Package};

/// List installed nix packages (user profile).
///
/// Modern Nix (2.x+) uses flake-based profiles that are INCOMPATIBLE with
/// `nix-env`. Calling `nix-env -q` on such a profile prints an error to
/// stderr and may **hang** while evaluating nixpkgs, which would block the
/// entire packages stream (the `packages::done` event never fires).
///
/// We therefore use ONLY `nix profile list`, which works for both legacy
/// nix-env profiles and modern flake profiles.
pub fn get_packages(_user_mode: bool) -> Vec<Package> {
    // `nix profile list` works on all profile types (legacy + flake).
    let profile_out = run_cmd("nix", &["profile", "list"]);

    if profile_out.trim().is_empty() || profile_out.contains("error") {
        return vec![];
    }

    parse_nix_profile_list(&profile_out)
}


fn parse_nix_profile_list(output: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_version: Option<String> = None;
    let mut current_store: Option<String> = None;

    let flush = |name: String, version: Option<String>, store: Option<String>, packages: &mut Vec<Package>| {
        // If no explicit Version: field, extract from store path:
        // /nix/store/<hash>-<name>-<version>  →  last component after name
        let version = version.unwrap_or_else(|| {
            store
                .as_deref()
                .and_then(|s| s.split_whitespace().next()) // first store path only
                .and_then(|p| p.rsplit('/').next())        // basename: hash-name-ver
                .and_then(|base| {
                    // skip the hash prefix (everything before the first '-' in a
                    // 32-char nix hash suffix, which always contains digits+letters)
                    let after_hash = base.splitn(2, '-').nth(1)?; // "name-ver" or "name"
                    let (_, ver) = split_nix_name_version(after_hash);
                    if ver == "unknown" { None } else { Some(ver) }
                })
                .unwrap_or_else(|| "unknown".into())
        });
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
    };

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Name:") {
            // Flush previous entry first
            if let Some(name) = current_name.take() {
                flush(name, current_version.take(), current_store.take(), &mut packages);
            }
            current_name = Some(line.trim_start_matches("Name:").trim().to_string());
        } else if line.starts_with("Version:") {
            current_version = Some(line.trim_start_matches("Version:").trim().to_string());
        } else if line.starts_with("Store paths:") {
            current_store = Some(line.trim_start_matches("Store paths:").trim().to_string());
        }
    }

    // Flush the last entry
    if let Some(name) = current_name.take() {
        flush(name, current_version.take(), current_store.take(), &mut packages);
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

/// Get pending nix updates.
///
/// `nix-env -u --dry-run` is incompatible with modern flake-based profiles
/// (same as `-q` — it errors and may hang, blocking `updates::done`).
/// `nix profile upgrade` has no `--dry-run` / `--check` flag.
///
/// We return empty here to avoid hanging. The user can still trigger
/// `nix profile upgrade '.*'` from the terminal panel via the update button.
pub fn get_updates() -> Vec<super::Update> {
    vec![]
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
        assert_eq!(pkgs[0].version, "0.2.3"); // extracted from store path
        assert_eq!(pkgs[1].name, "firefox");
        assert_eq!(pkgs[1].version, "131.0");  // extracted from store path
    }

    #[test]
    fn parse_profile_list_real_world_flake() {
        // Real output from `nix profile list` with flake-based installs (no Version: line)
        let output = "\
Name:               bluetuith
Flake attribute:    legacyPackages.x86_64-linux.bluetuith
Original flake URL: flake:nixpkgs
Locked flake URL:   github:NixOS/nixpkgs/ac055f38c798b0d87695240c7b761b82fc7e5bc2?narHash=sha256-xxx
Store paths:        /nix/store/lkg44kimfd48pq34yvsp4084r0qc38d1-bluetuith-0.2.6

Name:               wtype
Flake attribute:    legacyPackages.x86_64-linux.wtype
Original flake URL: flake:nixpkgs
Locked flake URL:   github:NixOS/nixpkgs/d1c15b7d5806069da59e819999d70e1cec0760bf?narHash=sha256-yyy
Store paths:        /nix/store/q7jyal866z2bkmcdlyv14za2y6h0mb1j-wtype-0.4
";
        let pkgs = parse_nix_profile_list(output);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "bluetuith");
        assert_eq!(pkgs[0].version, "0.2.6");
        assert_eq!(pkgs[1].name, "wtype");
        assert_eq!(pkgs[1].version, "0.4");
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

}
