use serde::{Deserialize, Serialize};

pub mod apt;
pub mod cargo_mgr;
pub mod detect;
pub mod dnf;
pub mod flatpak;
pub mod nix;
pub mod npm_mgr;
pub mod pacman;
pub mod snap;
pub mod local;

/// Represents an installed or searchable package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub manager: String,
    pub source: Option<String>,
    pub is_user_installed: bool,
    pub icon_name: Option<String>,
    pub category: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Represents an available package manager on this system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub color: String,
    pub emoji: String,
}

/// A pending update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub package_id: String,
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub manager: String,
    pub source: Option<String>,
}

/// Strip ANSI escape sequences (colors, bold, etc.) from command output.
/// Handles CSI sequences like \x1b[1m, \x1b[0m, \x1b[38;5;42m, etc.
fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // CSI sequence: ESC [ <params> <final_byte>
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Consume parameter bytes (digits, semicolons, question marks)
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == ';' || c == '?' {
                        chars.next();
                    } else {
                        // Final byte (an ASCII letter) ends the sequence
                        if c.is_ascii_alphabetic() {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            // else: standalone ESC or OSC sequence — skip the ESC
        } else {
            result.push(ch);
        }
    }

    result
}

/// Run a command and return stdout, or empty string on failure.
/// ANSI escape codes are automatically stripped from the output.
/// Errors are logged to stderr for debugging.
///
/// LD_LIBRARY_PATH is explicitly cleared before spawning so that when running
/// inside a Tauri AppImage (which injects its own bundled libs into that var),
/// host system binaries like `flatpak` or `rpm` don't link against the wrong
/// libraries and crash or return empty output silently.
pub fn run_cmd(prog: &str, args: &[&str]) -> String {
    match std::process::Command::new(prog)
        .args(args)
        .stdin(std::process::Stdio::null())
        .env_remove("LD_LIBRARY_PATH")
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    eprintln!("[bubblegum] {} {:?} exited {}: {}", prog, args, output.status, stderr.trim());
                }
            }
            strip_ansi_codes(&String::from_utf8_lossy(&output.stdout).to_string())
        }
        Err(e) => {
            eprintln!("[bubblegum] failed to run {} {:?}: {}", prog, args, e);
            String::new()
        }
    }
}

/// Check if a binary is available in PATH
pub fn cmd_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdin(std::process::Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true when running inside a distrobox container
pub fn is_in_distrobox() -> bool {
    std::path::Path::new("/run/host").exists()
}

/// Returns true when running inside a flatpak sandbox
pub fn is_in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Run a command on the host system via flatpak-spawn/distrobox-host-exec when containerized,
/// falling back to a direct call otherwise (e.g. native installs, CI).
/// ANSI escape codes are automatically stripped from the output.
pub fn run_host_cmd(prog: &str, args: &[&str]) -> String {
    if is_in_flatpak() {
        let mut full_args = vec!["--host", prog];
        full_args.extend_from_slice(args);
        match std::process::Command::new("flatpak-spawn")
            .args(&full_args)
            .stdin(std::process::Stdio::null())
            .env_remove("LD_LIBRARY_PATH")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        eprintln!("[bubblegum] flatpak-spawn {} {:?} exited {}: {}", prog, args, output.status, stderr.trim());
                    }
                }
                strip_ansi_codes(&String::from_utf8_lossy(&output.stdout).to_string())
            }
            Err(e) => {
                eprintln!("[bubblegum] failed to run flatpak-spawn {} {:?}: {}", prog, args, e);
                String::new()
            }
        }
    } else if is_in_distrobox() {
        let mut full_args = vec![prog];
        full_args.extend_from_slice(args);
        match std::process::Command::new("distrobox-host-exec")
            .args(&full_args)
            .stdin(std::process::Stdio::null())
            .env_remove("LD_LIBRARY_PATH")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        eprintln!("[bubblegum] distrobox-host-exec {} {:?} exited {}: {}", prog, args, output.status, stderr.trim());
                    }
                }
                strip_ansi_codes(&String::from_utf8_lossy(&output.stdout).to_string())
            }
            Err(e) => {
                eprintln!("[bubblegum] failed to run distrobox-host-exec {} {:?}: {}", prog, args, e);
                String::new()
            }
        }
    } else {
        run_cmd(prog, args)
    }
}

/// Check whether a binary exists on the host system.
/// Proxies through flatpak-spawn/distrobox-host-exec if containerized; otherwise uses `which`.
pub fn host_cmd_exists(name: &str) -> bool {
    if is_in_flatpak() {
        std::process::Command::new("flatpak-spawn")
            .args(["--host", "which", name])
            .stdin(std::process::Stdio::null())
            .env_remove("LD_LIBRARY_PATH")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else if is_in_distrobox() {
        std::process::Command::new("distrobox-host-exec")
            .args(["which", name])
            .stdin(std::process::Stdio::null())
            .env_remove("LD_LIBRARY_PATH")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        cmd_exists(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_bold() {
        // nix profile list wraps names in bold: ESC[1m...ESC[0m
        assert_eq!(strip_ansi_codes("\x1b[1mbluetuith\x1b[0m"), "bluetuith");
    }

    #[test]
    fn strip_ansi_colors() {
        assert_eq!(
            strip_ansi_codes("\x1b[38;5;42mgreen\x1b[0m"),
            "green"
        );
    }

    #[test]
    fn strip_ansi_multiple() {
        assert_eq!(
            strip_ansi_codes("\x1b[1mName:\x1b[0m \x1b[36mfirefox\x1b[0m"),
            "Name: firefox"
        );
    }

    #[test]
    fn strip_ansi_no_codes() {
        assert_eq!(strip_ansi_codes("plain text"), "plain text");
    }

    #[test]
    fn strip_ansi_empty() {
        assert_eq!(strip_ansi_codes(""), "");
    }

    #[test]
    fn strip_ansi_complex_sgr() {
        // Bold + underline + 256-color
        assert_eq!(
            strip_ansi_codes("\x1b[1;4;38;5;196mred\x1b[0m"),
            "red"
        );
    }
}
