// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod managers;

use managers::{ManagerInfo, Package, Update};
use serde::Serialize;
use tauri::Emitter;

// ─── Event payloads ──────────────────────────────────────────────────────────

/// Each stream call gets a unique request_id. The frontend ignores
/// events whose request_id doesn't match the latest call, which
/// prevents stale in-flight threads from polluting a newer stream.

#[derive(Clone, Serialize)]
struct PackagesChunk {
    request_id: String,
    manager: String,
    packages: Vec<Package>,
}

#[derive(Clone, Serialize)]
struct PackagesDone {
    request_id: String,
}

#[derive(Clone, Serialize)]
struct UpdatesChunk {
    request_id: String,
    manager: String,
    updates: Vec<Update>,
}

#[derive(Clone, Serialize)]
struct UpdatesDone {
    request_id: String,
}

#[derive(Clone, Serialize)]
struct TerminalLine {
    request_id: String,
    text: String,
    is_stderr: bool,
}

#[derive(Clone, Serialize)]
struct TerminalDone {
    request_id: String,
    exit_code: i32,
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Return all detected package managers on this system
#[tauri::command]
fn get_managers() -> Vec<ManagerInfo> {
    managers::detect::detect_managers()
}

/// Stream packages asynchronously — returns immediately.
/// Emits `packages::chunk` per manager, then `packages::done`.
/// Every event carries `request_id` so the frontend can discard
/// events from superseded calls (prevents duplicates on fast navigation).
#[tauri::command]
async fn stream_packages(
    request_id: String,
    manager_id: String,
    user_mode: bool,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use managers::detect::SUPPORTED_MANAGER_IDS;

    let ids: Vec<String> = if manager_id == "all" {
        SUPPORTED_MANAGER_IDS.iter().map(|s| s.to_string()).collect()
    } else {
        vec![manager_id.clone()]
    };

    let total = ids.len();
    let done_count = Arc::new(Mutex::new(0usize));

    for id in ids {
        let app = app.clone();
        let rid = request_id.clone();
        let done_count = done_count.clone();

        thread::spawn(move || {
            let pkgs: Vec<Package> = match id.as_str() {
                "apt"     => managers::apt::get_packages(user_mode),
                "dnf"     => managers::dnf::get_packages(user_mode),
                "flatpak" => managers::flatpak::get_packages(user_mode),
                "pacman"  => managers::pacman::get_packages(user_mode),
                "snap"    => managers::snap::get_packages(user_mode),
                "nix"     => managers::nix::get_packages(user_mode),
                "cargo"   => managers::cargo_mgr::get_packages(user_mode),
                "npm"     => managers::npm_mgr::get_packages(user_mode),
                "local"   => managers::local::get_packages(user_mode),
                _         => vec![],
            };

            // Always emit chunk (even empty) so frontend marks manager done.
            let _ = app.emit("packages::chunk", PackagesChunk {
                request_id: rid.clone(),
                manager: id,
                packages: pkgs,
            });

            let mut count = done_count.lock().unwrap();
            *count += 1;
            if *count == total {
                let _ = app.emit("packages::done", PackagesDone {
                    request_id: rid,
                });
            }
        });
    }

    Ok(())
}

/// Stream updates asynchronously — returns immediately.
/// Emits `updates::chunk` per manager, then `updates::done`.
/// Uses the same request_id pattern as stream_packages to prevent stale events.
#[tauri::command]
async fn stream_updates(request_id: String, app: tauri::AppHandle) -> Result<(), String> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let detected = managers::detect::detect_managers();
    let available: Vec<String> = detected
        .into_iter()
        .filter(|m| m.available)
        .map(|m| m.id)
        .collect();

    let total = available.len();
    if total == 0 {
        let _ = app.emit("updates::done", UpdatesDone { request_id });
        return Ok(());
    }

    let done_count = Arc::new(Mutex::new(0usize));

    for id in available {
        let app = app.clone();
        let rid = request_id.clone();
        let done_count = done_count.clone();

        thread::spawn(move || {
            let updates: Vec<Update> = match id.as_str() {
                "apt"     => managers::apt::get_updates(),
                "dnf"     => managers::dnf::get_updates(),
                "flatpak" => managers::flatpak::get_updates(),
                "pacman"  => managers::pacman::get_updates(),
                "snap"    => managers::snap::get_updates(),
                "nix"     => managers::nix::get_updates(),
                "cargo"   => managers::cargo_mgr::get_updates(),
                "npm"     => managers::npm_mgr::get_updates(),
                _         => vec![],
            };

            let _ = app.emit("updates::chunk", UpdatesChunk {
                request_id: rid.clone(),
                manager: id,
                updates,
            });

            let mut count = done_count.lock().unwrap();
            *count += 1;
            if *count == total {
                let _ = app.emit("updates::done", UpdatesDone { request_id: rid });
            }
        });
    }

    Ok(())
}

/// Find an icon for a package and return it as a base64 data URL.
/// Falls back to None if not found.
///
/// Strategy:
/// 1. Build name variants (lowercase, dotted short, hyphenated first word, stripped prefixes).
/// 2. Search .desktop files across host (/run/host/) and container paths to discover the
///    canonical Icon= name, using both exact filename matching and Exec=/Name= content scanning.
/// 3. Search all XDG icon theme directories including /run/host/ (the distrobox host mount).
#[tauri::command]
fn find_icon(name: String) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    // Reject names with path traversal or directory separators
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }

    let home = std::env::var("HOME").unwrap_or_default();

    // Build name variants to try
    let mut variants: Vec<String> = Vec::new();
    variants.push(name.clone());
    let name_lower = name.to_lowercase();
    if name_lower != name {
        variants.push(name_lower.clone());
    }
    // Dotted flatpak ID: org.mozilla.firefox → firefox
    if name.contains('.') {
        if let Some(short) = name.rsplit('.').next() {
            let s = short.to_lowercase();
            if !variants.contains(&s) {
                variants.push(s);
            }
        }
    }
    // Hyphenated: obs-studio → first word "obs", underscore form obs_studio
    if name_lower.contains('-') {
        let first = name_lower.split('-').next().unwrap_or(&name_lower).to_string();
        if first.len() > 2 && !variants.contains(&first) {
            variants.push(first);
        }
        let us = name_lower.replace('-', "_");
        if !variants.contains(&us) {
            variants.push(us);
        }
    }
    // Strip common packaging prefixes
    for prefix in &["lib", "python3-", "python-", "perl-"] {
        if let Some(stripped) = name_lower.strip_prefix(prefix) {
            let s = stripped.to_string();
            if s.len() > 2 && !variants.contains(&s) {
                variants.push(s);
            }
        }
    }

    // Step 1: Discover the canonical icon name from .desktop files
    let icon_name = find_icon_name_from_desktops(&name, &variants, &home);

    // Step 2: If the icon name is an absolute path, load it directly
    if let Some(ref iname) = icon_name {
        let p = std::path::Path::new(iname.as_str());
        if p.is_absolute() && p.exists() && is_safe_icon_path(iname, &home) {
            if let Ok(bytes) = std::fs::read(p) {
                let mime = if iname.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
                return Some(format!("data:{};base64,{}", mime, STANDARD.encode(&bytes)));
            }
        }
    }

    // Step 3: Build ordered search name list (desktop-discovered name first)
    let mut search_names: Vec<String> = Vec::new();
    if let Some(ref iname) = icon_name {
        if !search_names.contains(iname) {
            search_names.push(iname.clone());
        }
        // Short component of dotted icon names
        if iname.contains('.') {
            if let Some(short) = iname.rsplit('.').next() {
                let s = short.to_lowercase();
                if !search_names.contains(&s) {
                    search_names.push(s);
                }
            }
        }
        let iname_lower = iname.to_lowercase();
        if !search_names.contains(&iname_lower) {
            search_names.push(iname_lower);
        }
    }
    for v in &variants {
        if !search_names.contains(v) {
            search_names.push(v.clone());
        }
    }

    // Icon base directories — includes /run/host/ which is the distrobox host mount
    let icon_bases: Vec<String> = vec![
        format!("{}/.local/share/flatpak/exports/share/icons", home),
        "/var/lib/flatpak/exports/share/icons".to_string(),
        "/run/host/usr/share/icons".to_string(),
        "/run/host/usr/local/share/icons".to_string(),
        "/usr/share/icons".to_string(),
        "/usr/local/share/icons".to_string(),
        format!("{}/.local/share/icons", home),
        "/var/lib/snapd/desktop/icons".to_string(),
    ];

    let pixmap_bases: Vec<String> = vec![
        "/run/host/usr/share/pixmaps".to_string(),
        "/run/host/usr/local/share/pixmaps".to_string(),
        "/usr/share/pixmaps".to_string(),
        "/usr/local/share/pixmaps".to_string(),
    ];

    // Icon theme search order; empty string means search directly under the base
    let themes: &[&str] = &["hicolor", "Adwaita", "AdwaitaLegacy", "gnome", "Papirus", "breeze", ""];
    // Size search order — prefer larger/scalable
    let sizes: &[&str] = &[
        "scalable", "256x256", "512x512", "128x128", "1024x1024",
        "64x64", "48x48", "32x32", "256x256@2", "128x128@2",
    ];

    for n in &search_names {
        if n.is_empty() {
            continue;
        }

        for base in &icon_bases {
            for theme in themes {
                let theme_base = if theme.is_empty() {
                    base.clone()
                } else {
                    format!("{}/{}", base, theme)
                };
                for sz in sizes {
                    for ext in &["svg", "png"] {
                        let p = format!("{}/{}/apps/{}.{}", theme_base, sz, n, ext);
                        if let Some(data) = try_read_icon_path(&p) {
                            return Some(data);
                        }
                    }
                }
            }
            // Direct under base (some exports lay icons flat)
            for ext in &["png", "svg"] {
                if let Some(data) = try_read_icon_path(&format!("{}/{}.{}", base, n, ext)) {
                    return Some(data);
                }
            }
        }

        for base in &pixmap_bases {
            for ext in &["png", "svg"] {
                if let Some(data) = try_read_icon_path(&format!("{}/{}.{}", base, n, ext)) {
                    return Some(data);
                }
            }
        }

        // Snap
        for path in &[
            format!("/snap/{}/current/meta/gui/icon.png", n),
            format!("/snap/{}/current/meta/gui/icon.svg", n),
            format!("/snap/{}/current/meta/gui/{}.png", n, n),
        ] {
            if let Some(data) = try_read_icon_path(path) {
                return Some(data);
            }
        }
    }

    None
}

fn try_read_icon_path(path: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    // Only allow known image extensions
    if !path.ends_with(".png") && !path.ends_with(".svg") {
        return None;
    }
    let p = std::path::Path::new(path);
    if p.exists() {
        if let Ok(bytes) = std::fs::read(p) {
            let mime = if path.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
            return Some(format!("data:{};base64,{}", mime, STANDARD.encode(&bytes)));
        }
    }
    None
}

/// Check that an icon path is under a known directory and has a valid extension.
/// Prevents arbitrary file reads via crafted .desktop Icon= values.
fn is_safe_icon_path(path: &str, home: &str) -> bool {
    let p = std::path::Path::new(path);

    // Must have a valid image extension
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "png" && ext != "svg" {
        return false;
    }

    // Reject ".." path components
    for component in p.components() {
        if component == std::path::Component::ParentDir {
            return false;
        }
    }

    // Must be under a known icon directory
    let home_icons = format!("{}/.local/share/icons", home);
    let home_flatpak = format!("{}/.local/share/flatpak", home);
    let allowed: &[&str] = &[
        &home_icons,
        &home_flatpak,
        "/usr/share/icons",
        "/usr/share/pixmaps",
        "/usr/local/share/icons",
        "/usr/local/share/pixmaps",
        "/run/host/usr/share/icons",
        "/run/host/usr/share/pixmaps",
        "/run/host/usr/local/share/icons",
        "/run/host/usr/local/share/pixmaps",
        "/var/lib/flatpak/exports/share/icons",
        "/run/host/var/lib/flatpak/exports/share/icons",
        "/var/lib/snapd/desktop/icons",
        "/snap/",
    ];

    allowed.iter().any(|prefix| path.starts_with(prefix))
}

/// Search .desktop files (host + container) to find the canonical Icon= name for a package.
///
/// Pass 1 – exact filename: tries {name}.desktop and {variant}.desktop in all dirs.
/// Pass 2 – content scan: reads all .desktop files and matches on:
///   - filename stem containing the package name
///   - Exec= binary basename matching any name variant
///   - Name= field matching any name variant (spaces → hyphens normalised)
fn find_icon_name_from_desktops(name: &str, variants: &[String], home: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    let desktop_dirs: Vec<String> = vec![
        "/run/host/usr/share/applications".to_string(),
        "/run/host/usr/local/share/applications".to_string(),
        "/usr/share/applications".to_string(),
        "/usr/local/share/applications".to_string(),
        "/var/lib/flatpak/exports/share/applications".to_string(),
        "/run/host/var/lib/flatpak/exports/share/applications".to_string(),
        format!("{}/.local/share/applications", home),
        format!("{}/.local/share/flatpak/exports/share/applications", home),
        "/var/lib/snapd/desktop/applications".to_string(),
    ];

    // Pass 1: exact filename match
    for dir in &desktop_dirs {
        let candidates = std::iter::once(name).chain(variants.iter().map(|s| s.as_str()));
        for candidate in candidates {
            let path = format!("{}/{}.desktop", dir, candidate);
            if let Some(icon) = parse_desktop_file_icon(&path) {
                return Some(icon);
            }
        }
    }

    // Pass 2: full directory scan with content matching and scoring
    let mut best_match: Option<(String, i32)> = None;

    for dir in &desktop_dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if !fname.ends_with(".desktop") {
                continue;
            }
            let stem = fname.trim_end_matches(".desktop");
            let fpath = entry.path().to_string_lossy().to_string();

            let content = match std::fs::read_to_string(&fpath) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut file_icon: Option<String> = None;
            let mut score = 0;

            if stem == name_lower {
                score = score.max(100);
            } else if variants.iter().any(|v| v == stem) {
                score = score.max(90);
            } else if stem.starts_with(&name_lower) {
                score = score.max(80);
            } else if stem.contains(&name_lower) {
                score = score.max(70);
            } else if variants.iter().any(|v| !v.is_empty() && stem.contains(v.as_str())) {
                score = score.max(10);
            }

            for line in content.lines() {
                if let Some(stripped) = line.strip_prefix("Icon=") {
                    let icon = stripped.trim();
                    if !icon.is_empty() {
                        file_icon = Some(icon.to_string());
                    }
                } else if let Some(stripped) = line.strip_prefix("Exec=") {
                    let exec_val = stripped.trim();
                    let exec_bin = exec_val
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .split('/')
                        .next_back()
                        .unwrap_or("")
                        .to_lowercase();
                    if !exec_bin.is_empty() {
                        if exec_bin == name_lower {
                            score = score.max(95);
                        } else if variants.iter().any(|v| v == &exec_bin) {
                            score = score.max(85);
                        } else if exec_bin.len() > 2 && name_lower.starts_with(&exec_bin) {
                            score = score.max(75);
                        }
                    }
                } else if line.starts_with("Name=") && !line.starts_with("Name[") {
                    let app_name_raw = line[5..].trim().to_lowercase();
                    let app_name_norm = app_name_raw.replace(' ', "-");
                    if app_name_norm == name_lower || app_name_raw == name_lower {
                        score = score.max(90);
                    } else if variants.iter().any(|v| v == &app_name_norm || v == &app_name_raw) {
                        score = score.max(80);
                    }
                }
            }

            if score > 0 {
                if let Some(icon) = file_icon {
                    if best_match.is_none() || score > best_match.as_ref().unwrap().1 {
                        best_match = Some((icon, score));
                    }
                }
            }
        }
    }

    best_match.map(|(icon, _)| icon)
}

fn parse_desktop_file_icon(path: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("Icon=") {
            let icon = stripped.trim();
            if !icon.is_empty() {
                return Some(icon.to_string());
            }
        }
    }
    None
}

/// Return a single shell command that uninstalls multiple packages for one manager.
/// All package names are appended to a single invocation so polkit only asks once.
#[tauri::command]
fn get_batch_uninstall_command(manager_id: String, pkg_ids: Vec<String>) -> String {
    let names: Vec<String> = pkg_ids
        .iter()
        .filter_map(|id| {
            let name = id.split_once(':')
                .map(|(_, n)| n)
                .unwrap_or(id)
                .to_string();
            if name.is_empty() || name.starts_with('-') { None } else { Some(name) }
        })
        .collect();
    let joined = names.join(" ");

    match manager_id.as_str() {
        "apt"     => format!("pkexec apt-get remove -y -- {}", joined),
        "dnf"     => format!("pkexec dnf remove -y -- {}", joined),
        "flatpak" => format!("flatpak uninstall -y -- {}", joined),
        "pacman"  => {
            // AUR helpers can uninstall any pacman package (official + AUR)
            match managers::pacman::detect_aur_helper() {
                Some(helper) => format!("{} -R --noconfirm -- {}", helper, joined),
                None         => format!("pkexec pacman -R --noconfirm -- {}", joined),
            }
        }
        "snap"    => format!("pkexec snap remove {}", joined),
        "nix"     => format!("nix-env -e {}", joined),
        "cargo"   => format!("cargo uninstall {}", joined),
        "npm"     => format!("npm uninstall -g {}", joined),
        "local"   => format!("# Manual uninstallation required for: {}", joined),
        _         => format!("# Unknown manager: {}", manager_id),
    }
}

/// Return the shell command that would update all packages for a given manager.
/// Does NOT execute the command — used by the terminal panel.
#[tauri::command]
fn get_update_command(manager_id: String) -> String {
    match manager_id.as_str() {
        "apt"     => "pkexec apt-get upgrade -y".into(),
        "dnf"     => "pkexec dnf upgrade --refresh -y".into(),
        "flatpak" => "flatpak update -y".into(),
        "pacman"  => {
            // If an AUR helper is available, use it to update everything
            // (official repos + AUR in one shot)
            match managers::pacman::detect_aur_helper() {
                Some(helper) => format!("{} -Syu --noconfirm", helper),
                None         => "pkexec pacman -Syu --noconfirm".into(),
            }
        }
        "snap"    => "pkexec snap refresh".into(),
        "nix"     => "nix profile upgrade '.*'".into(),
        _         => format!("# Unknown manager: {}", manager_id),
    }
}

// ─── Safe process streaming ──────────────────────────────────────────────────

/// Spawn a process with proper argument separation (no shell interpretation),
/// stream stdout/stderr as `terminal::line` events, and return the exit code.
/// **Must** be called from a background thread (uses blocking I/O).
fn stream_process(
    prog: &str,
    args: &[&str],
    request_id: &str,
    app: &tauri::AppHandle,
) -> i32 {
    use std::io::{BufRead, BufReader};
    use std::process::{Command as SysCommand, Stdio};
    use std::thread;

    let mut child = match SysCommand::new(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("LD_LIBRARY_PATH")
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit(
                "terminal::line",
                TerminalLine {
                    request_id: request_id.to_string(),
                    text: format!("spawn error: {}", e),
                    is_stderr: true,
                },
            );
            return -1;
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let app_out = app.clone();
    let app_err = app.clone();
    let rid_out = request_id.to_string();
    let rid_err = request_id.to_string();

    let t_out = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = app_out.emit(
                "terminal::line",
                TerminalLine { request_id: rid_out.clone(), text: line, is_stderr: false },
            );
        }
    });

    let t_err = thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = app_err.emit(
                "terminal::line",
                TerminalLine { request_id: rid_err.clone(), text: line, is_stderr: true },
            );
        }
    });

    t_out.join().ok();
    t_err.join().ok();
    child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
}

/// Execute a package manager update safely. Arguments are never shell-interpreted.
/// Handles distrobox routing for host-targeted managers (dnf).
#[tauri::command]
async fn execute_update(
    request_id: String,
    manager_id: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::thread;

    // Validate manager_id against known list
    let valid = ["apt", "dnf", "flatpak", "pacman", "snap", "nix", "local"];
    if !valid.contains(&manager_id.as_str()) {
        return Err(format!("Unknown manager: {}", manager_id));
    }

    thread::spawn(move || {
        let is_distrobox = managers::is_in_distrobox();
        // dnf targets the host system (detected via host_cmd_exists in detect.rs)
        let host_mgr = manager_id == "dnf";

        // Base command: first element is the program, rest are args
        // For pacman: use AUR helper (paru/yay) if available — it handles
        // both official repos and AUR in a single invocation, no pkexec needed.
        let base: Vec<&str> = match manager_id.as_str() {
            "apt"     => vec!["pkexec", "apt-get", "upgrade", "-y"],
            "dnf"     => vec!["pkexec", "dnf", "upgrade", "--refresh", "-y"],
            "flatpak" => vec!["flatpak", "update", "-y"],
            "pacman"  => {
                match managers::pacman::detect_aur_helper() {
                    Some("paru") => vec!["paru", "-Syu", "--noconfirm"],
                    Some("yay")  => vec!["yay", "-Syu", "--noconfirm"],
                    _            => vec!["pkexec", "pacman", "-Syu", "--noconfirm"],
                }
            }
            "snap"    => vec!["pkexec", "snap", "refresh"],
            "nix"     => vec!["nix", "profile", "upgrade", ".*"],
            "local"   => {
                let _ = app.emit("terminal::line", TerminalLine {
                    request_id: request_id.clone(),
                    text: "Local apps do not support updates via Bubblegum.".into(),
                    is_stderr: false,
                });
                let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code: 0 });
                return;
            }
            _         => unreachable!(),
        };

        let (prog, args): (&str, Vec<&str>) = if is_distrobox && host_mgr {
            ("distrobox-host-exec", base)
        } else {
            (base[0], base[1..].to_vec())
        };

        let exit_code = stream_process(prog, &args, &request_id, &app);
        let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code });
    });

    Ok(())
}

/// Execute a batch uninstall safely. Package names are passed as separate process
/// arguments — never interpolated into a shell string.
/// Handles distrobox routing for host-targeted managers (dnf).
#[tauri::command]
async fn execute_batch_uninstall(
    request_id: String,
    manager_id: String,
    pkg_ids: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::thread;

    let valid = ["apt", "dnf", "flatpak", "pacman", "snap", "nix", "cargo", "npm", "local"];
    if !valid.contains(&manager_id.as_str()) {
        return Err(format!("Unknown manager: {}", manager_id));
    }
    if pkg_ids.is_empty() {
        return Err("No packages specified".into());
    }

    thread::spawn(move || {
        let is_distrobox = managers::is_in_distrobox();
        let host_mgr = manager_id == "dnf";

        // Extract clean package names (strip "manager:" prefix)
        // Reject names that look like flags to prevent argument injection.
        let names: Vec<String> = pkg_ids
            .iter()
            .filter_map(|id| {
                let name = id.split_once(':')
                    .map(|(_, n)| n)
                    .unwrap_or(id)
                    .to_string();
                // Block empty names and names that start with '-' (flag injection)
                if name.is_empty() || name.starts_with('-') {
                    None
                } else {
                    Some(name)
                }
            })
            .collect();

        if names.is_empty() {
            let _ = app.emit("terminal::line", TerminalLine {
                request_id: request_id.clone(),
                text: "Error: no valid package names after filtering".into(),
                is_stderr: true,
            });
            let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code: 1 });
            return;
        }

        // Base command prefix (before package names)
        let mut base: Vec<&str> = match manager_id.as_str() {
            "apt"     => vec!["pkexec", "apt-get", "remove", "-y"],
            "dnf"     => vec!["pkexec", "dnf", "remove", "-y"],
            "flatpak" => vec!["flatpak", "uninstall", "-y"],
            "pacman"  => {
                match managers::pacman::detect_aur_helper() {
                    Some("paru") => vec!["paru", "-R", "--noconfirm"],
                    Some("yay")  => vec!["yay", "-R", "--noconfirm"],
                    _            => vec!["pkexec", "pacman", "-R", "--noconfirm"],
                }
            }
            "snap"    => vec!["pkexec", "snap", "remove"],
            "nix"     => vec!["nix-env", "-e"],
            "cargo"   => vec!["cargo", "uninstall"],
            "npm"     => vec!["npm", "uninstall", "-g"],
            "local"   => {
                let _ = app.emit("terminal::line", TerminalLine {
                    request_id: request_id.clone(),
                    text: "This app is not installed via a package manager, you need to figure out self how to uninstall it.".into(),
                    is_stderr: false,
                });
                let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code: 0 });
                return;
            }
            _         => unreachable!(),
        };

        // Append package names as separate args (safe — no shell interpretation)
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        base.extend_from_slice(&name_refs);

        let (prog, args): (&str, Vec<&str>) = if is_distrobox && host_mgr {
            ("distrobox-host-exec", base)
        } else {
            (base[0], base[1..].to_vec())
        };

        let exit_code = stream_process(prog, &args, &request_id, &app);
        let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code });
    });

    Ok(())
}

/// Update firmware via fwupd: refresh metadata then apply updates.
/// Streams output through terminal events so the user sees progress.
#[tauri::command]
async fn update_firmware(
    request_id: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::thread;

    thread::spawn(move || {
        let is_distrobox = managers::is_in_distrobox();

        // fwupdmgr is a host-level tool — route through distrobox-host-exec if needed
        let (refresh_prog, refresh_args): (&str, Vec<&str>) = if is_distrobox {
            ("distrobox-host-exec", vec!["pkexec", "fwupdmgr", "refresh"])
        } else {
            ("pkexec", vec!["fwupdmgr", "refresh"])
        };

        let (update_prog, update_args): (&str, Vec<&str>) = if is_distrobox {
            ("distrobox-host-exec", vec!["pkexec", "fwupdmgr", "update"])
        } else {
            ("pkexec", vec!["fwupdmgr", "update"])
        };

        // Step 1: refresh metadata
        let _ = app.emit(
            "terminal::line",
            TerminalLine {
                request_id: request_id.clone(),
                text: "$ pkexec fwupdmgr refresh".into(),
                is_stderr: false,
            },
        );
        let code1 = stream_process(refresh_prog, &refresh_args, &request_id, &app);
        // Exit code 2 = metadata already up to date — not an error
        if code1 != 0 && code1 != 2 {
            let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code: code1 });
            return;
        }

        // Step 2: apply updates
        let _ = app.emit(
            "terminal::line",
            TerminalLine {
                request_id: request_id.clone(),
                text: "$ pkexec fwupdmgr update".into(),
                is_stderr: false,
            },
        );
        let code2 = stream_process(update_prog, &update_args, &request_id, &app);

        let _ = app.emit("terminal::done", TerminalDone { request_id, exit_code: code2 });
    });

    Ok(())
}

// ─── App Init ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_managers,
            stream_packages,
            stream_updates,
            find_icon,
            update_firmware,
            get_batch_uninstall_command,
            get_update_command,
            execute_update,
            execute_batch_uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
