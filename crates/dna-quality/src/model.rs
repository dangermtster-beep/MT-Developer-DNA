use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub const QUALITY_SCHEMA_VERSION: &str = "0.5";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityRunKind {
    Test,
    Quality,
    Performance,
    Security,
    ReleaseCheck,
}

impl QualityRunKind {
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Test => "testing",
            Self::Quality | Self::ReleaseCheck => "quality",
            Self::Performance => "performance",
            Self::Security => "security",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    Tests,
    Format,
    Lint,
    Build,
    Security,
    Performance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    NotRun,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum GateStatus {
    Pass,
    Warn,
    Fail,
    Blocked,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MetricCapability {
    Measured,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub id: String,
    pub category: CheckCategory,
    pub program: String,
    pub args: Vec<String>,
    pub required: bool,
    pub mutates_source: bool,
}

impl CommandSpec {
    pub fn validate(&self) -> Result<(), QualityError> {
        if self.id.trim().is_empty() || self.program.trim().is_empty() {
            return Err(QualityError::InvalidField("command id/program"));
        }
        if self.program.contains(['\n', '\r'])
            || self.args.iter().any(|arg| arg.contains(['\0', '\n', '\r']))
        {
            return Err(QualityError::UnsafeCommand(self.program.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolInfo {
    pub program: String,
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct PerformanceMetrics {
    pub duration_ms: u64,
    pub peak_rss_bytes: Option<u64>,
    pub peak_rss_capability: MetricCapability,
    pub cpu_time_ms: Option<u64>,
    pub cpu_time_capability: MetricCapability,
    pub idle_cpu_percent: Option<u32>,
    pub idle_cpu_capability: MetricCapability,
    pub measurement_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    pub id: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub required: bool,
    pub exit_code: Option<i32>,
    pub tool: ToolInfo,
    pub command: Vec<String>,
    pub summary: String,
    pub evidence: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentInfo {
    pub os: String,
    pub architecture: String,
    pub cpu_model: Option<String>,
    pub total_memory_bytes: Option<u64>,
    pub build_mode: String,
    pub logical_cpu_count: Option<usize>,
    pub workload: String,
}

impl EnvironmentInfo {
    pub fn current(workload: impl Into<String>) -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            cpu_model: std::env::var("PROCESSOR_IDENTIFIER").ok(),
            total_memory_bytes: total_memory_bytes(),
            build_mode: "release-check".to_owned(),
            logical_cpu_count: std::thread::available_parallelism().ok().map(usize::from),
            workload: workload.into(),
        }
    }
}

#[cfg(windows)]
fn total_memory_bytes() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(status.ullTotalPhys)
}

#[cfg(not(windows))]
fn total_memory_bytes() -> Option<u64> {
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct QualityPolicy {
    pub schema_version: String,
    pub required_checks: Vec<String>,
    pub warnings: Vec<String>,
    pub performance: PerformancePolicy,
    pub timeout_seconds: u64,
    pub max_output_kb_per_check: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PerformancePolicy {
    pub regression_warning_percent: u32,
    pub minimum_repetitions: usize,
}

impl Default for PerformancePolicy {
    fn default() -> Self {
        Self {
            regression_warning_percent: 15,
            minimum_repetitions: 3,
        }
    }
}

impl Default for QualityPolicy {
    fn default() -> Self {
        Self {
            schema_version: QUALITY_SCHEMA_VERSION.to_owned(),
            required_checks: vec![
                "build".to_owned(),
                "tests".to_owned(),
                "security_critical".to_owned(),
            ],
            warnings: vec![
                "format".to_owned(),
                "lint".to_owned(),
                "performance_regression".to_owned(),
            ],
            performance: PerformancePolicy::default(),
            timeout_seconds: 600,
            max_output_kb_per_check: 512,
        }
    }
}

impl QualityPolicy {
    pub fn from_json(contents: &str) -> Result<Self, QualityError> {
        if contents.trim().is_empty() {
            return Err(QualityError::InvalidPolicy("politika boş".to_owned()));
        }
        let policy: Self = serde_json::from_str(contents)
            .map_err(|error| QualityError::InvalidPolicy(error.to_string()))?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), QualityError> {
        if self.schema_version != QUALITY_SCHEMA_VERSION {
            return Err(QualityError::InvalidPolicy(format!(
                "şema {} yerine {} olmalı",
                self.schema_version, QUALITY_SCHEMA_VERSION
            )));
        }
        if self.timeout_seconds == 0
            || self.max_output_kb_per_check == 0
            || self.performance.minimum_repetitions == 0
            || self.performance.minimum_repetitions > 20
        {
            return Err(QualityError::InvalidPolicy(
                "timeout/çıktı/tekrar sınırları geçersiz".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QualityReport {
    pub schema_version: String,
    pub project: String,
    pub language: String,
    pub mode: String,
    pub run_kind: QualityRunKind,
    pub timestamp_utc: String,
    pub environment: EnvironmentInfo,
    pub checks: Vec<CheckResult>,
    pub release_gate: GateStatus,
    pub notes: Vec<String>,
}

impl QualityReport {
    pub fn validate(&self) -> Result<(), QualityError> {
        if self.schema_version != QUALITY_SCHEMA_VERSION {
            return Err(QualityError::InvalidField("schema_version"));
        }
        if self.project.trim().is_empty() || self.language.trim().is_empty() {
            return Err(QualityError::InvalidField("project/language"));
        }
        if self
            .checks
            .iter()
            .any(|check| check.status == CheckStatus::Pass && check.evidence.trim().is_empty())
        {
            return Err(QualityError::InvalidField("PASS evidence"));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum QualityError {
    #[error("Geçersiz kalite alanı: {0}")]
    InvalidField(&'static str),
    #[error("Geçersiz kalite politikası: {0}")]
    InvalidPolicy(String),
    #[error("Güvenli olmayan komut reddedildi: {0}")]
    UnsafeCommand(String),
    #[error("Proje kökü mevcut bir klasör olmalıdır: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("{operation} başarısız ({path}): {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Kalite JSON hatası ({path}): {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specification_policy_shape_is_supported() {
        let policy = QualityPolicy::from_json(
            r#"{
                "schema_version":"0.5",
                "required_checks":["build","tests","security_critical"],
                "warnings":["format","lint","performance_regression"],
                "performance":{"regression_warning_percent":15,"minimum_repetitions":3},
                "timeout_seconds":600,
                "max_output_kb_per_check":512
            }"#,
        )
        .unwrap();
        assert_eq!(policy.performance.minimum_repetitions, 3);
    }

    #[test]
    fn empty_and_broken_policy_are_rejected() {
        assert!(QualityPolicy::from_json("").is_err());
        assert!(QualityPolicy::from_json("{").is_err());
    }
}
