//! Configuration loading for SmartCommit (Module 13).
//!
//! SmartCommit can optionally be configured with a `.smartcommit.toml` file
//! placed at the root of your Git repository. Every field is optional — if the
//! file doesn't exist, or a field is omitted, the built-in default is used.
//!
//! # Resolution order (highest priority first)
//!
//! 1. **CLI flag** — always wins. A flag the user typed on the command line
//!    overrides everything.
//! 2. **`.smartcommit.toml`** — values from the project config file override
//!    the built-in defaults when no CLI flag was given.
//! 3. **Built-in default** — used when neither a CLI flag nor a config value
//!    was provided.
//!
//! # Example `.smartcommit.toml`
//!
//! ```toml
//! [matching]
//! minimum_confidence = 0.70
//! ambiguity_margin   = 0.05
//!
//! [timeline]
//! gap_minutes = 15
//!
//! [commit]
//! require_confirmation = true
//! ```
//!
//! All sections and all fields within them are optional. An empty file is
//! valid and equivalent to having no file at all.

use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur while loading or validating `.smartcommit.toml`.
///
/// All variants produce a human-readable message suitable for printing
/// directly to the user. The file is only read and validated once per
/// command invocation, so these errors appear early, before any Git
/// work begins.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The `.smartcommit.toml` file exists but could not be read from
    /// disk (e.g. a permissions problem). The inner error contains the
    /// OS-level reason.
    #[error("Could not read .smartcommit.toml: {0}")]
    Io(#[from] std::io::Error),

    /// The file was read successfully but is not valid TOML, or contains
    /// keys that do not match the expected structure. The inner string
    /// contains the parser's error message.
    #[error("Could not parse .smartcommit.toml: {0}")]
    Parse(String),

    /// The TOML parsed correctly but a value is out of its allowed range
    /// (e.g. `minimum_confidence = 1.5`, `gap_minutes = 0`). The inner
    /// string names the offending field and its constraint.
    #[error("Invalid configuration: {0}")]
    Validation(String),
}

// ── Raw (deserialization) types ───────────────────────────────────────────────
//
// These private structs mirror the TOML structure exactly, with every field
// wrapped in `Option` so that serde can tell the difference between "field
// was absent" (None) and "field was present with a value" (Some). After
// deserialization they are validated and converted into the public config
// types below, which use plain values and supply defaults for any `None`.

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    matching: Option<RawMatchingConfig>,
    timeline: Option<RawTimelineConfig>,
    commit:   Option<RawCommitConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawMatchingConfig {
    minimum_confidence: Option<f32>,
    ambiguity_margin:   Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTimelineConfig {
    gap_minutes:        Option<u64>,
    random_min_minutes: Option<u64>,
    random_max_minutes: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct RawCommitConfig {
    require_confirmation: Option<bool>,
}

// ── Public config types ───────────────────────────────────────────────────────

/// The fully resolved SmartCommit configuration.
///
/// Produced by [`load_config`]. Every field is guaranteed to hold a valid,
/// in-range value — either from the project's `.smartcommit.toml` or from
/// the built-in defaults. The CLI layer reads this once after repository
/// discovery and uses it to supply defaults wherever no explicit flag was
/// given by the user.
#[derive(Debug, Clone, PartialEq)]
pub struct SmartCommitConfig {
    /// Controls how the file matcher assigns staged files to commit messages.
    pub matching: MatchingConfig,
    /// Controls how the timeline engine spaces commits in time.
    pub timeline: TimelineConfig,
    /// Controls interactive behaviour of the `commit` subcommand.
    pub commit: CommitConfig,
}

/// Thresholds that govern the file-to-message matching step (Module 6/7).
///
/// These values correspond to the `MIN_ASSIGNMENT_SCORE` and
/// `AMBIGUITY_MARGIN` constants that were previously hardcoded in
/// `planner.rs`. Making them configurable lets teams with very different
/// codebase layouts tune the matcher without recompiling.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchingConfig {
    /// The lowest match score at which a file will be assigned to a commit
    /// message. Files whose best candidate scores below this threshold are
    /// left unassigned rather than placed into a low-confidence group.
    ///
    /// Valid range: `0.0..=1.0`. Default: `0.3`.
    pub minimum_confidence: f32,

    /// If a file's top two candidate messages are within this margin of
    /// each other (and the second-best score is real evidence, not zero),
    /// the file is considered ambiguous and left unassigned instead of
    /// being arbitrarily assigned to the higher-scoring message.
    ///
    /// Valid range: `0.0..=1.0`. Default: `0.1`.
    pub ambiguity_margin: f32,
}

/// Timeline spacing used by the `plan` and `commit` subcommands.
///
/// Controls the default gap between consecutive commit timestamps. All
/// values can be overridden at the command line — CLI flags always win
/// over config values, which in turn win over built-in defaults.
///
/// # Fixed vs random mode
///
/// - **Fixed mode** (default): every gap between commits is exactly
///   `gap_minutes`. Activated when neither `random_min_minutes` nor
///   `random_max_minutes` is set in config, and neither
///   `--random-min-minutes` nor `--random-max-minutes` is given on
///   the CLI.
/// - **Random mode**: each gap is chosen randomly from
///   `[random_min_minutes, random_max_minutes]`. Activated when both
///   random fields are set in config, or both random CLI flags are
///   given. The CLI flags take precedence over config: if either
///   `--random-min-minutes` or `--random-max-minutes` is given on the
///   command line, the CLI values are used and config is ignored for
///   the random range entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineConfig {
    /// The default number of minutes between consecutive commit
    /// timestamps when using fixed-interval mode and no
    /// `--interval-minutes` flag is given.
    ///
    /// Must be greater than 0. Default: `10`.
    pub gap_minutes: u64,

    /// The lower bound (inclusive) of the random gap range, in minutes.
    /// Only used when `random_max_minutes` is also set. When both are
    /// set, random mode becomes the default and `gap_minutes` is ignored
    /// unless `--interval-minutes` is given on the CLI.
    ///
    /// Must be greater than 0 and less than or equal to
    /// `random_max_minutes`. Default: `5`.
    pub random_min_minutes: Option<u64>,

    /// The upper bound (inclusive) of the random gap range, in minutes.
    /// Only used when `random_min_minutes` is also set.
    ///
    /// Must be greater than or equal to `random_min_minutes`.
    /// Default: `30`.
    pub random_max_minutes: Option<u64>,
}

/// Behaviour of the interactive confirmation prompt in `smartcommit commit`.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitConfig {
    /// When `true` (the default), `smartcommit commit` asks for explicit
    /// confirmation before creating any commits. Set to `false` to make
    /// non-interactive execution the default (equivalent to always passing
    /// `--yes`). The `--yes` CLI flag still overrides this when it is
    /// provided.
    pub require_confirmation: bool,
}

// ── Default implementations ───────────────────────────────────────────────────

impl Default for SmartCommitConfig {
    fn default() -> Self {
        Self {
            matching: MatchingConfig::default(),
            timeline: TimelineConfig::default(),
            commit:   CommitConfig::default(),
        }
    }
}

impl Default for MatchingConfig {
    /// Returns the same values that were previously hardcoded as
    /// `MIN_ASSIGNMENT_SCORE` (0.3) and `AMBIGUITY_MARGIN` (0.1) in
    /// `planner.rs`. These are the values used when no config file exists
    /// and no CLI flags are given, preserving the pre-Module-13 behaviour
    /// exactly.
    fn default() -> Self {
        Self {
            minimum_confidence: 0.3,
            ambiguity_margin:   0.1,
        }
    }
}

impl Default for TimelineConfig {
    /// Returns the same 10-minute fixed interval that was previously the
    /// hardcoded `--interval-minutes` default in the CLI. `random_min_minutes`
    /// and `random_max_minutes` default to `None`, meaning fixed mode is
    /// used unless the user explicitly opts into random mode via config or
    /// CLI flags.
    fn default() -> Self {
        Self {
            gap_minutes: 10,
            random_min_minutes: None,
            random_max_minutes: None,
        }
    }
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self { require_confirmation: true }
    }
}

// ── Loader ────────────────────────────────────────────────────────────────────

/// Loads the SmartCommit configuration for the repository rooted at
/// `repo_root`.
///
/// Looks for `<repo_root>/.smartcommit.toml`. If the file does not exist,
/// [`SmartCommitConfig::default`] is returned silently — the absence of a
/// config file is not an error. If the file exists but cannot be read,
/// parsed, or passes validation, a [`ConfigError`] is returned.
///
/// # Resolution order
///
/// This function only handles the config-file layer. It is the caller's
/// responsibility to apply CLI-flag overrides on top of the returned config,
/// so that the documented resolution order (CLI > config > default) is
/// respected end-to-end.
///
/// # Errors
///
/// - [`ConfigError::Io`] — file exists but could not be read.
/// - [`ConfigError::Parse`] — file is not valid TOML or has unrecognised
///   structure.
/// - [`ConfigError::Validation`] — a value is out of its allowed range.
pub fn load_config(repo_root: &Path) -> Result<SmartCommitConfig, ConfigError> {
    let config_path = repo_root.join(".smartcommit.toml");

    // A missing file is not an error — just use defaults.
    if !config_path.exists() {
        return Ok(SmartCommitConfig::default());
    }

    let text = std::fs::read_to_string(&config_path)?;

    let raw: RawConfig = toml::from_str(&text)
        .map_err(|e| ConfigError::Parse(e.to_string()))?;

    // ── [matching] ────────────────────────────────────────────────────────────
    let matching = {
        let raw = raw.matching.unwrap_or_default();
        let defaults = MatchingConfig::default();
        let minimum_confidence = raw.minimum_confidence.unwrap_or(defaults.minimum_confidence);
        let ambiguity_margin   = raw.ambiguity_margin.unwrap_or(defaults.ambiguity_margin);

        if !(0.0..=1.0).contains(&minimum_confidence) {
            return Err(ConfigError::Validation(format!(
                "matching.minimum_confidence must be between 0.0 and 1.0, got {minimum_confidence}"
            )));
        }
        if !(0.0..=1.0).contains(&ambiguity_margin) {
            return Err(ConfigError::Validation(format!(
                "matching.ambiguity_margin must be between 0.0 and 1.0, got {ambiguity_margin}"
            )));
        }

        MatchingConfig { minimum_confidence, ambiguity_margin }
    };

    // ── [timeline] ────────────────────────────────────────────────────────────
    let timeline = {
        let raw = raw.timeline.unwrap_or_default();
        let gap_minutes = raw.gap_minutes.unwrap_or(TimelineConfig::default().gap_minutes);

        if gap_minutes == 0 {
            return Err(ConfigError::Validation(
                "timeline.gap_minutes must be greater than 0".to_string(),
            ));
        }

        // random_min and random_max must both be present or both absent.
        // If only one is set, that's a configuration error — the user
        // almost certainly forgot to add the other, and silently ignoring
        // it would produce unexpected fixed-mode behaviour.
        let (random_min_minutes, random_max_minutes) = match (
            raw.random_min_minutes,
            raw.random_max_minutes,
        ) {
            (Some(min), Some(max)) => {
                if min == 0 {
                    return Err(ConfigError::Validation(
                        "timeline.random_min_minutes must be greater than 0".to_string(),
                    ));
                }
                if max < min {
                    return Err(ConfigError::Validation(format!(
                        "timeline.random_max_minutes ({max}) must be >= random_min_minutes ({min})"
                    )));
                }
                (Some(min), Some(max))
            }
            (None, None) => (None, None),
            (Some(_), None) => {
                return Err(ConfigError::Validation(
                    "timeline.random_min_minutes is set but random_max_minutes is missing — both must be provided together".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(ConfigError::Validation(
                    "timeline.random_max_minutes is set but random_min_minutes is missing — both must be provided together".to_string(),
                ));
            }
        };

        TimelineConfig { gap_minutes, random_min_minutes, random_max_minutes }
    };

    // ── [commit] ─────────────────────────────────────────────────────────────
    let commit = {
        let raw = raw.commit.unwrap_or_default();
        CommitConfig {
            require_confirmation: raw
                .require_confirmation
                .unwrap_or(CommitConfig::default().require_confirmation),
        }
    };

    Ok(SmartCommitConfig { matching, timeline, commit })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Creates a unique temporary directory for one test, matching the
    /// pattern used by the rest of smartcommit-core's test suite.
    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-config-test-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir for config test");
        dir
    }

    fn write_config(dir: &PathBuf, content: &str) {
        fs::write(dir.join(".smartcommit.toml"), content)
            .expect("failed to write .smartcommit.toml in temp dir");
    }

    // ── Happy-path tests ──────────────────────────────────────────────────────

    #[test]
    fn returns_defaults_when_no_file_exists() {
        // The absence of .smartcommit.toml must never be an error.
        let dir = temp_dir();
        let config = load_config(&dir).unwrap();
        assert_eq!(config, SmartCommitConfig::default());
    }

    #[test]
    fn parses_a_fully_specified_config_file() {
        let dir = temp_dir();
        write_config(&dir, r#"
[matching]
minimum_confidence = 0.70
ambiguity_margin   = 0.05

[timeline]
gap_minutes = 15

[commit]
require_confirmation = false
"#);
        let config = load_config(&dir).unwrap();

        assert!((config.matching.minimum_confidence - 0.70).abs() < 1e-6);
        assert!((config.matching.ambiguity_margin   - 0.05).abs() < 1e-6);
        assert_eq!(config.timeline.gap_minutes, 15);
        assert!(!config.commit.require_confirmation);
    }

    #[test]
    fn partial_config_fills_missing_fields_with_defaults() {
        // Only [timeline] is specified; the other sections must fall back
        // to their built-in defaults rather than being treated as errors.
        let dir = temp_dir();
        write_config(&dir, "[timeline]\ngap_minutes = 5\n");

        let config = load_config(&dir).unwrap();

        assert_eq!(config.matching, MatchingConfig::default());
        assert_eq!(config.timeline.gap_minutes, 5);
        assert_eq!(config.commit, CommitConfig::default());
    }

    #[test]
    fn empty_file_is_equivalent_to_no_file() {
        // An empty .smartcommit.toml is valid TOML and must return defaults.
        let dir = temp_dir();
        write_config(&dir, "");

        let config = load_config(&dir).unwrap();
        assert_eq!(config, SmartCommitConfig::default());
    }

    // ── Error tests ───────────────────────────────────────────────────────────

    #[test]
    fn rejects_invalid_toml_syntax() {
        let dir = temp_dir();
        write_config(&dir, "this is not valid toml :::::");
        let err = load_config(&dir).unwrap_err();
        assert!(
            matches!(err, ConfigError::Parse(_)),
            "expected Parse error, got: {err}"
        );
    }

    #[test]
    fn rejects_minimum_confidence_above_one() {
        let dir = temp_dir();
        write_config(&dir, "[matching]\nminimum_confidence = 1.5\n");
        let err = load_config(&dir).unwrap_err();
        assert!(
            matches!(err, ConfigError::Validation(_)),
            "expected Validation error, got: {err}"
        );
    }

    #[test]
    fn rejects_minimum_confidence_below_zero() {
        let dir = temp_dir();
        write_config(&dir, "[matching]\nminimum_confidence = -0.1\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_ambiguity_margin_out_of_range() {
        let dir = temp_dir();
        write_config(&dir, "[matching]\nambiguity_margin = 2.0\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_zero_gap_minutes() {
        // A gap of 0 would make all commits land at the same timestamp.
        let dir = temp_dir();
        write_config(&dir, "[timeline]\ngap_minutes = 0\n");
        let err = load_config(&dir).unwrap_err();
        assert!(
            matches!(err, ConfigError::Validation(_)),
            "expected Validation error, got: {err}"
        );
    }

    #[test]
    fn parses_random_range_config() {
        let dir = temp_dir();
        write_config(&dir, r#"
[timeline]
random_min_minutes = 5
random_max_minutes = 30
"#);
        let config = load_config(&dir).unwrap();
        assert_eq!(config.timeline.random_min_minutes, Some(5));
        assert_eq!(config.timeline.random_max_minutes, Some(30));
        // gap_minutes still defaults when not specified
        assert_eq!(config.timeline.gap_minutes, TimelineConfig::default().gap_minutes);
    }

    #[test]
    fn rejects_random_min_without_max() {
        // Both fields must be provided together — one alone is a config error.
        let dir = temp_dir();
        write_config(&dir, "[timeline]\nrandom_min_minutes = 5\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_random_max_without_min() {
        let dir = temp_dir();
        write_config(&dir, "[timeline]\nrandom_max_minutes = 30\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_inverted_random_range() {
        // max < min is always an error regardless of the values.
        let dir = temp_dir();
        write_config(&dir, "[timeline]\nrandom_min_minutes = 30\nrandom_max_minutes = 5\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_zero_random_min() {
        let dir = temp_dir();
        write_config(&dir, "[timeline]\nrandom_min_minutes = 0\nrandom_max_minutes = 10\n");
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }
}
