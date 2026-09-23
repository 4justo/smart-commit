use crate::{ChangedFile, CommitPlan};
use chrono::{DateTime, Duration, FixedOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedCommitTime {
    pub commit_index: usize,
    pub timestamp: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineMode {
    Fixed { interval: Duration },
    Random { min: Duration, max: Duration },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimelineError {
    #[error("cannot generate a timeline for zero commits")]
    ZeroCommits,

    #[error("Fixed timeline interval must be positive")]
    InvalidInterval,

    #[error(
        "Random timeline range is invalid: min ({min:?}) must be >= zero and <= max ({max:?})"
    )]
    InvalidRange { min: Duration, max: Duration },

    #[error(
        "commit {commit_index} would be timestamped {timestamp} which is after now ({now}); pass allow_future to permit this"
    )]
    FutureTimestamp {
        commit_index: usize,
        timestamp: DateTime<FixedOffset>,
        now: DateTime<FixedOffset>,
    },
}

pub trait RandomSource {
    fn next_unit(&mut self) -> f64;
}

pub struct SeededRandomSource {
    state: u64,
}

impl SeededRandomSource {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl RandomSource for SeededRandomSource {
    fn next_unit(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        (z >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

pub struct SystemRandomSource {
    inner: SeededRandomSource,
}

impl SystemRandomSource {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self {
            inner: SeededRandomSource::new(seed),
        }
    }
}

impl Default for SystemRandomSource {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomSource for SystemRandomSource {
    fn next_unit(&mut self) -> f64 {
        self.inner.next_unit()
    }
}

pub fn generate_timeline(
    commit_count: usize,
    mode: TimelineMode,
    start: Option<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
    allow_future: bool,
    rng: &mut dyn RandomSource,
) -> Result<Vec<PlannedCommitTime>, TimelineError> {
    if commit_count == 0 {
        return Err(TimelineError::ZeroCommits);
    }

    validate_mode(&mode)?;

    let gap_count = commit_count - 1;
    let gaps: Vec<Duration> = (0..gap_count).map(|_| next_gap(&mode, rng)).collect();

    let timestamps: Vec<DateTime<FixedOffset>> = match start {
        Some(start) => {
            let mut times = Vec::with_capacity(commit_count);
            let mut current = start;
            times.push(current);
            for gap in &gaps {
                current += *gap;
                times.push(current);
            }
            times
        }
        None => {
            // Anchor the last commit at `now`, walk backward. Every
            // subsequent element is `now` minus an accumulating
            // non-negative offset, so nothing here can land after
            // `now` — no validation needed in this branch.
            let mut times = vec![now; commit_count];
            let mut current = now;
            for i in (0..gap_count).rev() {
                current -= gaps[i];
                times[i] = current;
            }
            times
        }
    };

    if !allow_future {
        for (commit_index, timestamp) in timestamps.iter().enumerate() {
            if *timestamp > now {
                return Err(TimelineError::FutureTimestamp {
                    commit_index,
                    timestamp: *timestamp,
                    now,
                });
            }
        }
    }

    Ok(timestamps
        .into_iter()
        .enumerate()
        .map(|(commit_index, timestamp)| PlannedCommitTime {
            commit_index,
            timestamp,
        })
        .collect())
}

fn validate_mode(mode: &TimelineMode) -> Result<(), TimelineError> {
    match mode {
        TimelineMode::Fixed { interval } => {
            if *interval <= Duration::zero() {
                return Err(TimelineError::InvalidInterval);
            }
        }
        TimelineMode::Random { min, max } => {
            if *min < Duration::zero() || *max < Duration::zero() || min > max {
                return Err(TimelineError::InvalidRange {
                    min: *min,
                    max: *max,
                });
            }
        }
    }
    Ok(())
}

fn next_gap(mode: &TimelineMode, rng: &mut dyn RandomSource) -> Duration {
    match mode {
        TimelineMode::Fixed { interval } => *interval,
        TimelineMode::Random { min, max } => {
            let min_ms = min.num_milliseconds() as f64;
            let max_ms = max.num_milliseconds() as f64;
            let frac = rng.next_unit().clamp(0.0, 1.0);
            let ms = min_ms + (max_ms - min_ms) * frac;
            Duration::milliseconds(ms.round() as i64)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedCommitGroup {
    pub message: String,
    pub files: Vec<ChangedFile>,
    pub confidence: f32,
    pub timestamp: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedCommitPlan {
    pub groups: Vec<TimedCommitGroup>,
    pub unassigned: Vec<ChangedFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ApplyTimelineError {
    #[error("timeline has {timestamps} timestamp(s) but the plan has {groups} group(s)")]
    CountMismatch { timestamps: usize, groups: usize },
}

pub fn apply_timeline_to_plan(
    plan: CommitPlan,
    timestamps: &[PlannedCommitTime],
) -> Result<TimedCommitPlan, ApplyTimelineError> {
    if timestamps.len() != plan.groups.len() {
        return Err(ApplyTimelineError::CountMismatch {
            timestamps: timestamps.len(),
            groups: plan.groups.len(),
        });
    }

    let mut sorted_timestamps = timestamps.to_vec();
    sorted_timestamps.sort_by_key(|t| t.commit_index);

    let groups = plan
        .groups
        .into_iter()
        .zip(sorted_timestamps.iter())
        .map(|(group, t)| TimedCommitGroup {
            message: group.message,
            files: group.files,
            confidence: group.confidence,
            timestamp: t.timestamp,
        })
        .collect();

    Ok(TimedCommitPlan {
        groups,
        unassigned: plan.unassigned,
        warnings: plan.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(rfc3339: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(rfc3339).expect("test fixture datetime should parse")
    }

    #[test]
    fn fixed_intervals_are_exact() {
        let start = dt("2026-09-01T09:00:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00"); // safely in the future of every generated timestamp
        let mut rng = SeededRandomSource::new(1);

        let result = generate_timeline(
            3,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            Some(start),
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].timestamp, start);
        assert_eq!(result[1].timestamp, start + Duration::minutes(10));
        assert_eq!(result[2].timestamp, start + Duration::minutes(20));
        assert_eq!(result[0].commit_index, 0);
        assert_eq!(result[2].commit_index, 2);
    }

    #[test]
    fn random_gaps_stay_within_bounds() {
        let start = dt("2026-09-01T09:00:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(42);

        let result = generate_timeline(
            20,
            TimelineMode::Random {
                min: Duration::minutes(5),
                max: Duration::minutes(15),
            },
            Some(start),
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        for pair in result.windows(2) {
            let gap = pair[1].timestamp - pair[0].timestamp;
            assert!(
                gap >= Duration::minutes(5) && gap <= Duration::minutes(15),
                "gap {gap:?} was outside [5, 15] minutes"
            );
        }
    }

    #[test]
    fn timestamps_are_chronologically_ordered() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(7);

        // No custom start — exercises the backward-from-now branch.
        let result = generate_timeline(
            10,
            TimelineMode::Random {
                min: Duration::minutes(1),
                max: Duration::minutes(60),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        for pair in result.windows(2) {
            assert!(
                pair[0].timestamp < pair[1].timestamp,
                "timestamps must be strictly increasing by commit_index"
            );
        }
        assert_eq!(
            result.last().unwrap().timestamp,
            now,
            "with no custom start, the last commit should land exactly on `now`"
        );
    }

    #[test]
    fn single_commit_with_no_start_lands_on_now() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let result = generate_timeline(
            1,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].timestamp, now);
        assert_eq!(result[0].commit_index, 0);
    }

    #[test]
    fn single_commit_with_custom_start_lands_on_start() {
        let start = dt("2026-09-01T09:00:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let result = generate_timeline(
            1,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            Some(start),
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        assert_eq!(result[0].timestamp, start);
    }

    #[test]
    fn custom_start_is_respected_exactly() {
        let start = dt("2026-09-01T09:00:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(3);

        let result = generate_timeline(
            5,
            TimelineMode::Fixed {
                interval: Duration::minutes(2),
            },
            Some(start),
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        assert_eq!(result[0].timestamp, start);
    }

    #[test]
    fn rejects_an_inverted_random_range() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let err = generate_timeline(
            3,
            TimelineMode::Random {
                min: Duration::minutes(15),
                max: Duration::minutes(5),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .unwrap_err();

        assert!(matches!(err, TimelineError::InvalidRange { .. }));
    }

    #[test]
    fn rejects_a_zero_or_negative_fixed_interval() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let err = generate_timeline(
            3,
            TimelineMode::Fixed {
                interval: Duration::zero(),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .unwrap_err();

        assert_eq!(err, TimelineError::InvalidInterval);
    }

    #[test]
    fn rejects_zero_commits() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let err = generate_timeline(
            0,
            TimelineMode::Fixed {
                interval: Duration::minutes(1),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .unwrap_err();

        assert_eq!(err, TimelineError::ZeroCommits);
    }

    #[test]
    fn a_custom_start_that_would_run_past_now_is_rejected_by_default() {
        let start = dt("2026-11-30T23:55:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        // 3 commits, 10-minute gaps, starting 5 minutes before `now`:
        // commit 0 = 23:55 (fine), commit 1 = 00:05 — already past
        // `now`, so that's the first offender the check reports.
        let err = generate_timeline(
            3,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            Some(start),
            now,
            false,
            &mut rng,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            TimelineError::FutureTimestamp {
                commit_index: 1,
                ..
            }
        ));
    }

    #[test]
    fn allow_future_permits_a_start_that_runs_past_now() {
        let start = dt("2026-11-30T23:55:00+03:00");
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let result = generate_timeline(
            3,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            Some(start),
            now,
            true, // allow_future
            &mut rng,
        )
        .expect("should succeed when future timestamps are explicitly allowed");

        assert!(result[2].timestamp > now);
    }

    #[test]
    fn no_start_never_produces_a_future_timestamp_even_with_a_huge_range() {
        // Regression-style check: even with a very large random range
        // and many commits, the backward-from-now construction must
        // never exceed `now`.
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(99);

        let result = generate_timeline(
            50,
            TimelineMode::Random {
                min: Duration::hours(1),
                max: Duration::hours(48),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        assert!(result.iter().all(|t| t.timestamp <= now));
    }

    fn changed_file(path: &str) -> ChangedFile {
        ChangedFile {
            path: std::path::PathBuf::from(path),
            change_kind: crate::ChangeKind::Modified,
            old_path: None,
        }
    }

    #[test]
    fn applies_timeline_to_a_plan_pairing_by_commit_index() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let plan = CommitPlan {
            groups: vec![
                crate::CommitGroup {
                    message: "Fix authentication".to_string(),
                    files: vec![changed_file("src/auth.rs")],
                    confidence: 0.9,
                },
                crate::CommitGroup {
                    message: "Improve dashboard".to_string(),
                    files: vec![changed_file("src/dashboard.rs")],
                    confidence: 0.85,
                },
            ],
            unassigned: vec![changed_file("src/utils.rs")],
            warnings: vec![],
        };

        let timestamps = generate_timeline(
            2,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        let timed = apply_timeline_to_plan(plan, &timestamps).expect("counts should match");

        assert_eq!(timed.groups.len(), 2);
        assert_eq!(timed.groups[0].message, "Fix authentication");
        assert_eq!(timed.groups[0].timestamp, timestamps[0].timestamp);
        assert_eq!(timed.groups[1].message, "Improve dashboard");
        assert_eq!(timed.groups[1].timestamp, timestamps[1].timestamp);
        assert_eq!(timed.unassigned, vec![changed_file("src/utils.rs")]);
    }

    #[test]
    fn rejects_a_timestamp_count_mismatch() {
        let now = dt("2026-12-01T00:00:00+03:00");
        let mut rng = SeededRandomSource::new(1);

        let plan = CommitPlan {
            groups: vec![crate::CommitGroup {
                message: "Fix authentication".to_string(),
                files: vec![],
                confidence: 0.0,
            }],
            unassigned: vec![],
            warnings: vec![],
        };

        // Two timestamps requested, but the plan only has one group.
        let timestamps = generate_timeline(
            2,
            TimelineMode::Fixed {
                interval: Duration::minutes(10),
            },
            None,
            now,
            false,
            &mut rng,
        )
        .expect("should succeed");

        let err = apply_timeline_to_plan(plan, &timestamps).unwrap_err();
        assert_eq!(
            err,
            ApplyTimelineError::CountMismatch {
                timestamps: 2,
                groups: 1
            }
        );
    }
}
