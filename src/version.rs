//! SemVer 2.0.0 precedence, dependency-free.
//!
//! Release tooling outside the crate (the preflight, the manifest generator,
//! and the release workflow) delegates to `scripts/semver.py`. This module is
//! the in-crate equivalent so `tests/contract.rs` can assert ADR-0001 with the
//! real ordering instead of a hand-rolled approximation.
//!
//! The defect this replaces compared only the `major.minor.patch` core:
//! `0.3.2-dev.1` and `0.3.2` looked equal, so a prerelease could be ranked
//! ahead of its own release and the contract test still passed.
//!
//! Precedence rules implemented (specification section 11):
//!
//! * the numeric core is compared field by field;
//! * a version *with* a prerelease has lower precedence than one without
//!   (`1.0.0-rc.1 < 1.0.0`);
//! * prerelease identifiers are compared left to right; numeric identifiers
//!   compare numerically and rank below alphanumeric ones; a larger set of
//!   identifiers wins when all shared ones are equal;
//! * build metadata is validated but carries no precedence.

use std::cmp::Ordering;

/// A parsed semantic version.
///
/// The raw text is retained for diagnostics. Note that `PartialEq`/`Ord`
/// compare *precedence*, so two versions that differ only in build metadata
/// are equal: the specification gives them identical precedence.
#[derive(Debug, Clone)]
pub struct Version {
    raw: String,
    core: [u64; 3],
    prerelease: Option<Vec<String>>,
}

impl Version {
    /// The original text.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The `major.minor.patch` numbers.
    pub fn core(&self) -> [u64; 3] {
        self.core
    }

    /// The dot-separated prerelease identifiers, if any.
    pub fn prerelease(&self) -> Option<&[String]> {
        self.prerelease.as_deref()
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let core = self.core.cmp(&other.core);
        if core != Ordering::Equal {
            return core;
        }
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            // Section 11.3: a prerelease has lower precedence than the release.
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => compare_identifiers(left, right),
        }
    }
}

fn compare_identifiers(left: &[String], right: &[String]) -> Ordering {
    for (a, b) in left.iter().zip(right.iter()) {
        let (a_numeric, b_numeric) = (is_numeric(a), is_numeric(b));
        let ordering = match (a_numeric, b_numeric) {
            (true, true) => compare_numeric(a, b),
            // Section 11.4.3: numeric identifiers rank below alphanumeric ones.
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => a.as_bytes().cmp(b.as_bytes()),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    // Section 11.4.4: a larger set of identifiers has higher precedence.
    left.len().cmp(&right.len())
}

/// Compare two digit strings without parsing: both are free of leading zeros,
/// so a longer one is larger and equal lengths compare lexicographically.
fn compare_numeric(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn is_numeric(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

/// Parse `value`, returning `None` unless it is a valid semantic version.
pub fn parse(value: &str) -> Option<Version> {
    let (rest, build) = match value.split_once('+') {
        Some((rest, build)) => (rest, Some(build)),
        None => (value, None),
    };
    if let Some(build) = build {
        // Validated for well-formedness, then deliberately unused: build
        // metadata does not participate in precedence.
        if !valid_identifiers(build, false) {
            return None;
        }
    }

    let (core, prerelease) = match rest.split_once('-') {
        Some((core, pre)) => {
            if !valid_identifiers(pre, true) {
                return None;
            }
            (core, Some(pre.split('.').map(str::to_owned).collect()))
        }
        None => (rest, None),
    };

    let mut fields = core.split('.');
    let mut numbers = [0u64; 3];
    for slot in &mut numbers {
        *slot = parse_core_number(fields.next()?)?;
    }
    if fields.next().is_some() {
        return None;
    }

    Some(Version {
        raw: value.to_owned(),
        core: numbers,
        prerelease,
    })
}

/// `true` when `value` is valid semver.
pub fn is_valid(value: &str) -> bool {
    parse(value).is_some()
}

/// Compare two version strings by precedence.
///
/// Returns `None` if either side is not valid semver, so callers cannot
/// mistake an unparseable version for an ordering.
pub fn compare(left: &str, right: &str) -> Option<Ordering> {
    Some(parse(left)?.cmp(&parse(right)?))
}

/// ADR-0001: `language` must not have higher precedence than `release`.
///
/// Returns `None` if either side is not valid semver.
pub fn language_not_ahead(language: &str, release: &str) -> Option<bool> {
    Some(compare(language, release)? != Ordering::Greater)
}

fn parse_core_number(field: &str) -> Option<u64> {
    if !is_numeric(field) {
        return None;
    }
    // Section 2: no leading zeroes in the numeric core.
    if field.len() > 1 && field.starts_with('0') {
        return None;
    }
    field.parse().ok()
}

fn valid_identifiers(value: &str, prerelease: bool) -> bool {
    if value.is_empty() {
        return false;
    }
    value.split('.').all(|identifier| {
        if identifier.is_empty() || !identifier.bytes().all(is_identifier_byte) {
            return false;
        }
        // Section 9: numeric prerelease identifiers must not have leading
        // zeroes. Build metadata identifiers are unrestricted in this respect.
        if prerelease && is_numeric(identifier) && identifier.len() > 1 {
            return !identifier.starts_with('0');
        }
        true
    })
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn ord(left: &str, right: &str) -> Ordering {
        compare(left, right).expect("both sides are valid semver")
    }

    #[test]
    fn core_is_compared_numerically() {
        assert_eq!(ord("1.0.0", "0.9.9"), Ordering::Greater);
        assert_eq!(ord("0.3.1", "0.3.10"), Ordering::Less);
        assert_eq!(ord("10.0.0", "9.0.0"), Ordering::Greater);
        assert_eq!(ord("1.2.3", "1.2.3"), Ordering::Equal);
    }

    #[test]
    fn prerelease_ranks_below_its_release() {
        // The inverse of what `sort -V` reports.
        assert_eq!(ord("1.0.0-rc.1", "1.0.0"), Ordering::Less);
        assert_eq!(ord("0.3.2-dev.1", "0.3.2"), Ordering::Less);
        assert_eq!(ord("0.3.2", "0.3.2-dev.1"), Ordering::Greater);
    }

    #[test]
    fn prerelease_identifiers_order_by_the_specification() {
        assert_eq!(ord("1.0.0-alpha", "1.0.0-beta"), Ordering::Less);
        assert_eq!(ord("1.0.0-alpha.1", "1.0.0-alpha.2"), Ordering::Less);
        // Numeric identifiers compare numerically, not lexicographically.
        assert_eq!(ord("1.0.0-rc.2", "1.0.0-rc.10"), Ordering::Less);
        // Numeric ranks below alphanumeric.
        assert_eq!(ord("1.0.0-1", "1.0.0-alpha"), Ordering::Less);
        // A larger set of identifiers wins when the shared prefix is equal.
        assert_eq!(ord("1.0.0-alpha", "1.0.0-alpha.1"), Ordering::Less);
        // The example sequence from section 11.4.1.
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in ordered.windows(2) {
            assert_eq!(ord(pair[0], pair[1]), Ordering::Less, "{pair:?}");
        }
    }

    #[test]
    fn build_metadata_has_no_precedence() {
        assert_eq!(ord("1.0.0+a", "1.0.0+b"), Ordering::Equal);
        assert_eq!(ord("1.0.0", "1.0.0+build.7"), Ordering::Equal);
        assert_eq!(ord("1.0.0-rc.1+x", "1.0.0-rc.1+y"), Ordering::Equal);
    }

    #[test]
    fn invalid_versions_are_rejected() {
        for invalid in [
            "",
            "1",
            "1.0",
            "1.0.0.0",
            "1.0.0-",
            "1.0.0+",
            "1.0.0-01",
            "01.0.0",
            "1.0.0-rc..1",
            "1.0.0-rc.01",
            "1.0.0-rc_1",
            "v1.0.0",
            "1.0.0 ",
            "1.0.0-ä",
        ] {
            assert!(!is_valid(invalid), "{invalid:?} should be rejected");
            assert!(compare(invalid, "1.0.0").is_none(), "{invalid:?}");
        }
    }

    #[test]
    fn numeric_core_overflow_is_rejected() {
        assert!(!is_valid("18446744073709551616.0.0"));
    }

    #[test]
    fn language_not_ahead_enforces_adr_0001() {
        assert_eq!(language_not_ahead("0.3.1", "0.3.1"), Some(true));
        assert_eq!(language_not_ahead("0.3.1", "0.3.2"), Some(true));
        // A prerelease of the release is still behind it.
        assert_eq!(language_not_ahead("0.3.2-dev.7", "0.3.2"), Some(true));
        assert_eq!(language_not_ahead("0.3.2", "0.3.1"), Some(false));
        // `sort -V` would have accepted this pair.
        assert_eq!(language_not_ahead("0.3.2-rc.1", "0.3.2"), Some(true));
        assert_eq!(language_not_ahead("broken", "0.3.1"), None);
    }

    #[test]
    fn raw_text_is_preserved() {
        let version = parse("0.3.2-dev.7+build.9").expect("valid");
        assert_eq!(version.as_str(), "0.3.2-dev.7+build.9");
        assert_eq!(version.core(), [0, 3, 2]);
        assert_eq!(version.prerelease().map(<[String]>::len), Some(2));
    }
}
