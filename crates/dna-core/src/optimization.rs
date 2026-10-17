use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const OPTIMIZATION_SCHEMA_VERSION: &str = "0.6";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OptimizationMode {
    Analyze,
    Preview,
    Apply,
    Rollback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSource {
    Test,
    Lint,
    StaticAnalysis,
    Security,
    Performance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OptimizationSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FixRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingStatus {
    Open,
    Previewed,
    Applied,
    VerificationFailed,
    RolledBack,
    NotFixable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationFinding {
    pub finding_id: String,
    pub source: FindingSource,
    pub severity: OptimizationSeverity,
    pub confidence: Confidence,
    pub file: String,
    pub line_range: LineRange,
    pub evidence: String,
    pub reproduction: String,
    pub proposed_fix: String,
    pub risk: FixRisk,
    pub verification_steps: Vec<String>,
    pub status: FindingStatus,
    pub auto_fixable: bool,
}

impl OptimizationFinding {
    pub fn validate(&self) -> Result<(), OptimizationError> {
        if !safe_id(&self.finding_id) {
            return Err(OptimizationError::InvalidField("finding_id"));
        }
        if self.file.is_empty()
            || self.file.starts_with(['/', '\\'])
            || self.file.contains("..")
            || self.line_range.start == 0
            || self.line_range.end < self.line_range.start
        {
            return Err(OptimizationError::InvalidField("file/line_range"));
        }
        if self.evidence.trim().is_empty()
            || self.reproduction.trim().is_empty()
            || self.proposed_fix.trim().is_empty()
            || self.verification_steps.is_empty()
        {
            return Err(OptimizationError::InvalidField("finding evidence"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRecord {
    pub explicitly_confirmed: bool,
    pub approved_scope: Vec<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GateResultRecord {
    pub phase: String,
    pub gate: String,
    pub report_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationFixRecord {
    pub schema_version: String,
    pub fix_id: String,
    pub finding_ids: Vec<String>,
    pub target_files: Vec<String>,
    pub patch_digest: String,
    pub before_hashes: Vec<String>,
    pub after_hashes: Vec<String>,
    pub approval_record: ApprovalRecord,
    pub applied_at: String,
    pub tests_run: Vec<String>,
    pub gate_results: Vec<GateResultRecord>,
    pub rollback_artifact: String,
    pub status: FindingStatus,
}

impl OptimizationFixRecord {
    pub fn validate(&self) -> Result<(), OptimizationError> {
        if self.schema_version != OPTIMIZATION_SCHEMA_VERSION
            || !safe_id(&self.fix_id)
            || self.finding_ids.is_empty()
            || self.target_files.is_empty()
            || self.target_files.len() != self.before_hashes.len()
            || self.target_files.len() != self.after_hashes.len()
            || !self.approval_record.explicitly_confirmed
            || self.rollback_artifact.trim().is_empty()
        {
            return Err(OptimizationError::InvalidField("fix record"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationReport {
    pub schema_version: String,
    pub project: String,
    pub language: String,
    pub mode: OptimizationMode,
    pub timestamp_utc: String,
    pub scanned_files: usize,
    pub scan_truncated: bool,
    pub findings: Vec<OptimizationFinding>,
    pub fix: Option<OptimizationFixRecord>,
    pub notes: Vec<String>,
}

impl OptimizationReport {
    pub fn validate(&self) -> Result<(), OptimizationError> {
        if self.schema_version != OPTIMIZATION_SCHEMA_VERSION
            || self.project.trim().is_empty()
            || self.language.trim().is_empty()
            || self.timestamp_utc.trim().is_empty()
        {
            return Err(OptimizationError::InvalidField("optimization report"));
        }
        for finding in &self.findings {
            finding.validate()?;
        }
        if let Some(fix) = &self.fix {
            fix.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OptimizationError {
    #[error("Geçersiz optimizasyon alanı: {0}")]
    InvalidField(&'static str),
    #[error("APPLY için açık kullanıcı onayı gerekir")]
    ApprovalRequired,
    #[error("Geçersiz optimizasyon durum geçişi")]
    InvalidTransition,
}

pub fn validate_transition(
    from: FindingStatus,
    to: FindingStatus,
    approved: bool,
) -> Result<(), OptimizationError> {
    let allowed = match (from, to) {
        (FindingStatus::Open, FindingStatus::Previewed) => true,
        (FindingStatus::Previewed, FindingStatus::Applied) => approved,
        (FindingStatus::Applied, FindingStatus::VerificationFailed) => true,
        (FindingStatus::Applied, FindingStatus::RolledBack)
        | (FindingStatus::VerificationFailed, FindingStatus::RolledBack) => approved,
        _ => false,
    };
    if allowed {
        Ok(())
    } else if matches!(to, FindingStatus::Applied | FindingStatus::RolledBack) && !approved {
        Err(OptimizationError::ApprovalRequired)
    } else {
        Err(OptimizationError::InvalidTransition)
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_rollback_require_explicit_approval() {
        assert_eq!(
            validate_transition(FindingStatus::Previewed, FindingStatus::Applied, false),
            Err(OptimizationError::ApprovalRequired)
        );
        assert!(
            validate_transition(FindingStatus::Previewed, FindingStatus::Applied, true).is_ok()
        );
        assert!(
            validate_transition(FindingStatus::Applied, FindingStatus::RolledBack, true).is_ok()
        );
    }

    #[test]
    fn invalid_transition_and_path_are_rejected() {
        assert_eq!(
            validate_transition(FindingStatus::Open, FindingStatus::Applied, true),
            Err(OptimizationError::InvalidTransition)
        );
        let finding = OptimizationFinding {
            finding_id: "finding-1".to_owned(),
            source: FindingSource::StaticAnalysis,
            severity: OptimizationSeverity::Low,
            confidence: Confidence::High,
            file: "../outside.rs".to_owned(),
            line_range: LineRange { start: 1, end: 1 },
            evidence: "kanıt".to_owned(),
            reproduction: "yeniden üret".to_owned(),
            proposed_fix: "düzelt".to_owned(),
            risk: FixRisk::Low,
            verification_steps: vec!["test".to_owned()],
            status: FindingStatus::Open,
            auto_fixable: true,
        };
        assert_eq!(
            finding.validate(),
            Err(OptimizationError::InvalidField("file/line_range"))
        );
    }
}
