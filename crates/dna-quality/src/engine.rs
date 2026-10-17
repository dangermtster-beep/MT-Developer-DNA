use crate::{
    CancellationToken, CheckCategory, CheckResult, CheckRunner, CheckStatus, EnvironmentInfo,
    GateEvaluator, LanguageQualityAdapter, LocalSecurityScanner, MetricCapability,
    PerformanceMetrics, QualityError, QualityPolicy, QualityReport, QualityRunKind, RunnerLimits,
    ToolInfo, QUALITY_SCHEMA_VERSION,
};
use std::path::Path;
use std::time::Duration;

pub struct QualityEngine;

impl QualityEngine {
    pub fn run(
        root: &Path,
        project: &str,
        language: &str,
        kind: QualityRunKind,
        policy: &QualityPolicy,
        timestamp_utc: &str,
        cancellation: &CancellationToken,
    ) -> Result<QualityReport, QualityError> {
        policy.validate()?;
        let selection = LanguageQualityAdapter::discover(root, language, kind);
        let runner = CheckRunner::new(RunnerLimits {
            timeout: Duration::from_secs(policy.timeout_seconds),
            max_output_bytes: policy.max_output_kb_per_check.saturating_mul(1024),
            sample_interval: Duration::from_millis(25),
        });
        let mut checks = selection.unavailable;
        if matches!(
            kind,
            QualityRunKind::Security | QualityRunKind::ReleaseCheck
        ) {
            checks.push(LocalSecurityScanner::scan(root)?);
        }
        if kind == QualityRunKind::Performance {
            if let Some(spec) = selection.commands.first() {
                checks.push(run_performance(
                    root,
                    spec,
                    &runner,
                    policy.performance.minimum_repetitions,
                    cancellation,
                )?);
            } else if checks.is_empty() {
                checks.push(unavailable_performance("Ölçülebilir iş yükü bulunamadı."));
            }
        } else {
            for spec in &selection.commands {
                if spec.mutates_source {
                    checks.push(not_run_mutating(spec));
                } else {
                    checks.push(runner.run(root, spec, cancellation)?);
                }
            }
        }
        checks.sort_by(|left, right| left.id.cmp(&right.id));
        let release_gate =
            GateEvaluator::evaluate(&checks, policy, kind == QualityRunKind::ReleaseCheck);
        let report = QualityReport {
            schema_version: QUALITY_SCHEMA_VERSION.to_owned(),
            project: project.to_owned(),
            language: language.to_owned(),
            mode: "REVIEW".to_owned(),
            run_kind: kind,
            timestamp_utc: timestamp_utc.to_owned(),
            environment: EnvironmentInfo::current(format!("{:?}", kind)),
            checks,
            release_gate,
            notes: vec![
                "Kontroller talep üzerine çalıştı; arka plan daemon/polling başlatılmadı."
                    .to_owned(),
            ],
        };
        report.validate()?;
        Ok(report)
    }
}

fn run_performance(
    root: &Path,
    spec: &crate::CommandSpec,
    runner: &CheckRunner,
    repetitions: usize,
    cancellation: &CancellationToken,
) -> Result<CheckResult, QualityError> {
    let mut warm_up = runner.run(root, spec, cancellation)?;
    if warm_up.status != CheckStatus::Pass {
        warm_up.id = "performance_workload".to_owned();
        warm_up.category = CheckCategory::Performance;
        warm_up.required = true;
        warm_up.summary = format!("Warm-up başarısız: {}", warm_up.summary);
        return Ok(warm_up);
    }
    let mut results = Vec::new();
    for _ in 0..repetitions {
        let mut result = runner.run(root, spec, cancellation)?;
        if result.status != CheckStatus::Pass {
            result.id = "performance_workload".to_owned();
            result.category = CheckCategory::Performance;
            result.required = true;
            return Ok(result);
        }
        results.push(result);
    }
    let mut durations = results
        .iter()
        .map(|item| item.metrics.duration_ms)
        .collect::<Vec<_>>();
    durations.sort_unstable();
    let median = durations[durations.len() / 2];
    let peak_rss = results
        .iter()
        .filter_map(|item| item.metrics.peak_rss_bytes)
        .max();
    let cpu_time = results
        .iter()
        .filter_map(|item| item.metrics.cpu_time_ms)
        .max();
    let first = &results[0];
    Ok(CheckResult {
        id: "performance_workload".to_owned(),
        category: CheckCategory::Performance,
        status: CheckStatus::Pass,
        required: true,
        exit_code: Some(0),
        tool: first.tool.clone(),
        command: first.command.clone(),
        summary: format!(
            "Warm-up ve {repetitions} ölçüm tekrarı tamamlandı; medyan süre {median} ms."
        ),
        evidence: format!("Tekrar süreleri (ms): {durations:?}"),
        stdout_truncated: results.iter().any(|item| item.stdout_truncated),
        stderr_truncated: results.iter().any(|item| item.stderr_truncated),
        metrics: PerformanceMetrics {
            duration_ms: median,
            peak_rss_bytes: peak_rss,
            peak_rss_capability: if peak_rss.is_some() {
                MetricCapability::Measured
            } else {
                MetricCapability::Unavailable
            },
            cpu_time_ms: cpu_time,
            cpu_time_capability: if cpu_time.is_some() {
                MetricCapability::Measured
            } else {
                MetricCapability::Unavailable
            },
            idle_cpu_percent: None,
            idle_cpu_capability: MetricCapability::Unavailable,
            measurement_method: first.metrics.measurement_method.clone(),
        },
    })
}

fn unavailable_performance(reason: &str) -> CheckResult {
    CheckResult {
        id: "performance_workload".to_owned(),
        category: CheckCategory::Performance,
        status: CheckStatus::Unavailable,
        required: true,
        exit_code: None,
        tool: ToolInfo {
            program: String::new(),
            available: false,
            version: None,
        },
        command: Vec::new(),
        summary: reason.to_owned(),
        evidence: reason.to_owned(),
        stdout_truncated: false,
        stderr_truncated: false,
        metrics: PerformanceMetrics::default(),
    }
}

fn not_run_mutating(spec: &crate::CommandSpec) -> CheckResult {
    CheckResult {
        id: spec.id.clone(),
        category: spec.category,
        status: CheckStatus::NotRun,
        required: spec.required,
        exit_code: None,
        tool: ToolInfo {
            program: spec.program.clone(),
            available: false,
            version: None,
        },
        command: std::iter::once(spec.program.clone())
            .chain(spec.args.iter().cloned())
            .collect(),
        summary: "REVIEW modunda kaynak değiştiren kontrol çalıştırılmadı.".to_owned(),
        evidence: "Auto-fix/formatlama yetkisi verilmedi.".to_owned(),
        stdout_truncated: false,
        stderr_truncated: false,
        metrics: PerformanceMetrics::default(),
    }
}
