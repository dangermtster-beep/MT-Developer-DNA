use crate::{EnvironmentInfo, QualityError, QUALITY_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PerformanceBaseline {
    pub schema_version: String,
    pub environment: EnvironmentInfo,
    pub command: Vec<String>,
    pub repetitions: usize,
    pub median_duration_ms: u64,
    pub peak_rss_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BaselineComparison {
    pub comparable: bool,
    pub reason: String,
    pub duration_change_percent: Option<i64>,
    pub peak_rss_change_percent: Option<i64>,
}

pub struct BaselineStore;

impl BaselineStore {
    pub fn from_json(path: &Path, contents: &str) -> Result<PerformanceBaseline, QualityError> {
        let baseline: PerformanceBaseline =
            serde_json::from_str(contents).map_err(|source| QualityError::Json {
                path: path.to_path_buf(),
                source,
            })?;
        if baseline.schema_version != QUALITY_SCHEMA_VERSION || baseline.repetitions == 0 {
            return Err(QualityError::InvalidField("performance baseline"));
        }
        Ok(baseline)
    }

    pub fn to_json(baseline: &PerformanceBaseline) -> Result<Vec<u8>, QualityError> {
        serde_json::to_vec_pretty(baseline).map_err(|source| QualityError::Json {
            path: Path::new("baselines/performance.json").to_path_buf(),
            source,
        })
    }

    pub fn compare(
        current: &PerformanceBaseline,
        baseline: &PerformanceBaseline,
    ) -> BaselineComparison {
        if current.environment != baseline.environment || current.command != baseline.command {
            return BaselineComparison {
                comparable: false,
                reason: "Makine/iş yükü/derleme koşulları eşdeğer değil.".to_owned(),
                duration_change_percent: None,
                peak_rss_change_percent: None,
            };
        }
        BaselineComparison {
            comparable: true,
            reason: "Koşullar eşdeğer.".to_owned(),
            duration_change_percent: percent(
                current.median_duration_ms,
                baseline.median_duration_ms,
            ),
            peak_rss_change_percent: match (current.peak_rss_bytes, baseline.peak_rss_bytes) {
                (Some(current), Some(old)) => percent(current, old),
                _ => None,
            },
        }
    }
}

fn percent(current: u64, baseline: u64) -> Option<i64> {
    if baseline == 0 {
        None
    } else {
        Some((((current as i128 - baseline as i128) * 100) / baseline as i128) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_workload_is_not_compared() {
        let baseline = PerformanceBaseline {
            schema_version: QUALITY_SCHEMA_VERSION.to_owned(),
            environment: EnvironmentInfo::current("a"),
            command: vec!["cargo".to_owned(), "test".to_owned()],
            repetitions: 3,
            median_duration_ms: 100,
            peak_rss_bytes: Some(1000),
        };
        let mut current = baseline.clone();
        current.environment.workload = "b".to_owned();
        assert!(!BaselineStore::compare(&current, &baseline).comparable);
    }
}
