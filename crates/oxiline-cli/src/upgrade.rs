//! Self-update engine (`doc/10-updater.md`). See the plan in
//! `docs/superpowers/plans/2026-08-09-unified-updater.md`.

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
}
