//! Decision logic for Agwaita CLI build version calculation.
//!
//! This module defines pure functions for computing the build version string
//! from Git metadata (tags, branch, modifications, diff hash).

/// Strips a leading 'v' prefix from a tag if present.
///
/// Only a single leading 'v' is stripped (e.g. `v1.1.0-stable` becomes `1.1.0-stable`,
/// while `vv1.1.0-stable` becomes `v1.1.0-stable`).
pub fn strip_tag_prefix(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// Checks if a Git tag matches the required `<Major>.<Minor>.<Patch>[-<etat>]` format.
///
/// Example of valid tags: `1.2.0-beta`, `1.2.0-rc1`, `0.1.0-alpha.1`, `1.1.0`.
/// Tags like `v1.2.0`, `1.2`, or `1.2-beta` are not considered valid release tags here.
pub fn is_valid_tag(tag: &str) -> bool {
    let (version_part, etat_part) = match tag.split_once('-') {
        Some((v, e)) => (v, Some(e)),
        None => (tag, None),
    };

    if let Some(etat) = etat_part
        && (etat.is_empty() || !etat.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'))
    {
        return false;
    }

    let version_segments: Vec<&str> = version_part.split('.').collect();
    if version_segments.len() != 3 {
        return false;
    }

    version_segments
        .iter()
        .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()))
}

/// Extracts the root segment of a Git branch name.
///
/// For example, `dev/audio/feature` yields `dev`, and `main` yields `main`.
/// When HEAD is detached or the branch is unavailable, `"detached"` is returned as an explicit
/// fallback to clearly identify dirty detached-state builds without ambiguity in bug reports.
pub fn extract_branch_root(branch: Option<&str>) -> &str {
    match branch.map(str::trim) {
        Some(b) if !b.is_empty() && b != "HEAD" => b.split('/').next().unwrap_or(b),
        _ => "detached",
    }
}

/// Computes the fallback version string when Git is unavailable or when not building inside a Git repository.
///
/// Output format: `<CARGO_PKG_VERSION>-nogit`.
pub fn compute_fallback_version(pkg_version: &str) -> String {
    format!("{pkg_version}-nogit")
}

/// Computes the final version string based on Git state and diff hash.
///
/// Rules:
/// 1. HEAD on tag, unmodified: `<tag>`
/// 2. HEAD on tag, modified: `<tag>-<hash>`
/// 3. HEAD without tag, unmodified: `SNAPSHOT-<hash>`
/// 4. HEAD without tag, modified: `<racine>-<hash>` (where `<racine>` is the first branch segment)
pub fn compute_version(tag: Option<&str>, is_modified: bool, branch: Option<&str>, diff_hash: &str) -> String {
    let normalized_tag = tag.map(strip_tag_prefix);
    let valid_tag = normalized_tag.filter(|t| is_valid_tag(t));

    match (valid_tag, is_modified) {
        (Some(tag_name), false) => tag_name.to_string(),
        (Some(tag_name), true) => format!("{tag_name}-{diff_hash}"),
        (None, false) => format!("SNAPSHOT-{diff_hash}"),
        (None, true) => {
            let branch_root = extract_branch_root(branch);
            format!("{branch_root}-{diff_hash}")
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tag_prefix() {
        assert_eq!(strip_tag_prefix("v1.1.0-stable"), "1.1.0-stable");
        assert_eq!(strip_tag_prefix("1.1.0-stable"), "1.1.0-stable");
        assert_eq!(strip_tag_prefix("vv1.1.0-stable"), "v1.1.0-stable");
        assert_eq!(strip_tag_prefix("v1.1.0"), "1.1.0");
    }

    #[test]
    fn test_compute_fallback_version() {
        assert_eq!(compute_fallback_version("0.1.0"), "0.1.0-nogit");
        assert_eq!(compute_fallback_version("1.2.3"), "1.2.3-nogit");
    }

    #[test]
    fn test_is_valid_tag() {
        assert!(is_valid_tag("1.2.0-beta"));
        assert!(is_valid_tag("1.2.0-rc1"));
        assert!(is_valid_tag("0.1.0-alpha.1"));
        assert!(is_valid_tag("10.20.300-release1"));
        assert!(is_valid_tag("1.1.0-stable"));
        assert!(is_valid_tag("1.1.0"));
        assert!(is_valid_tag("1.2.0"));

        assert!(!is_valid_tag("v1.2.0-beta"));
        assert!(!is_valid_tag("vv1.1.0-stable"));
        assert!(!is_valid_tag("v1.1.0"));
        assert!(!is_valid_tag("vv1.1.0"));
        assert!(!is_valid_tag("v1.2.0"));
        assert!(!is_valid_tag("1.2-beta"));
        assert!(!is_valid_tag("1.2"));
        assert!(!is_valid_tag("1.2.0-"));
        assert!(!is_valid_tag("-beta"));
        assert!(!is_valid_tag("1.2.a-beta"));
    }

    #[test]
    fn test_extract_branch_root() {
        assert_eq!(
            extract_branch_root(Some(
                "dev/audio-manger-to-deprecate-pavucontrol/audio-struct"
            )),
            "dev"
        );
        assert_eq!(extract_branch_root(Some("main")), "main");
        assert_eq!(extract_branch_root(Some("feature/ui/topbar")), "feature");
        assert_eq!(extract_branch_root(Some("")), "detached");
        assert_eq!(extract_branch_root(Some("   ")), "detached");
        assert_eq!(extract_branch_root(Some("HEAD")), "detached");
        assert_eq!(extract_branch_root(None), "detached");
    }

    #[test]
    fn test_compute_version_rule_1_tag_unmodified() {
        let ver = compute_version(Some("1.2.0-beta"), false, Some("main"), "e3b0c442");
        assert_eq!(ver, "1.2.0-beta");

        let ver_v = compute_version(Some("v1.1.0-stable"), false, Some("main"), "e3b0c442");
        assert_eq!(ver_v, "1.1.0-stable");

        let ver_no_v = compute_version(Some("1.1.0-stable"), false, Some("main"), "e3b0c442");
        assert_eq!(ver_no_v, "1.1.0-stable");

        let ver_v_no_suffix = compute_version(Some("v1.1.0"), false, Some("main"), "e3b0c442");
        assert_eq!(ver_v_no_suffix, "1.1.0");

        let ver_no_suffix = compute_version(Some("1.1.0"), false, Some("main"), "e3b0c442");
        assert_eq!(ver_no_suffix, "1.1.0");
    }

    #[test]
    fn test_compute_version_rule_2_tag_modified() {
        let ver = compute_version(Some("1.2.0-beta"), true, Some("main"), "abcd1234");
        assert_eq!(ver, "1.2.0-beta-abcd1234");

        let ver_v = compute_version(Some("v1.1.0-stable"), true, Some("main"), "abcd1234");
        assert_eq!(ver_v, "1.1.0-stable-abcd1234");
    }

    #[test]
    fn test_compute_version_rule_3_no_tag_unmodified() {
        let ver = compute_version(None, false, Some("main"), "e3b0c442");
        assert_eq!(ver, "SNAPSHOT-e3b0c442");
    }

    #[test]
    fn test_compute_version_rule_3_invalid_tag_unmodified() {
        let ver = compute_version(Some("vv1.1.0"), false, Some("dev/my-branch"), "e3b0c442");
        assert_eq!(ver, "SNAPSHOT-e3b0c442");

        let ver_vv = compute_version(
            Some("vv1.1.0-stable"),
            false,
            Some("dev/my-branch"),
            "e3b0c442",
        );
        assert_eq!(ver_vv, "SNAPSHOT-e3b0c442");
    }

    #[test]
    fn test_compute_version_rule_4_no_tag_modified_branch() {
        let ver = compute_version(
            None,
            true,
            Some("dev/audio-manger-to-deprecate-pavucontrol/audio-struct"),
            "12345678",
        );
        assert_eq!(ver, "dev-12345678");
    }

    #[test]
    fn test_compute_version_rule_4_no_tag_modified_main() {
        let ver = compute_version(None, true, Some("main"), "12345678");
        assert_eq!(ver, "main-12345678");
    }

    #[test]
    fn test_compute_version_rule_4_no_tag_modified_detached_head() {
        let ver_none = compute_version(None, true, None, "12345678");
        assert_eq!(ver_none, "detached-12345678");

        let ver_empty = compute_version(None, true, Some(""), "12345678");
        assert_eq!(ver_empty, "detached-12345678");

        let ver_head = compute_version(None, true, Some("HEAD"), "12345678");
        assert_eq!(ver_head, "detached-12345678");
    }
}
