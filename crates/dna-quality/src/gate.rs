use crate::{CheckResult, CheckStatus, GateStatus, QualityPolicy};

pub struct GateEvaluator;

impl GateEvaluator {
    pub fn evaluate(
        checks: &[CheckResult],
        policy: &QualityPolicy,
        require_all_policy_checks: bool,
    ) -> GateStatus {
        let required = |check: &CheckResult| {
            check.required || policy.required_checks.iter().any(|id| id == &check.id)
        };
        if checks
            .iter()
            .any(|check| required(check) && check.status == CheckStatus::Fail)
        {
            return GateStatus::Fail;
        }
        if checks.iter().any(|check| {
            required(check)
                && matches!(check.status, CheckStatus::NotRun | CheckStatus::Unavailable)
        }) {
            return GateStatus::Blocked;
        }
        if require_all_policy_checks
            && policy.required_checks.iter().any(|id| {
                !checks
                    .iter()
                    .any(|check| &check.id == id && check.status == CheckStatus::Pass)
            })
        {
            return GateStatus::Blocked;
        }
        if checks.iter().any(|check| {
            matches!(check.status, CheckStatus::Warn | CheckStatus::Fail)
                || (policy.warnings.iter().any(|id| id == &check.id)
                    && matches!(check.status, CheckStatus::NotRun | CheckStatus::Unavailable))
        }) {
            GateStatus::Warn
        } else {
            GateStatus::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckCategory, PerformanceMetrics, ToolInfo};

    fn check(id: &str, status: CheckStatus, required: bool) -> CheckResult {
        CheckResult {
            id: id.to_owned(),
            category: CheckCategory::Tests,
            status,
            required,
            exit_code: None,
            tool: ToolInfo {
                program: "tool".to_owned(),
                available: true,
                version: None,
            },
            command: Vec::new(),
            summary: "summary".to_owned(),
            evidence: "evidence".to_owned(),
            stdout_truncated: false,
            stderr_truncated: false,
            metrics: PerformanceMetrics::default(),
        }
    }

    #[test]
    fn required_not_run_blocks_and_required_failure_fails() {
        let policy = QualityPolicy::default();
        assert_eq!(
            GateEvaluator::evaluate(&[check("tests", CheckStatus::NotRun, true)], &policy, false),
            GateStatus::Blocked
        );
        assert_eq!(
            GateEvaluator::evaluate(&[check("tests", CheckStatus::Fail, true)], &policy, false),
            GateStatus::Fail
        );
    }

    #[test]
    fn strict_release_requires_every_policy_check() {
        let policy = QualityPolicy::default();
        assert_eq!(
            GateEvaluator::evaluate(&[check("tests", CheckStatus::Pass, true)], &policy, true),
            GateStatus::Blocked
        );
    }
}
