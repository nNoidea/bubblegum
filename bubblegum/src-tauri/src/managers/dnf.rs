use std::collections::HashSet;
use super::{run_host_cmd, Package};

/// Get all packages via rpm (works on Fedora, RHEL, etc.)
/// Uses VENDOR to distinguish official, COPR, RPM Fusion and third-party packages.
pub fn get_packages(user_mode: bool) -> Vec<Package> {
    // NOTE: %{FROM_REPO} was removed in RPM 6 (Fedora 43+). Use %{VENDOR} instead.
    let out = run_host_cmd(
        "rpm",
        &[
            "-qa",
            "--queryformat",
            // NAME \t VERSION-RELEASE \t SUMMARY \t GROUP \t SIZE \t VENDOR
            "%{NAME}\\t%{VERSION}-%{RELEASE}\\t%{SUMMARY}\\t%{GROUP}\\t%{SIZE}\\t%{VENDOR}\\n",
        ],
    );

    // In user mode we build a set of packages that own .desktop files –
    // these are the true end-user GUI applications.
    let desktop_pkgs: HashSet<String> = if user_mode {
        build_desktop_package_set_rpm()
    } else {
        HashSet::new()
    };

    out.lines()
        .filter_map(|line| parse_rpm_line(line, user_mode, &desktop_pkgs))
        .collect()
}

/// Returns the names of all RPM packages that own at least one .desktop file.
/// This is the canonical signal that a package is a user-facing GUI application.
fn build_desktop_package_set_rpm() -> HashSet<String> {
    // Step 1: find .desktop files on the host
    let find_out = run_host_cmd(
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

    // Step 2: batch-query rpm to find the owning package for each file.
    // rpm prints errors to stderr (which we ignore) for unowned files, so
    // stdout only contains valid "PKGNAME" lines.
    let mut args = vec!["--queryformat", "%{NAME}\\n", "-qf"];
    args.extend_from_slice(&files);

    let rpm_out = run_host_cmd("rpm", &args);
    rpm_out
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("error:")
                && !l.starts_with("file ")
                && !l.contains("not owned")
        })
        .map(|l| l.to_string())
        .collect()
}

fn parse_rpm_line(line: &str, user_mode: bool, desktop_pkgs: &HashSet<String>) -> Option<Package> {
    let parts: Vec<&str> = line.splitn(6, '\t').collect();
    if parts.len() < 2 {
        return None;
    }

    let name = parts[0].trim().to_string();
    let version = parts[1].trim().to_string();
    let description = parts.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let group = parts.get(3).map(|s| s.trim().to_string()).unwrap_or_default();
    let size: Option<u64> = parts.get(4).and_then(|s| s.trim().parse().ok());
    let vendor = parts.get(5).map(|s| s.trim()).unwrap_or("").to_string();

    let source = classify_rpm_vendor(&vendor);

    if user_mode && !should_show_in_user_mode_rpm(&name, &source, desktop_pkgs) {
        return None;
    }

    Some(Package {
        id: format!("rpm:{}", name),
        name: name.clone(),
        version,
        description,
        manager: "dnf".into(),
        source: Some(source),
        is_user_installed: false,
        icon_name: Some(name.clone()),
        category: Some(map_rpm_group(&group)),
        size_bytes: size,
    })
}

/// Decides whether a package should appear in user mode.
///
/// Rules (in priority order):
/// 1. Always filter out pure lib/dev artifacts by name regardless of source.
/// 2. COPR, RPM Fusion, third-party or locally-installed → show (user explicitly chose these
///    repositories / packages).
/// 3. Official Fedora → show only if the package ships a .desktop file.
fn should_show_in_user_mode_rpm(
    name: &str,
    source: &str,
    desktop_pkgs: &HashSet<String>,
) -> bool {
    let n = name.to_lowercase();

    // ── Rule 1: always hide pure artifacts, even from COPR ────────────────────
    // Development / debug subpackages (e.g. curl-devel, foo-debuginfo)
    for suffix in &["-devel", "-debuginfo", "-debugsource", "-debugdata", "-static"] {
        if n.ends_with(suffix) {
            return false;
        }
    }
    // Library packages (lib* is almost never a user-facing app)
    if n.starts_with("lib") {
        return false;
    }
    // Font packages
    if n.ends_with("-fonts") || n.starts_with("fonts-") || n.ends_with("-font") || n.contains("-fonts-") {
        return false;
    }
    // Kernel & firmware
    if n.starts_with("kernel") || n.contains("firmware") {
        return false;
    }

    // ── Rule 2: non-official sources → always show ────────────────────────────
    // User had to manually enable COPR, RPM Fusion, or add a third-party repo.
    // Locally-installed (no vendor) means they installed an RPM by hand.
    if source.starts_with("copr:")
        || source.starts_with("rpmfusion")
        || source.starts_with("third-party:")
        || source == "locally-installed"
    {
        return true;
    }

    // ── Rule 3: official Fedora → only if it ships a .desktop file ────────────
    // This is the canonical signal that a package is a user-facing GUI app.
    // (firefox, gimp, vlc, thunderbird, etc. all have .desktop files)
    desktop_pkgs.contains(name)
}

/// Classify an RPM VENDOR string (from `%{VENDOR}`) into a source label.
///
/// RPM 6 (Fedora 43+) removed the `FROM_REPO` tag, so we use `VENDOR` instead.
///
/// | VENDOR value                          | source label              |
/// |---------------------------------------|---------------------------|
/// | "Fedora Project"                      | `official`                |
/// | "Fedora Copr - user <owner>"          | `copr:<owner>`            |
/// | "(none)" / empty                      | `locally-installed`       |
/// | anything else                         | `third-party:<vendor>`    |
pub fn classify_rpm_vendor(vendor: &str) -> String {
    let v = vendor.trim();

    if v.is_empty() || v == "(none)" {
        return "locally-installed".into();
    }

    // Official Fedora
    if v == "Fedora Project" || v.starts_with("Red Hat") {
        return "official".into();
    }

    // COPR – vendor format: "Fedora Copr - user <owner>"
    if v.starts_with("Fedora Copr") || v.to_lowercase().contains("copr") {
        // Extract owner name after "user "
        if let Some(idx) = v.find("user ") {
            let owner = v[idx + 5..].trim();
            if !owner.is_empty() {
                return format!("copr:{}", owner);
            }
        }
        return "copr".into();
    }

    // RPM Fusion
    let lower = v.to_lowercase();
    if lower.contains("rpmfusion") || lower.contains("rpm fusion") {
        if lower.contains("nonfree") || lower.contains("non-free") {
            return "rpmfusion-nonfree".into();
        }
        return "rpmfusion-free".into();
    }

    // Well-known third-party vendors
    if lower.contains("google") {
        return "third-party:google".into();
    }
    if lower.contains("microsoft") {
        return "third-party:microsoft".into();
    }
    if lower.contains("nvidia") {
        return "third-party:nvidia".into();
    }
    if lower.contains("brave") {
        return "third-party:brave".into();
    }
    if lower.contains("slack") {
        return "third-party:slack".into();
    }
    if lower.contains("spotify") {
        return "third-party:spotify".into();
    }
    if lower.contains("discord") {
        return "third-party:discord".into();
    }
    if lower.contains("zoom") {
        return "third-party:zoom".into();
    }

    // Generic third-party
    format!("third-party:{}", v)
}

/// Classify an RPM repo-ID string (from `dnf check-update` output) into a source label.
/// Kept for updates parsing where we get repo IDs directly from dnf, not VENDOR.
pub fn classify_rpm_repo(from_repo: &str) -> String {
    let repo = from_repo.trim();

    if repo.is_empty() || repo == "@System" || repo == "@@commandline" {
        return "locally-installed".into();
    }

    // Official Fedora repositories
    if matches!(repo, "fedora" | "updates" | "updates-testing" | "rawhide")
        || repo.starts_with("fedora-")
        || (repo.starts_with("updates") && !repo.contains("rpmfusion"))
        || repo.starts_with("koji-")
    {
        return "official".into();
    }

    // COPR – format: "copr:copr.fedorainfracloud.org:username:reponame"
    if let Some(stripped) = repo.strip_prefix("copr:") {
        let parts: Vec<&str> = repo.splitn(5, ':').collect();
        if parts.len() >= 4 {
            return format!("copr:{}/{}", parts[2], parts[3]);
        }
        return format!("copr:{}", stripped);
    }

    if repo.starts_with("rpmfusion-free") {
        return "rpmfusion-free".into();
    }
    if repo.starts_with("rpmfusion-nonfree") {
        return "rpmfusion-nonfree".into();
    }

    let lower = repo.to_lowercase();
    if lower.contains("google") { return "third-party:google".into(); }
    if lower.contains("vscode") || lower.contains("microsoft") { return "third-party:microsoft".into(); }
    if lower.contains("brave") { return "third-party:brave".into(); }
    if lower.contains("nvidia") { return "third-party:nvidia".into(); }

    format!("third-party:{}", repo)
}

fn map_rpm_group(group: &str) -> String {
    let g = group.to_lowercase();
    if g.contains("internet") || g.contains("network") {
        "Internet".into()
    } else if g.contains("multimedia") || g.contains("sound") || g.contains("video") {
        "Multimedia".into()
    } else if g.contains("office") || g.contains("productivity") {
        "Office".into()
    } else if g.contains("game") {
        "Games".into()
    } else if g.contains("development") {
        "Development".into()
    } else if g.contains("system") {
        "System".into()
    } else {
        "Other".into()
    }
}

/// Get pending updates via dnf
pub fn get_updates() -> Vec<super::Update> {
    let out = run_host_cmd("dnf", &["check-update", "--refresh", "--quiet"]);
    out.lines()
        .filter_map(parse_update_line)
        .collect()
}

fn parse_update_line(line: &str) -> Option<super::Update> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    // Format: name.arch  new_version  repo
    let full_name = parts[0];
    let name = full_name.split('.').next().unwrap_or(full_name).to_string();
    let new_version = parts[1].to_string();
    let from_repo = parts.get(2).copied().unwrap_or("");
    let source = classify_rpm_repo(from_repo);

    Some(super::Update {
        package_id: format!("rpm:{}", name),
        name: name.clone(),
        current_version: "installed".into(),
        new_version,
        manager: "dnf".into(),
        source: Some(source),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn parse_rpm_basic() {
        let desktop = HashSet::new();
        let line = "firefox\t131.0-1.fc43\tMozilla Firefox Web browser\tApplications/Internet\t268435456\tFedora Project";
        let pkg = parse_rpm_line(line, false, &desktop);
        assert!(pkg.is_some());
        let pkg = pkg.unwrap();
        assert_eq!(pkg.name, "firefox");
        assert_eq!(pkg.version, "131.0-1.fc43");
        assert_eq!(pkg.source, Some("official".into()));
    }

    #[test]
    fn parse_rpm_short() {
        let desktop = HashSet::new();
        assert!(parse_rpm_line("too-short", false, &desktop).is_none());
    }

    #[test]
    fn classify_vendor_fedora() {
        assert_eq!(classify_rpm_vendor("Fedora Project"), "official");
    }

    #[test]
    fn classify_vendor_copr() {
        assert_eq!(classify_rpm_vendor("Fedora Copr - user someuser"), "copr:someuser");
    }

    #[test]
    fn classify_vendor_none() {
        assert_eq!(classify_rpm_vendor("(none)"), "locally-installed");
    }

    #[test]
    fn classify_vendor_google() {
        assert_eq!(classify_rpm_vendor("Google LLC"), "third-party:google");
    }

    #[test]
    fn classify_repo_official() {
        assert_eq!(classify_rpm_repo("fedora"), "official");
        assert_eq!(classify_rpm_repo("updates"), "official");
    }

    #[test]
    fn classify_repo_copr() {
        assert_eq!(
            classify_rpm_repo("copr:copr.fedorainfracloud.org:user:repo"),
            "copr:user/repo"
        );
    }

    #[test]
    fn classify_repo_rpmfusion() {
        assert_eq!(classify_rpm_repo("rpmfusion-free-updates"), "rpmfusion-free");
        assert_eq!(classify_rpm_repo("rpmfusion-nonfree"), "rpmfusion-nonfree");
    }

    #[test]
    fn parse_dnf_update() {
        let u = parse_update_line("firefox.x86_64  131.0-1.fc43  updates");
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.name, "firefox");
        assert_eq!(u.new_version, "131.0-1.fc43");
        assert_eq!(u.source, Some("official".into()));
    }

    #[test]
    fn parse_dnf_update_short() {
        assert!(parse_update_line("firefox.x86_64  131.0").is_none());
    }
}
