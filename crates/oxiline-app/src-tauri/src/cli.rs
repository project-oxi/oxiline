//! CLI install commands — mirror `oximemo::cli` (see spec).
//!
//! The `oxiline` CLI is shipped **inside** the Tauri `.app` as an
//! `externalBin` sidecar. Settings → "Install command" creates a symlink
//! at `/usr/local/bin/oxiline` via a one-time macOS admin prompt
//! (`osascript`). `cli_status` reports whether the symlink exists and
//! points at *this* `.app`'s bundle.

use std::path::{Path, PathBuf};

#[derive(serde::Serialize, specta::Type, PartialEq, Eq, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[specta(rename_all = "lowercase")]
pub enum CliState {
    /// Symlink present and points at this app's bundled CLI.
    Installed,
    /// No symlink present.
    NotInstalled,
    /// Symlink target differs (stray copy or post-install move of the `.app`).
    Stale,
}

/// Pure classification of the `/usr/local/bin/oxiline` link state.
///
/// - `link_target = Some(p)` → read_link succeeded (it's a symlink).
///   `Installed` iff `p` and `bundled` canonicalize to the same path.
///   Otherwise `Stale`.
/// - `link_target = None` → not a symlink (Err from read_link).
///   `Stale` if the path exists (stray copy), `NotInstalled` if absent.
///
/// `bundled == None` short-circuits to `NotInstalled` — the app couldn't
/// locate its own binary (only happens on exotic platforms).
pub fn classify(link_target: Option<&Path>, link_exists: bool, bundled: Option<&Path>) -> CliState {
    let Some(bundled) = bundled else {
        return CliState::NotInstalled;
    };
    let canonical_eq =
        |a: &Path, b: &Path| std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok();
    match link_target {
        Some(target) if canonical_eq(target, bundled) => CliState::Installed,
        Some(_) => CliState::Stale,
        None => {
            if link_exists {
                CliState::Stale
            } else {
                CliState::NotInstalled
            }
        }
    }
}

/// Path of the bundled CLI, derived from the running executable so it
/// tracks wherever the user installed the `.app`. `None` only if the exe
/// path can't be resolved.
fn bundled_cli_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // The externalBin sidecar lands in Contents/MacOS/ next to the main
    // binary, under its base name — Tauri strips the `-<triple>` suffix
    // during bundling (input `oxiline-<triple>` → bundled `oxiline`).
    Some(dir.join("oxiline"))
}

#[tauri::command]
#[specta::specta]
pub fn cli_status() -> Result<CliState, String> {
    let link = Path::new("/usr/local/bin/oxiline");
    let bundled = bundled_cli_path();
    let link_target = std::fs::read_link(link).ok();
    let link_exists = link.exists();
    Ok(classify(
        link_target.as_deref(),
        link_exists,
        bundled.as_deref(),
    ))
}

#[tauri::command]
#[specta::specta]
pub fn install_cli() -> Result<(), String> {
    let target = bundled_cli_path().ok_or_else(|| "could not locate the app bundle".to_string())?;
    if !target.exists() {
        return Err("bundled CLI binary is missing".to_string());
    }
    // Shell-quote the path; app-bundle paths never contain a quote, guard.
    let q = target.display().to_string().replace('\'', "'\"'\"'");
    run_admin(&format!("ln -sf '{q}' /usr/local/bin/oxiline"))
}

#[tauri::command]
#[specta::specta]
pub fn uninstall_cli() -> Result<(), String> {
    run_admin("rm -f /usr/local/bin/oxiline")
}

/// Run a shell snippet with administrator privileges via osascript. macOS
/// shows its standard auth dialog once; cancelling surfaces as an error.
fn run_admin(shell_script: &str) -> Result<(), String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script {} with administrator privileges",
            applescript_string(shell_script)
        ))
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Quote `s` as an AppleScript double-quoted string literal.
fn applescript_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real on-disk fixture: a tempdir + a "bundled CLI" file inside
    /// it. Returns the bundle path; canonicalize resolves symlinks so the
    /// `read_link` target matches.
    fn bundled_in_tmp() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("oxiline");
        std::fs::write(&bundled, b"#!/bin/sh\nexit 0\n").unwrap();
        (tmp, bundled)
    }

    #[test]
    fn classify_none_bundled_is_not_installed() {
        assert_eq!(classify(None, false, None), CliState::NotInstalled);
    }

    #[test]
    fn classify_absent_link_is_not_installed() {
        let (_tmp, bundled) = bundled_in_tmp();
        assert_eq!(
            classify(None, false, Some(&bundled)),
            CliState::NotInstalled
        );
    }

    #[test]
    fn classify_stray_file_at_link_is_stale() {
        let (_tmp, bundled) = bundled_in_tmp();
        // Link missing, but a regular file at the link path → Stale.
        assert_eq!(classify(None, true, Some(&bundled)), CliState::Stale);
    }

    #[test]
    fn classify_matching_symlink_is_installed() {
        let (_tmp, bundled) = bundled_in_tmp();
        // Symlink at a fresh tmpdir path → bundled target.
        let link_dir = tempfile::tempdir().unwrap();
        let link = link_dir.path().join("oxiline-link");
        std::os::unix::fs::symlink(&bundled, &link).unwrap();
        // The fixture's read_link path is the symlink itself, target = bundled.
        let target = std::fs::read_link(&link).unwrap();
        assert_eq!(
            classify(Some(&target), true, Some(&bundled)),
            CliState::Installed
        );
    }

    #[test]
    fn classify_diverging_symlink_is_stale() {
        let (_tmp, bundled) = bundled_in_tmp();
        let other = tempfile::tempdir().unwrap();
        let other_path = other.path().join("oxiline");
        std::fs::write(&other_path, b"#!/bin/sh\nexit 0\n").unwrap();
        assert_eq!(
            classify(Some(&other_path), true, Some(&bundled)),
            CliState::Stale
        );
    }

    #[test]
    fn applescript_string_escapes_quotes_and_backslashes() {
        // Backslash, quote, and a benign character.
        let s = applescript_string(r#"a'b\c"d"#);
        // Expected: starts/ends with `"`; inner backslash + quote escaped.
        assert_eq!(s, r#""a'b\\c\"d""#);
    }
}
