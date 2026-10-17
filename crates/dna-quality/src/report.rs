use crate::{MetricCapability, QualityError, QualityReport};
use std::fmt::Write as _;
use std::path::Path;

pub struct QualityReporter;

impl QualityReporter {
    pub fn json(report: &QualityReport, project_root: &Path) -> Result<Vec<u8>, QualityError> {
        report.validate()?;
        let safe = sanitized(report, project_root);
        serde_json::to_vec_pretty(&safe).map_err(|source| QualityError::Json {
            path: Path::new("quality-report.json").to_path_buf(),
            source,
        })
    }

    pub fn markdown(report: &QualityReport, project_root: &Path) -> Result<String, QualityError> {
        report.validate()?;
        let report = sanitized(report, project_root);
        let mut output = format!(
            "# MT Developer DNA kalite raporu\n\n- Şema: {}\n- Proje: {}\n- Dil: {}\n- Çalışma: {:?}\n- Zaman: {}\n- Release gate: {:?}\n- Ortam: {} / {} / {}\n- CPU: {}\n- Toplam RAM: {}\n- İş yükü: {}\n\n",
            report.schema_version,
            report.project,
            report.language,
            report.run_kind,
            report.timestamp_utc,
            report.release_gate,
            report.environment.os,
            report.environment.architecture,
            report.environment.build_mode,
            report.environment.cpu_model.as_deref().unwrap_or("UNAVAILABLE"),
            report.environment.total_memory_bytes.map(|value| format!("{value} byte")).unwrap_or_else(|| "UNAVAILABLE".to_owned()),
            report.environment.workload
        );
        output.push_str("## Kontroller\n\n");
        for check in &report.checks {
            writeln!(output, "### {} — {:?}\n", check.id, check.status).unwrap();
            writeln!(
                output,
                "- Araç: {} ({})",
                check.tool.program,
                check.tool.version.as_deref().unwrap_or("sürüm alınamadı")
            )
            .unwrap();
            writeln!(output, "- Komut: `{}`", check.command.join(" ")).unwrap();
            writeln!(output, "- Süre: {} ms", check.metrics.duration_ms).unwrap();
            metric(
                &mut output,
                "Peak RSS",
                check.metrics.peak_rss_bytes,
                check.metrics.peak_rss_capability,
                "byte",
            );
            metric(
                &mut output,
                "CPU time",
                check.metrics.cpu_time_ms,
                check.metrics.cpu_time_capability,
                "ms",
            );
            writeln!(output, "- Boşta CPU: UNAVAILABLE").unwrap();
            writeln!(output, "- Özet: {}", check.summary).unwrap();
            writeln!(output, "- Kanıt: {}\n", check.evidence).unwrap();
        }
        output.push_str(
            "> Temiz tarama güvenlik garantisi değildir. Ölçülmeyen metriklere sayı atanmadı.\n",
        );
        Ok(output)
    }
}

fn metric(
    output: &mut String,
    name: &str,
    value: Option<u64>,
    capability: MetricCapability,
    unit: &str,
) {
    match (capability, value) {
        (MetricCapability::Measured, Some(value)) => {
            writeln!(output, "- {name}: {value} {unit}").unwrap()
        }
        _ => writeln!(output, "- {name}: UNAVAILABLE").unwrap(),
    }
}

fn sanitized(report: &QualityReport, project_root: &Path) -> QualityReport {
    let mut safe = report.clone();
    let root = project_root.to_string_lossy();
    safe.project = redact(&safe.project, &root);
    safe.environment.workload = redact(&safe.environment.workload, &root);
    for check in &mut safe.checks {
        check.summary = redact(&check.summary, &root);
        check.evidence = redact(&check.evidence, &root);
        check.command = check
            .command
            .iter()
            .map(|value| redact(value, &root))
            .collect();
        if let Some(version) = &mut check.tool.version {
            *version = redact(version, &root);
        }
    }
    safe.notes = safe
        .notes
        .iter()
        .map(|value| redact(value, &root))
        .collect();
    safe
}

fn redact(value: &str, root: &str) -> String {
    let mut output = if root.is_empty() || root == "." {
        value.to_owned()
    } else {
        value.replace(root, "<PROJECT>")
    };
    for line in output.clone().lines() {
        let lower = line.to_ascii_lowercase();
        if [
            "api_key",
            "access_token",
            "password=",
            "token=",
            "sk-proj-",
            "ghp_",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            output = output.replace(line, "[REDACTED]");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CheckCategory, CheckStatus, EnvironmentInfo, GateStatus, PerformanceMetrics,
        QualityRunKind, ToolInfo, QUALITY_SCHEMA_VERSION,
    };

    #[test]
    fn json_and_markdown_redact_paths_and_tokens() {
        let root = Path::new("C:/private/project");
        let report = QualityReport {
            schema_version: QUALITY_SCHEMA_VERSION.to_owned(),
            project: "sample".to_owned(),
            language: "rust".to_owned(),
            mode: "REVIEW".to_owned(),
            run_kind: QualityRunKind::Test,
            timestamp_utc: "2026-10-16T00:00:00Z".to_owned(),
            environment: EnvironmentInfo::current("tests"),
            checks: vec![crate::CheckResult {
                id: "tests".to_owned(),
                category: CheckCategory::Tests,
                status: CheckStatus::Pass,
                required: true,
                exit_code: Some(0),
                tool: ToolInfo {
                    program: "cargo".to_owned(),
                    available: true,
                    version: None,
                },
                command: Vec::new(),
                summary: "ok".to_owned(),
                evidence: "C:/private/project API_KEY=secret".to_owned(),
                stdout_truncated: false,
                stderr_truncated: false,
                metrics: PerformanceMetrics::default(),
            }],
            release_gate: GateStatus::Pass,
            notes: Vec::new(),
        };
        let json = String::from_utf8(QualityReporter::json(&report, root).unwrap()).unwrap();
        let markdown = QualityReporter::markdown(&report, root).unwrap();
        assert!(!json.contains("API_KEY=secret"));
        assert!(!markdown.contains("C:/private/project"));
    }

    #[test]
    fn relative_dot_does_not_redact_punctuation() {
        assert_eq!(redact("cargo 1.99.1", "."), "cargo 1.99.1");
    }
}
