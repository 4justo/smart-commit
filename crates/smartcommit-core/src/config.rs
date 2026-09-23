use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Could not read .smc.toml: {0}")]
    Io(#[from] std::io::Error),

    #[error("Could not parse .smc.toml: {0}")]
    Parse(String),

    #[error("Invalid configuration: {0}")]
    Validation(String),
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    matching: Option<RawMatchingConfig>,
    timeline: Option<RawTimelineConfig>,
    commit: Option<RawCommitConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawMatchingConfig {
    minimum_confidence: Option<f32>,
    ambiguity_margin: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTimelineConfig {
    gap_minutes: Option<u64>,
    random_min_minutes: Option<u64>,
    random_max_minutes: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct RawCommitConfig {
    require_confirmation: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SmartCommitConfig {
    pub matching: MatchingConfig,
    pub timeline: TimelineConfig,
    pub commit: CommitConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchingConfig {
    pub minimum_confidence: f32,

    pub ambiguity_margin: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineConfig {
    pub gap_minutes: u64,

    pub random_min_minutes: Option<u64>,

    pub random_max_minutes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitConfig {
    pub require_confirmation: bool,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            minimum_confidence: 0.3,
            ambiguity_margin: 0.1,
        }
    }
}

impl Default for TimelineConfig {
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
        Self {
            require_confirmation: true,
        }
    }
}

pub fn load_config(repo_root: &Path) -> Result<SmartCommitConfig, ConfigError> {
    let config_path = repo_root.join(".smc.toml");

    if !config_path.exists() {
        return Ok(SmartCommitConfig::default());
    }

    let text = std::fs::read_to_string(&config_path)?;

    let raw: RawConfig = toml::from_str(&text).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let matching = {
        let raw = raw.matching.unwrap_or_default();
        let defaults = MatchingConfig::default();
        let minimum_confidence = raw
            .minimum_confidence
            .unwrap_or(defaults.minimum_confidence);
        let ambiguity_margin = raw.ambiguity_margin.unwrap_or(defaults.ambiguity_margin);

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

        MatchingConfig {
            minimum_confidence,
            ambiguity_margin,
        }
    };

    let timeline = {
        let raw = raw.timeline.unwrap_or_default();
        let gap_minutes = raw
            .gap_minutes
            .unwrap_or(TimelineConfig::default().gap_minutes);

        if gap_minutes == 0 {
            return Err(ConfigError::Validation(
                "timeline.gap_minutes must be greater than 0".to_string(),
            ));
        }

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

        TimelineConfig {
            gap_minutes,
            random_min_minutes,
            random_max_minutes,
        }
    };

    let commit = {
        let raw = raw.commit.unwrap_or_default();
        CommitConfig {
            require_confirmation: raw
                .require_confirmation
                .unwrap_or(CommitConfig::default().require_confirmation),
        }
    };

    Ok(SmartCommitConfig {
        matching,
        timeline,
        commit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-config-test-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir for config test");
        dir
    }

    fn write_config(dir: &Path, content: &str) {
        fs::write(dir.join(".smc.toml"), content).expect("failed to write .smc.toml in temp dir");
    }

    #[test]
    fn returns_defaults_when_no_file_exists() {
        let dir = temp_dir();
        let config = load_config(&dir).unwrap();
        assert_eq!(config, SmartCommitConfig::default());
    }

    #[test]
    fn parses_a_fully_specified_config_file() {
        let dir = temp_dir();
        write_config(
            &dir,
            r#"
[matching]
minimum_confidence = 0.70
ambiguity_margin   = 0.05

[timeline]
gap_minutes = 15

[commit]
require_confirmation = false
"#,
        );
        let config = load_config(&dir).unwrap();

        assert!((config.matching.minimum_confidence - 0.70).abs() < 1e-6);
        assert!((config.matching.ambiguity_margin - 0.05).abs() < 1e-6);
        assert_eq!(config.timeline.gap_minutes, 15);
        assert!(!config.commit.require_confirmation);
    }

    #[test]
    fn partial_config_fills_missing_fields_with_defaults() {
        let dir = temp_dir();
        write_config(&dir, "[timeline]\ngap_minutes = 5\n");

        let config = load_config(&dir).unwrap();

        assert_eq!(config.matching, MatchingConfig::default());
        assert_eq!(config.timeline.gap_minutes, 5);
        assert_eq!(config.commit, CommitConfig::default());
    }

    #[test]
    fn empty_file_is_equivalent_to_no_file() {
        let dir = temp_dir();
        write_config(&dir, "");

        let config = load_config(&dir).unwrap();
        assert_eq!(config, SmartCommitConfig::default());
    }

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
        write_config(
            &dir,
            r#"
[timeline]
random_min_minutes = 5
random_max_minutes = 30
"#,
        );
        let config = load_config(&dir).unwrap();
        assert_eq!(config.timeline.random_min_minutes, Some(5));
        assert_eq!(config.timeline.random_max_minutes, Some(30));
        assert_eq!(
            config.timeline.gap_minutes,
            TimelineConfig::default().gap_minutes
        );
    }

    #[test]
    fn rejects_random_min_without_max() {
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
        write_config(
            &dir,
            "[timeline]\nrandom_min_minutes = 30\nrandom_max_minutes = 5\n",
        );
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_zero_random_min() {
        let dir = temp_dir();
        write_config(
            &dir,
            "[timeline]\nrandom_min_minutes = 0\nrandom_max_minutes = 10\n",
        );
        let err = load_config(&dir).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }
}
