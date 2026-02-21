use std::collections::HashSet;
use super::{run_cmd, Package};

/// List installed packages via dpkg
pub fn get_packages(user_mode: bool) -> Vec<Package> {
    let out = run_cmd(
        "dpkg-query",
        &[
            "-W",
            "-f=${Package}\\t${Version}\\t${Status}\\t${binary:Summary}\\t${Installed-Size}\\t${Section}\\n",
        ],
    );

    // In user mode, build a set of packages that own .desktop files.
    let desktop_pkgs: HashSet<String> = if user_mode {
        build_desktop_package_set_apt()
    } else {
        HashSet::new()
    };

    out.lines()
        .filter_map(|line| parse_dpkg_line(line, user_mode, &desktop_pkgs))
        .collect()
}

/// Returns the names of all dpkg packages that own at least one .desktop file.
fn build_desktop_package_set_apt() -> HashSet<String> {
    // Step 1: find .desktop files
    let find_out = run_cmd(
        "find",
        &[
            "/usr/share/applications",
            "/usr/local/share/applications",
            "-maxdepth", "3",
            "-name", "*.desktop",
            "-type", "f",
        ],
    );

    let files: Vec<&str> = find_out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && l.ends_with(".desktop"))
        .collect();

    if files.is_empty() {
        return HashSet::new();
    }

    // Step 2: dpkg -S file1 file2 ... → "pkgname: /path/to/file"
    let mut args = vec!["-S"];
    args.extend_from_slice(&files);

    let dpkg_out = run_cmd("dpkg", &args);
    dpkg_out
        .lines()
        .filter_map(|line| {
            // "package: /path" or "package, other-pkg: /path"
            let colon = line.find(':')?;
            let pkgs_part = &line[..colon];
            // dpkg -S can return "pkg1, pkg2: file" for diversion cases; take the first
            let pkg = pkgs_part.split(',').next()?.trim().to_string();
            if pkg.is_empty() { None } else { Some(pkg) }
        })
        .collect()
}

fn parse_dpkg_line(line: &str, user_mode: bool, desktop_pkgs: &HashSet<String>) -> Option<Package> {
    let parts: Vec<&str> = line.splitn(6, '\t').collect();
    if parts.len() < 3 {
        return None;
    }

    let name = parts[0].trim().to_string();
    let version = parts[1].trim().to_string();
    let status = parts[2].trim();

    // Only include installed packages
    if !status.contains("install ok installed") {
        return None;
    }

    let description = parts.get(3).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    // size is in KB from dpkg
    let size: Option<u64> = parts.get(4).and_then(|s| s.trim().parse::<u64>().ok()).map(|kb| kb * 1024);
    let section = parts.get(5).map(|s| s.trim().to_string()).unwrap_or_default();

    if user_mode && !should_show_in_user_mode_apt(&name, &section, desktop_pkgs) {
        return None;
    }

    let source = classify_apt_source(&section);

    Some(Package {
        id: format!("apt:{}", name),
        name: name.clone(),
        version,
        description,
        manager: "apt".into(),
        source: Some(source),
        is_user_installed: false,
        icon_name: Some(name.clone()),
        category: Some(map_apt_section(&section)),
        size_bytes: size,
    })
}

/// Decides whether an APT package should appear in user mode.
///
/// Rules:
/// 1. Always hide lib*, *-dev, *-dbg, kernel, firmware artifacts.
/// 2. Show if the package ships a .desktop file (canonical GUI-app signal).
/// 3. Show if the section looks like a third-party/PPA package.
fn should_show_in_user_mode_apt(
    name: &str,
    section: &str,
    desktop_pkgs: &HashSet<String>,
) -> bool {
    let n = name.to_lowercase();

    // ── Rule 1: always hide pure artifacts ────────────────────────────────────
    if n.starts_with("lib") {
        return false;
    }
    for suffix in &["-dev", "-dbg", "-dbgsym", "-debug"] {
        if n.ends_with(suffix) {
            return false;
        }
    }
    if n.contains("linux-image") || n.contains("linux-headers") || n.contains("firmware") {
        return false;
    }
    // Font packages
    if n.starts_with("fonts-") || n.ends_with("-fonts") {
        return false;
    }

    let s = section.to_lowercase();
    // Section-based hard excludes (dpkg section categories that are never user apps)
    let area = s.split('/').next_back().unwrap_or(&s);
    if matches!(area, "libs" | "libdevel" | "debug" | "oldlibs" | "kernel" | "devel") {
        return false;
    }

    // ── Rule 2: show if it has a .desktop file ────────────────────────────────
    if desktop_pkgs.contains(name) {
        return true;
    }

    // ── Rule 3: show third-party packages ────────────────────────────────────
    // dpkg section can contain hints like "downloaded" or vendor-specific areas
    let section_area = s.split('/').next().unwrap_or(&s);
    if !matches!(
        section_area,
        "main" | "universe" | "multiverse" | "restricted"
            | "admin" | "cli-mono" | "comm" | "database" | "debug" | "devel"
            | "doc" | "editors" | "education" | "electronics" | "embedded"
            | "fonts" | "games" | "gnome" | "graphics" | "hamradio" | "haskell"
            | "httpd" | "interpreters" | "introspection" | "java" | "kde"
            | "kernel" | "libdevel" | "libs" | "lisp" | "localization" | "mail"
            | "math" | "misc" | "net" | "news" | "ocaml" | "oldlibs" | "otherosfs"
            | "perl" | "php" | "python" | "ruby" | "rust" | "science" | "shells"
            | "sound" | "tasks" | "tex" | "text" | "utils" | "vcs" | "video"
            | "web" | "x11" | "xfce" | "zope"
    ) {
        // Unknown section area = likely third-party PPA or manually installed
        return true;
    }

    false
}

/// Classify an APT package source from its dpkg section string.
///
/// Ubuntu/Debian sections have the format `[area/]category`:
///   - `main/utils`        → `official`
///   - `universe/web`      → `community`
///   - `multiverse/games`  → `proprietary`
///   - `restricted/net`    → `restricted`
///   - `utils`             → `official` (bare section = main)
///
/// Third-party debs (Google Chrome, Slack, etc.) often use non-standard sections;
/// we fall through to `third-party` for anything we don't recognize.
fn classify_apt_source(section: &str) -> String {
    let s = section.to_lowercase();
    // Extract the area (the part before '/', if any)
    let area = s.split('/').next().unwrap_or(&s);
    match area {
        "main" => "official".into(),
        "universe" => "community".into(),
        "multiverse" => "proprietary".into(),
        "restricted" => "restricted".into(),
        // No area prefix → interpreted as main
        "admin" | "cli-mono" | "comm" | "database" | "debug" | "devel"
        | "doc" | "editors" | "education" | "electronics" | "embedded"
        | "fonts" | "games" | "gnome" | "graphics" | "hamradio" | "haskell"
        | "httpd" | "interpreters" | "introspection" | "java" | "kde"
        | "kernel" | "libdevel" | "libs" | "lisp" | "localization" | "mail"
        | "math" | "misc" | "net" | "news" | "ocaml" | "oldlibs" | "otherosfs"
        | "perl" | "php" | "python" | "ruby" | "rust" | "science" | "shells"
        | "sound" | "tasks" | "tex" | "text" | "utils" | "vcs" | "video"
        | "web" | "x11" | "xfce" | "zope" => "official".into(),
        _ => {
            // Heuristic: known third-party patterns in the section string
            if s.contains("google") {
                "third-party:google".into()
            } else if s.contains("microsoft") || s.contains("vscode") {
                "third-party:microsoft".into()
            } else if s.contains("slack") {
                "third-party:slack".into()
            } else if s.contains("brave") {
                "third-party:brave".into()
            } else {
                format!("third-party:{}", area)
            }
        }
    }
}

fn map_apt_section(section: &str) -> String {
    let s = section.to_lowercase();
    // strip optional universe/multiverse prefix
    let core = s.split('/').next_back().unwrap_or(&s);
    match core {
        "web" | "net" | "network" => "Internet",
        "graphics" | "video" | "sound" | "multimedia" => "Multimedia",
        "office" | "editors" | "text" => "Office",
        "games" => "Games",
        "devel" | "perl" | "python" | "ruby" | "java" => "Development",
        "admin" | "utils" | "misc" | "shell" => "Utilities",
        "science" | "math" | "education" => "Education",
        _ => "Other",
    }
    .to_string()
}

/// Get pending upgradable packages
pub fn get_updates() -> Vec<super::Update> {
    // Run apt update first silently, then list upgradable
    let out = run_cmd("apt", &["list", "--upgradable", "--quiet"]);
    out.lines()
        .filter_map(parse_upgradable_line)
        .collect()
}

fn parse_upgradable_line(line: &str) -> Option<super::Update> {
    // Format: name/source version arch [upgradable from: old_version]
    if !line.contains("upgradable from:") {
        return None;
    }
    let slash = line.find('/')?;
    let name = line[..slash].trim().to_string();

    let after_slash = &line[slash + 1..];
    let parts: Vec<&str> = after_slash.splitn(3, ' ').collect();
    let new_version = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

    let old_version = line
        .find("upgradable from: ")
        .map(|i| line[i + 17..].trim_end_matches(']').trim().to_string())
        .unwrap_or_default();

    Some(super::Update {
        package_id: format!("apt:{}", name),
        name,
        current_version: old_version,
        new_version,
        manager: "apt".into(),
        source: Some("apt".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn parse_dpkg_installed() {
        let desktop = HashSet::new();
        let line = "curl\t8.5.0-2ubuntu10\tinstall ok installed\tcommand line tool\t436\tnet";
        let pkg = parse_dpkg_line(line, false, &desktop);
        assert!(pkg.is_some());
        let pkg = pkg.unwrap();
        assert_eq!(pkg.name, "curl");
        assert_eq!(pkg.version, "8.5.0-2ubuntu10");
        assert_eq!(pkg.description, Some("command line tool".into()));
    }

    #[test]
    fn parse_dpkg_not_installed() {
        let desktop = HashSet::new();
        let line = "curl\t8.5.0\tdeinstall ok config-files\tCLI tool\t436\tnet";
        let pkg = parse_dpkg_line(line, false, &desktop);
        assert!(pkg.is_none());
    }

    #[test]
    fn parse_dpkg_short_line() {
        let desktop = HashSet::new();
        let pkg = parse_dpkg_line("too\tshort", false, &desktop);
        assert!(pkg.is_none());
    }

    #[test]
    fn parse_upgradable() {
        let line = "curl/jammy-updates 8.6.0-1 amd64 [upgradable from: 8.5.0-2]";
        let u = parse_upgradable_line(line);
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "curl");
        assert_eq!(u.new_version, "8.6.0-1");
        assert_eq!(u.current_version, "8.5.0-2");
    }

    #[test]
    fn parse_upgradable_not_upgradable() {
        assert!(parse_upgradable_line("Listing...").is_none());
    }

    #[test]
    fn classify_source_main() {
        assert_eq!(classify_apt_source("main/net"), "official");
    }

    #[test]
    fn classify_source_universe() {
        assert_eq!(classify_apt_source("universe/web"), "community");
    }

    #[test]
    fn classify_source_thirdparty() {
        assert_eq!(classify_apt_source("google-chrome"), "third-party:google");
    }
}
