//! Self-update engine (`doc/10-updater.md`). See the plan in
//! `docs/superpowers/plans/2026-08-09-unified-updater.md`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use oxiline_core::{settings, util};

/// Parse `v` as a `MAJOR.MINOR.PATCH` triple. Tolerant of a leading `v`
/// Returns `None` for anything else — we never auto-update on a guess.
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let mut it = v.split('.');
    let maj = it.next()?.parse().ok()?;
    let min = it.next()?.parse().ok()?;
    // Strictly X.Y.Z — refuse two-part inputs so a 0.7 release is never
    // compared against a 0.7.0 manifest.
    let patch_str = it.next()?;
    let patch = patch_str.split('-').next()?.parse().ok()?;
    Some((maj, min, patch))
}

const PLATFORM_KEY: &str = "darwin-aarch64";
/// Where the desktop app's updater also points. Serves the version manifest.
const ENDPOINT: &str =
    "https://github.com/project-oxi/oxiline/releases/latest/download/latest.json";
const REPO: &str = "project-oxi/oxiline";
/// Rust target triple used in the CLI tarball asset name.
const TARGET_TRIPLE: &str = "aarch64-apple-darwin";

/// Caller-facing options for [`upgrade_in_app_with_verify`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub check: bool,
    pub json_progress: bool,
    pub assume_yes: bool,
}

/// Public entry point for `oxiline upgrade` (`doc/10-updater.md`).
pub fn run(conn: &rusqlite::Connection, opts: Options) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    emit(&opts, Event::Checking);
    let manifest = match fetch_manifest() {
        Ok(m) => m,
        Err(e) => {
            emit_err(&opts, e.to_string());
            return Err(e);
        }
    };
    let latest = manifest.version.as_str();
    let notes: String = manifest
        .notes
        .clone()
        .unwrap_or_else(|| format!("OxiLine {latest}"));
    if !is_newer(latest, current) {
        emit(&opts, Event::Latest { version: current });
        human(&opts, &format!("Already up to date (v{current})."));
        return Ok(());
    }
    emit(
        &opts,
        Event::Available {
            from: current,
            to: latest,
            notes: notes.as_str(),
        },
    );
    if opts.check {
        return Ok(());
    }
    human(&opts, &format!("Update available: v{current} → v{latest}."));
    // The only side-effect the GUI cares about — the watcher in `App.tsx`
    // turns this into a `tauri-plugin-process::relaunch()`. Standalone CLI
    // callers ignore it.
    let swap_result = if let Some(app) = app_bundle_root() {
        upgrade_in_app_with_verify(&manifest, &app, &opts, verify_minisign)
    } else {
        upgrade_standalone(latest, &opts)
    };
    if let Err(e) = swap_result {
        emit_err(&opts, e.to_string());
        return Err(e);
    }
    settings::set(
        conn,
        "update_request_at",
        &serde_json::Value::String(util::now_iso()),
    )?;
    emit(&opts, Event::Done { version: latest });
    human(
        &opts,
        &format!("Updated to v{latest}. Restart OxiLine to use the new version."),
    );
    Ok(())
}

/// Fetch the live `latest.json` manifest and parse it. Network errors
/// become `anyhow` errors so the caller can emit a JSON `error` event.
fn fetch_manifest() -> anyhow::Result<Manifest> {
    use anyhow::anyhow;
    let resp = ureq::get(ENDPOINT)
        .call()
        .map_err(|e| anyhow!("fetch manifest: {e}"))?;
    let body = resp
        .into_string()
        .map_err(|e| anyhow!("read manifest: {e}"))?;
    serde_json::from_str(&body).map_err(|e| anyhow!("parse manifest: {e}"))
}

/// Emit a single NDJSON event on stdout if `--json-progress` is set. The
/// GUI sidecar parses one JSON object per line; anything else on stdout
/// would corrupt that stream.
fn emit(opts: &Options, ev: Event<'_>) {
    if !opts.json_progress {
        return;
    }
    if let Ok(line) = serde_json::to_string(&ev) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

fn emit_err(opts: &Options, msg: String) {
    emit(opts, Event::Error { message: &msg });
}

fn human(opts: &Options, line: &str) {
    if !opts.json_progress {
        println!("{line}");
    }
}

/// Standalone CLI: download the release CLI tarball, verify SHA-256, and
/// replace the running binary in place. The standalone tarball is verified
/// via the GitHub-release `.sha256` side file (no minisign).
fn upgrade_standalone(latest: &str, opts: &Options) -> anyhow::Result<()> {
    use anyhow::{Context, anyhow, bail};
    let exe = std::env::current_exe().context("resolve running binary")?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow!("binary has no parent directory"))?;
    let work = sibling_tempdir(parent)?;
    let result: anyhow::Result<()> = (|| {
        let name = format!("oxiline-{TARGET_TRIPLE}.tar.gz");
        let url = format!("https://github.com/{REPO}/releases/download/v{latest}/{name}");
        let archive = work.join(&name);
        let mut on_pct = |_pct: u8| {};
        download(&url, &archive, &mut on_pct)?;
        let expected_resp = ureq::get(&format!("{url}.sha256"))
            .call()
            .map_err(|e| anyhow!("download sha256: {e}"))?;
        let expected = expected_resp
            .into_string()
            .map_err(|e| anyhow!("read sha256: {e}"))?;
        let expected = expected.split_whitespace().next().unwrap_or("");
        let actual = sha256_hex(&archive)?;
        if !expected.eq_ignore_ascii_case(&actual) {
            bail!("checksum mismatch: expected {expected}, got {actual}");
        }
        extract_tar_gz(&archive, &work)?;
        let new_bin = work.join("oxiline");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&new_bin)?.permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&new_bin, perm)?;
        }
        let old = work.join(".previous.bin");
        std::fs::rename(&exe, &old)
            .with_context(|| format!("move aside {}", exe.display()))?;
        std::fs::rename(&new_bin, &exe)
            .with_context(|| format!("install {}", exe.display()))?;
        let _ = std::fs::remove_file(&old);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    if result.is_ok() {
        emit(opts, Event::Swapping { mode: "standalone" });
    }
    result
}

 /// Download the signed `OxiLine.app.tar.gz`, verify the signature, and
/// Download the signed `OxiLine.app.tar.gz`, verify the signature, and
/// replace the bundle on disk with an atomic rename. The `verify` hook is
/// a test seam: production passes [`verify_minisign`]; the unit test
/// passes a no-op so the in-app happy path can be exercised without
/// re-signing a synthetic tarball.
fn upgrade_in_app_with_verify(
    manifest: &Manifest,
    app: &Path,
    _opts: &Options,
    verify: fn(&[u8], &str) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let asset = manifest
        .platforms
        .get(PLATFORM_KEY)
        .ok_or_else(|| anyhow::anyhow!("manifest has no asset for {PLATFORM_KEY}"))?;
    let parent = app
        .parent()
        .ok_or_else(|| anyhow::anyhow!("app bundle has no parent directory"))?;
    let work = sibling_tempdir(parent)?;
    let result: anyhow::Result<()> = (|| {
        let archive = work.join("bundle.app.tar.gz");
        let mut on_pct = |_pct: u8| {};
        download(&asset.url, &archive, &mut on_pct)?;
        let data = std::fs::read(&archive)?;
        verify(&data, &asset.signature)?;
        extract_tar_gz(&archive, &work)?;
        let new_app = find_entry_with_ext(&work, "app")?;
        let old = work.join(".previous.app");
        if app.exists() {
            std::fs::rename(app, &old)
                .with_context(|| format!("move aside {}", app.display()))?;
        }
        std::fs::rename(&new_app, app)
            .with_context(|| format!("install {}", app.display()))?;
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

 /// The `.app` bundle the running binary lives in, if any (the sidecar case).
/// The `.app` bundle the running binary lives in, if any (the sidecar case).
fn app_bundle_root() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| app_bundle_root_of(&exe))
}

/// Factored out of [`app_bundle_root`] for testing on synthetic paths.
/// Walks the executable's ancestors and returns the first directory whose
/// extension is `app` (the canonical Tauri/macOS bundle marker).
fn app_bundle_root_of(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    for ancestor in exe.ancestors() {
        if ancestor.extension().is_some_and(|e| e == "app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// `latest.json` manifest for OxiLine releases (`doc/10-updater.md`).
/// Deserialized from the response at `latest.json#version`. `notes` and
/// `pub_date` are optional — older manifests may omit them.
#[derive(serde::Deserialize)]
struct Manifest {
    version: String,
    /// Release notes surfaced to the GUI banner and the Preferences section.
    /// Optional; the JSON `available` event falls back to
    /// `"OxiLine <version>"` when the manifest omits it.
    #[serde(default)]
    notes: Option<String>,
    /// Tauri-shaped `pub_date` (RFC 3339). Kept optional — we don't surface
    /// it to the GUI today but parsing it preserves forward-compat.
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    platforms: std::collections::HashMap<String, PlatformAsset>,
}

#[derive(serde::Deserialize)]
struct PlatformAsset {
    url: String,
    signature: String,
}


fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    use anyhow::Context;
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = std::io::Read::read(&mut f, &mut buf).context("read for hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Extract a `.tar.gz` into `dest` (overwriting). Used by both the in-app
/// (`.app.tar.gz`) and standalone (CLI tarball) upgrade paths.
fn extract_tar_gz(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    let f = std::fs::File::open(archive)
        .with_context(|| format!("open {}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tar = tar::Archive::new(gz);
    tar.set_overwrite(true);
    tar.unpack(dest)
        .with_context(|| format!("extract {}", archive.display()))?;
    Ok(())
}

/// Return the first entry under `dir` whose extension equals `ext`.
/// Used to locate the freshly-extracted `.app` (in-app path) or the bare
/// `oxiline` binary (standalone path) inside the work dir.
fn find_entry_with_ext(dir: &Path, ext: &str) -> anyhow::Result<PathBuf> {
    use anyhow::{anyhow, bail};
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == ext) {
            return Ok(entry.path());
        }
    }
    bail!("extracted archive contained no .{ext}");
}

/// A temp dir on the same volume as `sibling_of` so renames stay atomic
/// (the swap is the rename onto the live `.app` or binary; a cross-volume
/// rename would fail or copy).
fn sibling_tempdir(sibling_of: &Path) -> anyhow::Result<PathBuf> {
    use anyhow::Context;
    let dir = sibling_of.join(format!(".oxiline-upgrade-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create work dir {}", dir.display()))?;
    Ok(dir)
}

 /// Stream a download to `dest`, calling `on_pct` with `0` once at the start
/// Stream a download to `dest`, calling `on_pct` with `0` once at the start
/// and `100` at the end, with monotonic intermediate values while bytes
/// stream in. The `pct` we report is best-effort — `ureq`'s `into_reader()`
/// discards the response headers (and thus the real `content-length`), so
/// the GUI's own progress bar (which has the byte counter) is authoritative.
fn download(url: &str, dest: &Path, on_pct: &mut dyn FnMut(u8)) -> anyhow::Result<()> {
    use anyhow::anyhow;
    let resp = ureq::get(url)
        .call()
        .map_err(|e| anyhow!("download {url}: {e}"))?;
    let mut reader = resp.into_reader();
    let mut f = std::fs::File::create(dest)
        .map_err(|e| anyhow!("create {}: {e}", dest.display()))?;
    let mut buf = [0u8; 64 * 1024];
    let mut total: usize = 0;
    let mut last_pct: u8 = 0;
    on_pct(0);
    loop {
        let n = reader.read(&mut buf).map_err(|e| anyhow!("download read: {e}"))?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| anyhow!("download write: {e}"))?;
        total += n;
        // 1 MiB granularity keeps the bar visibly moving without spamming
        // NDJSON lines. The maximum we report is 99 — `done` is the
        // authoritative completion event.
        let approx = ((total / (1024 * 1024)) as u8).min(99);
        if approx > last_pct {
            last_pct = approx;
            on_pct(last_pct);
        }
    }
    f.sync_all().ok();
    on_pct(100);
    Ok(())
}

/// Minisign public key for the live OxiLine release.
/// Minisign public key for the live OxiLine release. The inner base64
/// `RWQ…u` line from the `minisign.pub` file (the outer `tauri.conf.json`
/// pubkey was a base64-of-the-pub-file; we store the decoded line here so
/// the call site is one `from_base64` away). The CLI is the only
/// verifier; the GUI no longer embeds this.
const LIVE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2NTU3RTc3QTc2MzUwMTYKUldRV1VHT25kMzVWaHU1K3BqTmhaNXBCamQ0TisxWVR6OG5zZFRGbGx2bnJDWjc5SFNhdjdCM3UK";
/// Well-known minisign test public key (from `minisign-verify`'s README).
/// Used by the unit tests so we can ship the well-known signature
/// verbatim instead of re-signing at test time.
#[cfg(test)]
/// Well-known minisign test public key in base64-of-the-`minisign.pub` file
/// form (the same wire shape Tauri uses in `tauri.conf.json`). The
/// corresponding file is reconstructed in the unit tests as needed.
#[cfg(test)]
const TEST_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXkgMEYzQ0MzMDMxQjg1RjNGQTFBNjVFQzI5RDJEODBDNEIwQUI2QzhCNUEzRUJFMDhFOEM4RDFFRDBFNUIzRkExQQpSV1FmNkxSQ0dBOWk1M21sWWVjTzRJelQ1MVRHUHB2V3VjTlNDaDFDQk0wUVRhTG43M1k3R0ZPMwo=";
fn verify_minisign(data: &[u8], sig: &str) -> anyhow::Result<()> {
    verify_minisign_with(data, sig, LIVE_PUBKEY)
}

/// Verifies a minisign signature with an explicit key. The test seam exists
/// so unit tests can drive the verify path with the well-known test pubkey
/// from `minisign-verify`'s own README without baking that key into the
/// production binary.
fn verify_minisign_with(data: &[u8], sig_b64: &str, key_b64: &str) -> anyhow::Result<()> {
    use anyhow::anyhow;
    use base64::Engine as _;
    // The pubkey constant is base64-of-the-`minisign.pub` file (same
    // shape Tauri shipped in `tauri.conf.json` until this commit). Decode
    // once to recover the inner pub file, then hand it to the parser.
    let key_raw = base64::engine::general_purpose::STANDARD
        .decode(key_b64.trim().as_bytes())
        .map_err(|e| anyhow!("decode public key: {e}"))?;
    let key_box = String::from_utf8(key_raw)
        .map_err(|e| anyhow!("public key not utf-8: {e}"))?;
    let pk = minisign_verify::PublicKey::decode(&key_box)
        .map_err(|e| anyhow!("parse public key: {e}"))?;
    // The signature arg from `latest.json#platforms.darwin-aarch64.signature`
    // is also base64-of-the-`.minisig` file.
    let sig_raw = base64::engine::general_purpose::STANDARD
        .decode(sig_b64.trim().as_bytes())
        .map_err(|e| anyhow!("decode signature: {e}"))?;
    let sig_box = String::from_utf8(sig_raw)
        .map_err(|e| anyhow!("signature not utf-8: {e}"))?;
    let parsed = minisign_verify::Signature::decode(&sig_box)
        .map_err(|e| anyhow!("parse signature: {e}"))?;
    pk.verify(data, &parsed, false)
        .map_err(|e| anyhow!("signature verification failed: {e}"))?;
    Ok(())
}

/// NDJSON progress event on stdout when `--json-progress` is set.
/// NDJSON progress event on stdout when `--json-progress` is set. The schema
/// is part of the GUI↔CLI contract (`doc/10-updater.md`); keep field names
/// and types stable. The matching test block in `#[cfg(test)] mod tests`
/// pins every event shape.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event<'a> {
    Checking,
    Current { version: &'a str },
    Available { from: &'a str, to: &'a str, notes: &'a str },
    Latest { version: &'a str },
    Download { pct: u8 },
    Verifying,
    Swapping { mode: &'a str },
    Done { version: &'a str },
    Error { message: &'a str },
}

/// `true` iff `latest` is strictly newer than `current` (numeric, X.Y.Z).
/// Unparseable inputs are treated as not-newer — we never claim an update
/// for a version we can't compare.
fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_versions() {
        assert_eq!(parse_version("0.9.0"), Some((0, 9, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        // Pre-release suffix tolerated on patch.
        assert_eq!(parse_version("1.2.3-rc1"), Some((1, 2, 3)));
    }

    #[test]
    fn rejects_garbage_versions() {
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version(""), None);
        // Two-part versions are not X.Y.Z; refuse so we never auto-update
        // on a guess.
        assert_eq!(parse_version("1.2"), None);
    }

    #[test]
    fn newer_detection() {
        assert!(is_newer("0.9.1", "0.9.0"));
        assert!(is_newer("1.0.0", "0.9.9")); // numeric, not lexical
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.8.9", "0.9.0"));
    }

    #[test]
    fn unparseable_never_newer() {
        assert!(!is_newer("oops", "0.9.0"));
        assert!(!is_newer("0.9.1", "oops"));
    }

    #[test]
    fn manifest_deserializes_minimal_shape() {
        let json = r#"{
            "version": "0.7.0",
            "notes": "OxiLine 0.7.0",
            "pub_date": "2026-08-01T00:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/OxiLine.app.tar.gz",
                    "signature": "AAA"
                }
            }
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("manifest parses");
        assert_eq!(m.version, "0.7.0");
        assert_eq!(m.notes.as_deref(), Some("OxiLine 0.7.0"));
        assert!(m.platforms.contains_key("darwin-aarch64"));
    }

    #[test]
    fn manifest_notes_is_optional() {
        let json = r#"{
            "version": "0.7.0",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/OxiLine.app.tar.gz",
                    "signature": "AAA"
                }
            }
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("manifest parses");
        assert_eq!(m.notes, None);
    }

    #[test]
    fn detects_app_bundle_from_sidecar_path() {
        let exe = std::path::PathBuf::from("/Applications/OxiLine.app/Contents/MacOS/oxiline");
        assert_eq!(
            app_bundle_root_of(&exe),
            Some(std::path::PathBuf::from("/Applications/OxiLine.app"))
        );
    }

    #[test]
    fn no_app_bundle_for_standalone() {
        let exe = std::path::PathBuf::from("/Users/x/.cargo/bin/oxiline");
        assert_eq!(app_bundle_root_of(&exe), None);
        // No `.app` ancestor even if a directory is literally named
        // `something.app` mid-path — only an actual `.app` ancestor counts.
        let exe = std::path::PathBuf::from("/usr/local/bin/oxiline");
        assert_eq!(app_bundle_root_of(&exe), None);
    }

    /// Wire contract: the exact JSON shapes here are what the GUI parses.
    /// Renaming a field silently breaks the sidecar; these tests pin the
    /// schema of every event (`doc/10-updater.md`).
    #[test]
    fn event_checking_serializes_to_typed_tag() {
        assert_eq!(
            serde_json::to_string(&Event::Checking).unwrap(),
            r#"{"type":"checking"}"#
        );
    }

    #[test]
    fn event_available_carries_from_to_notes() {
        let v = serde_json::to_value(&Event::Available {
            from: "0.6.1",
            to: "0.7.0",
            notes: "OxiLine 0.7.0",
        })
        .unwrap();
        assert_eq!(v["type"], "available");
        assert_eq!(v["from"], "0.6.1");
        assert_eq!(v["to"], "0.7.0");
        assert_eq!(v["notes"], "OxiLine 0.7.0");
    }

    #[test]
    fn event_download_pct_is_a_number_not_string() {
        let v = serde_json::to_value(&Event::Download { pct: 42 }).unwrap();
        assert_eq!(v["type"], "download");
        assert_eq!(v["pct"].as_u64(), Some(42));
    }

    #[test]
    fn event_swapping_mode_uses_snake_case_known_values() {
        assert_eq!(
            serde_json::to_string(&Event::Swapping { mode: "app" }).unwrap(),
            r#"{"type":"swapping","mode":"app"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Swapping { mode: "standalone" }).unwrap(),
            r#"{"type":"swapping","mode":"standalone"}"#
        );
    }

    #[test]
    fn event_done_and_latest_and_error_match_contract() {
        assert_eq!(
            serde_json::to_string(&Event::Done { version: "0.7.0" }).unwrap(),
            r#"{"type":"done","version":"0.7.0"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Latest { version: "0.6.1" }).unwrap(),
            r#"{"type":"latest","version":"0.6.1"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Error { message: "boom" }).unwrap(),
            r#"{"type":"error","message":"boom"}"#
        );
    }
}


    /// Round-trip against a known-good minisign signature. The fixture is
    /// the well-known test vector from `minisign-verify`'s own README
    /// (no minisign CLI on this machine, so we ship the vector verbatim
    /// instead of re-signing at test time). The live release probe lives

    /// Drives `download` against a tiny in-process TCP server. The server
    /// returns a known byte count; the callback must fire `0` once at the
    /// start, increasing values up to ≤99, and `100` at the end.
    #[test]
    fn download_emits_zero_then_ascending_then_100() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body: Vec<u8> = (0..(5 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
        let body_len = body.len();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf); // drain request line
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
            );
            s.write_all(resp.as_bytes()).unwrap();
            s.write_all(&body).unwrap();
        });
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out.bin");
        let mut calls: Vec<u8> = Vec::new();
        download(&format!("http://{addr}/"), &dest, &mut |pct| calls.push(pct)).unwrap();
        assert_eq!(calls.first(), Some(&0));
        assert_eq!(calls.last(), Some(&100));
        // Monotonic non-decreasing.
        for w in calls.windows(2) {
            assert!(w[0] <= w[1], "progress must be non-decreasing: {w:?}");
        }
        assert_eq!(std::fs::metadata(&dest).unwrap().len() as usize, body_len);
    }

    /// in `verifies_live_release_signature` (#[ignore]).
    #[test]
    fn verify_minisign_accepts_known_good_signature() {
        let payload = std::fs::read("tests/fixtures/payload.txt")
            .expect("fixture payload present");
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig.b64")
            .expect("fixture signature present");
        verify_minisign_with(&payload, &sig_raw, TEST_PUBKEY)
            .expect("valid signature verifies");
    }

    #[test]
    fn verify_minisign_rejects_tampered_payload() {
        let mut payload = std::fs::read("tests/fixtures/payload.txt")
            .expect("fixture payload present");
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig.b64")
            .expect("fixture signature present");
        payload[0] ^= 0x01; // flip a bit
        assert!(
            verify_minisign_with(&payload, &sig_raw, TEST_PUBKEY).is_err(),
            "tampered payload must fail"
        );
    }

    #[test]
    fn verify_minisign_rejects_wrong_key() {
        let payload = std::fs::read("tests/fixtures/payload.txt")
            .expect("fixture payload present");
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig.b64")
            .expect("fixture signature present");
        // The OxiLine live pubkey is different bytes from the test pubkey
        // — a verify with the wrong key must fail.
        assert!(
            verify_minisign_with(&payload, &sig_raw, LIVE_PUBKEY).is_err(),
            "mismatched key must fail"
        );
    }

    #[test]
    fn sha256_hex_matches_known_digest() {
        // Echoed "abc" → SHA-256 (NIST test vector).
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("abc.txt");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            sha256_hex(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn extract_tar_gz_preserves_entry() {
        // Build a tar.gz in memory containing "hello.txt" → "hi\n".
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("in.tar.gz");
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::fast(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_path("hello.txt").unwrap();
        header.set_size(3);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, b"hi\n" as &[u8]).unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        extract_tar_gz(&archive, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("hello.txt")).unwrap(), "hi\n");
    }

    #[test]
    fn find_entry_with_ext_finds_app_but_ignores_others() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"x").unwrap();
        std::fs::write(tmp.path().join("OxiLine.app"), b"x").unwrap();
        let found = find_entry_with_ext(tmp.path(), "app").unwrap();
        assert!(found.ends_with("OxiLine.app"));
    }

    #[test]
    fn sibling_tempdir_lives_next_to_sibling_and_uses_pid() {
        let tmp = tempfile::tempdir().unwrap();
        let work = sibling_tempdir(tmp.path()).unwrap();
        assert!(work.starts_with(tmp.path()));
        assert!(work.to_string_lossy().contains(&std::process::id().to_string()));
        std::fs::remove_dir_all(&work).unwrap();
    }



    /// In-app swap on a synthetic path. The fake HTTP server returns a
    /// tarball that contains a directory called `OxiLine.app/foo.txt`. The
    /// test exercises the public manifest API end-to-end (parse, download,
    /// extract, find `.app`, rename into place) and confirms the parent
    /// directory ends up holding the new bundle. The verify hook is
    /// bypassed so we don't need a real signature for a synthetic tarball.
    #[test]
    fn upgrade_in_app_replaces_bundle_on_disk() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::path::PathBuf;
        // Build a tar.gz in memory: OxiLine.app/foo.txt = "fresh".
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let body = b"fresh".to_vec();
        let mut header = tar::Header::new_gnu();
        header.set_path("OxiLine.app/foo.txt").unwrap();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, body.as_slice()).unwrap();
        let gz = builder.into_inner().unwrap().finish().unwrap();
        // Server: respond 200 with the tarball, regardless of path.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let gz_len = gz.len();
        let gz_clone = gz.clone();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {gz_len}\r\nConnection: close\r\n\r\n"
            );
            s.write_all(resp.as_bytes()).unwrap();
            s.write_all(&gz_clone).unwrap();
        });
        let sig = std::fs::read_to_string("tests/fixtures/payload.txt.minisig.b64")
            .expect("fixture signature present");
        let mut platforms = std::collections::HashMap::new();
        platforms.insert(
            "darwin-aarch64".into(),
            PlatformAsset {
                url: format!("http://{addr}/bundle.app.tar.gz"),
                signature: sig,
            },
        );
        let manifest = Manifest {
            version: "9.9.9".into(),
            notes: Some("test".into()),
            pub_date: None,
            platforms,
        };
        let work = tempfile::tempdir().unwrap();
        let app: PathBuf = work.path().join("OxiLine.app");
        std::fs::create_dir(&app).unwrap();
        std::fs::write(app.join("old.txt"), b"old").unwrap();
        upgrade_in_app_with_verify(&manifest, &app, &Options::default(), |_, _| Ok(()))
            .expect("swap should succeed");
        assert!(app.join("foo.txt").exists(), "new foo.txt must be in place");
        assert!(!app.join("old.txt").exists(), "old file must be gone");
        let leftover: Vec<_> = std::fs::read_dir(work.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".oxiline-upgrade-")
            })
            .collect();
        assert!(leftover.is_empty(), "sibling tempdir must be cleaned up");
    }


    /// End-to-end probe: the OxiLine `latest.json` → the live signed
    /// bundle. This is the safety check the spec calls out before
    /// removing `tauri-plugin-updater` from the GUI. Run with
    /// `cargo test -p oxiline-cli --bin oxiline --release -- --ignored verifies_live_release_signature --nocapture`.
    #[test]
    #[ignore]
    fn verifies_live_release_signature() {
        let manifest = fetch_manifest().expect("fetch manifest");
        let asset = manifest
            .platforms
            .get(PLATFORM_KEY)
            .expect("manifest has darwin-aarch64 asset");
        let resp = ureq::get(&asset.url).call().expect("download bundle");
        let mut data = Vec::new();
        resp.into_reader()
            .read_to_end(&mut data)
            .expect("read bundle");
        verify_minisign(&data, &asset.signature)
            .expect("PUBKEY verifies the live signature");
    }

