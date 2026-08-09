//! Self-update engine (`doc/10-updater.md`). See the plan in
//! `docs/superpowers/plans/2026-08-09-unified-updater.md`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};


/// Parse `v` as a `MAJOR.MINOR.PATCH` triple. Tolerant of a leading `v`
/// and of a pre-release suffix on the patch (e.g. `1.2.3-rc1` → `(1,2,3)`).
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
const LIVE_PUBKEY: &str = "RWQWUGOnd35Vhu5+pjNhZ5pBjd4N+1YTz8nsdTFllvnrCZ79HSav7B3u";

/// Well-known minisign test public key (from `minisign-verify`'s README).
/// Used by the unit tests so we can ship the well-known signature
/// verbatim instead of re-signing at test time.
#[cfg(test)]
const TEST_PUBKEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";

/// Production entry point. Verifies a minisign signature over `data` using
/// the live OxiLine public key. Wire format: the raw `.minisig` file
/// content (untrusted comment + base64 sig box + trusted comment + final
/// base64).
fn verify_minisign(data: &[u8], sig: &str) -> anyhow::Result<()> {
    verify_minisign_with(data, sig, LIVE_PUBKEY)
}

/// Verifies a minisign signature with an explicit key. The test seam exists
/// so unit tests can drive the verify path with the well-known test pubkey
/// from `minisign-verify`'s own README without baking that key into the
/// production binary.
fn verify_minisign_with(data: &[u8], sig: &str, key_b64: &str) -> anyhow::Result<()> {
    use anyhow::anyhow;
    let pk = minisign_verify::PublicKey::from_base64(key_b64.trim())
        .map_err(|e| anyhow!("parse public key: {e}"))?;
    let parsed = minisign_verify::Signature::decode(sig)
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
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig")
            .expect("fixture signature present");
        verify_minisign_with(&payload, &sig_raw, TEST_PUBKEY)
            .expect("valid signature verifies");
    }

    #[test]
    fn verify_minisign_rejects_tampered_payload() {
        let mut payload = std::fs::read("tests/fixtures/payload.txt")
            .expect("fixture payload present");
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig")
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
        let sig_raw = std::fs::read_to_string("tests/fixtures/payload.txt.minisig")
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


