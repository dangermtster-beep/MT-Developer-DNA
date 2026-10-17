use super::*;

pub(crate) fn run_quality(
    args: QualityArgs,
    kind: QualityRunKind,
    save_baseline: bool,
) -> Result<String, CliError> {
    let project_root = fs::canonicalize(&args.project).map_err(|source| CliError::Io {
        operation: "Kalite proje kökünü çözümleme",
        path: args.project.clone(),
        source,
    })?;
    let analyzer = ProjectAnalyzer::default();
    let analysis = analyzer.analyze(&project_root, args.language.as_deref())?;
    let language = analysis
        .selected_language
        .clone()
        .ok_or(MemoryError::InvalidField {
            field: "language",
            message: "kalite kontrolünden önce kullanıcı dili seçmelidir".to_owned(),
        })?;
    let context = memory_context_if_present(&project_root)?;
    let project_id = context
        .as_ref()
        .map(|value| value.project_id.clone())
        .unwrap_or_else(|| infer_project_id(&project_root));
    let policy = load_quality_policy(args.policy.as_deref())?;
    let timestamp = dna_memory::utc_timestamp();
    let cancellation = CancellationToken::default();
    let mut report = QualityEngine::run(
        &project_root,
        &project_id,
        &language,
        kind,
        &policy,
        &timestamp,
        &cancellation,
    )?;

    let current_baseline = if kind == QualityRunKind::Performance {
        performance_baseline(&report, policy.performance.minimum_repetitions)
    } else {
        None
    };
    if let Some(current) = &current_baseline {
        let baseline_path = project_root.join(".mt-dna/baselines/performance.json");
        if baseline_path.is_file() {
            let metadata = fs::symlink_metadata(&baseline_path).map_err(|source| CliError::Io {
                operation: "Performance baseline bilgisini okuma",
                path: baseline_path.clone(),
                source,
            })?;
            if metadata.file_type().is_symlink() {
                return Err(CliError::SymlinkTarget(baseline_path));
            }
            let contents = fs::read_to_string(&baseline_path).map_err(|source| CliError::Io {
                operation: "Performance baseline okuma",
                path: baseline_path.clone(),
                source,
            })?;
            let previous = BaselineStore::from_json(&baseline_path, &contents)?;
            let comparison = BaselineStore::compare(current, &previous);
            report
                .notes
                .push(format!("Baseline karşılaştırması: {}", comparison.reason));
            if comparison.comparable
                && comparison.duration_change_percent.is_some_and(|value| {
                    value > i64::from(policy.performance.regression_warning_percent)
                })
            {
                report.checks.push(performance_regression_check(
                    comparison.duration_change_percent.unwrap_or_default(),
                ));
                report.release_gate = GateEvaluator::evaluate(&report.checks, &policy, false);
            }
        } else {
            report
                .notes
                .push("Karşılaştırılabilir performance baseline bulunamadı.".to_owned());
        }
    }

    let json = QualityReporter::json(&report, &project_root)?;
    let markdown = QualityReporter::markdown(&report, &project_root)?;
    let json_text = String::from_utf8_lossy(&json);
    let rendered = format!("{markdown}\n## JSON\n\n```json\n{json_text}\n```\n");
    if !args.save && !save_baseline {
        return Ok(format!(
            "{rendered}\nRapor salt-okunur üretildi; `.mt-dna` hafızası değiştirilmedi."
        ));
    }
    let report_id = quality_report_id(&timestamp, &json);
    let memory_plan = if save_baseline {
        let baseline = current_baseline.ok_or(QualityError::InvalidField(
            "successful performance baseline",
        ))?;
        let baseline_json = BaselineStore::to_json(&baseline)?;
        MemoryBridge::plan_quality_report_with_baseline(
            &project_root,
            kind.directory(),
            &report_id,
            json,
            markdown.into_bytes(),
            baseline_json,
        )?
    } else {
        MemoryBridge::plan_quality_report(
            &project_root,
            kind.directory(),
            &report_id,
            json,
            markdown.into_bytes(),
        )?
    };
    let memory_result = apply_or_preview_memory(
        &memory_plan,
        args.confirm,
        "V0.5 kalite raporu proje hafızasına kaydedildi",
    )?;
    Ok(format!("{rendered}\n{memory_result}"))
}

pub(crate) fn run_optimize(args: OptimizeArgs) -> Result<String, CliError> {
    let project_root = fs::canonicalize(&args.project).map_err(|source| CliError::Io {
        operation: "Optimizasyon proje kökünü çözümleme",
        path: args.project.clone(),
        source,
    })?;
    let analysis = ProjectAnalyzer::default().analyze(&project_root, args.language.as_deref())?;
    let language = analysis
        .selected_language
        .clone()
        .ok_or(MemoryError::InvalidField {
            field: "language",
            message: "optimizasyondan önce kullanıcı dili seçmelidir".to_owned(),
        })?;
    let context = memory_context_if_present(&project_root)?;
    let project_id = context
        .as_ref()
        .map(|value| value.project_id.clone())
        .unwrap_or_else(|| infer_project_id(&project_root));
    let mode: OptimizationMode = args.mode.into();

    if mode == OptimizationMode::Rollback {
        let fix_id = args.fix_id.as_deref().ok_or_else(|| {
            CliError::InvalidOptimizationArtifact("ROLLBACK için --fix-id gerekir".to_owned())
        })?;
        let mut outcome = OptimizationEngine::rollback(&project_root, fix_id, args.confirm)?;
        outcome.record.tests_run = vec!["language quality adapter".to_owned()];
        let mut notes = Vec::new();
        match optimization_quality_gate(&project_root, &project_id, &language, "rollback") {
            Ok(gate) => {
                outcome
                    .record
                    .gate_results
                    .push(gate_record("rollback", &gate));
                if matches!(gate.release_gate, GateStatus::Fail | GateStatus::Blocked) {
                    outcome.record.status = FindingStatus::VerificationFailed;
                }
                notes.push(format!(
                    "Rollback sonrası V0.5 kalite kapısı: {:?}",
                    gate.release_gate
                ));
            }
            Err(error) => {
                outcome.record.status = FindingStatus::VerificationFailed;
                outcome.record.gate_results.push(GateResultRecord {
                    phase: "rollback".to_owned(),
                    gate: "NOT_RUN".to_owned(),
                    report_timestamp: dna_memory::utc_timestamp(),
                });
                notes.push(format!(
                    "Rollback tamamlandı ancak kalite kapısı çalıştırılamadı: {error}"
                ));
            }
        }
        let report = OptimizationReport {
            schema_version: dna_core::OPTIMIZATION_SCHEMA_VERSION.to_owned(),
            project: project_id,
            language,
            mode,
            timestamp_utc: dna_memory::utc_timestamp(),
            scanned_files: 0,
            scan_truncated: false,
            findings: Vec::new(),
            fix: Some(outcome.record),
            notes,
        };
        return save_and_render_optimization(&project_root, &report, fix_id, None);
    }

    let mut report = OptimizationEngine::analyze(&project_root, &project_id, &language, mode)?;
    if mode == OptimizationMode::Analyze {
        return render_optimization(&project_root, &report, None);
    }

    let finding_id = match args.finding_id.as_deref() {
        Some(value) => value.to_owned(),
        None if mode == OptimizationMode::Preview => report
            .findings
            .iter()
            .find(|finding| finding.auto_fixable)
            .map(|finding| finding.finding_id.clone())
            .ok_or_else(|| {
                CliError::InvalidOptimizationArtifact(
                    "önizlenebilir düşük riskli Auto-Fix bulunamadı".to_owned(),
                )
            })?,
        None => {
            return Err(CliError::InvalidOptimizationArtifact(
                "APPLY için PREVIEW raporundaki --finding-id gerekir".to_owned(),
            ));
        }
    };
    let plan = OptimizationEngine::plan_fix(&project_root, &report, &finding_id)?;
    if mode == OptimizationMode::Preview {
        if let Some(finding) = report
            .findings
            .iter_mut()
            .find(|finding| finding.finding_id == finding_id)
        {
            finding.status = FindingStatus::Previewed;
        }
        return render_optimization(&project_root, &report, Some(&plan));
    }

    if !args.confirm {
        return Err(CliError::OptimizationConsentRequired);
    }
    MemoryStore::load(&project_root)?;
    let pre_gate =
        optimization_quality_gate(&project_root, &project_id, &language, "apply-before")?;
    if matches!(
        pre_gate.release_gate,
        GateStatus::Fail | GateStatus::Blocked
    ) {
        return Err(CliError::InvalidOptimizationArtifact(format!(
            "uygulama öncesi zorunlu kalite kapısı {:?}; hiçbir dosya değiştirilmedi",
            pre_gate.release_gate
        )));
    }
    let mut outcome = OptimizationEngine::apply(&plan, true)?;
    outcome.record.tests_run = vec!["language quality adapter".to_owned()];
    outcome
        .record
        .gate_results
        .push(gate_record("before", &pre_gate));
    let post_gate = optimization_quality_gate(&project_root, &project_id, &language, "apply-after");
    let post_gate_label = match post_gate {
        Ok(post_gate) => {
            outcome
                .record
                .gate_results
                .push(gate_record("after", &post_gate));
            if matches!(
                post_gate.release_gate,
                GateStatus::Fail | GateStatus::Blocked
            ) {
                outcome.record.status = FindingStatus::VerificationFailed;
                report.notes.push(
                    "Auto-Fix uygulandı ancak sonrası kalite kapısı geçmedi; otomatik rollback yapılmadı."
                        .to_owned(),
                );
            }
            format!("{:?}", post_gate.release_gate)
        }
        Err(error) => {
            outcome.record.status = FindingStatus::VerificationFailed;
            outcome.record.gate_results.push(GateResultRecord {
                phase: "after".to_owned(),
                gate: "NOT_RUN".to_owned(),
                report_timestamp: dna_memory::utc_timestamp(),
            });
            report.notes.push(format!(
                "Auto-Fix uygulandı ancak sonrası kalite kapısı çalıştırılamadı: {error}; otomatik rollback yapılmadı."
            ));
            "NOT_RUN".to_owned()
        }
    };
    if let Some(finding) = report
        .findings
        .iter_mut()
        .find(|finding| finding.finding_id == finding_id)
    {
        finding.status = outcome.record.status;
    }
    report.fix = Some(outcome.record);
    report.notes.push(format!(
        "Uygulama öncesi gate {:?}, sonrası gate {}.",
        pre_gate.release_gate, post_gate_label
    ));
    save_and_render_optimization(&project_root, &report, &plan.fix_id, Some(&plan))
}

pub(crate) fn optimization_quality_gate(
    root: &Path,
    project_id: &str,
    language: &str,
    workload: &str,
) -> Result<dna_quality::QualityReport, CliError> {
    let mut policy = QualityPolicy::default();
    policy.timeout_seconds = policy.timeout_seconds.max(300);
    let mut report = QualityEngine::run(
        root,
        project_id,
        language,
        QualityRunKind::Quality,
        &policy,
        &dna_memory::utc_timestamp(),
        &CancellationToken::default(),
    )?;
    report
        .notes
        .push(format!("V0.6 optimizasyon doğrulama aşaması: {workload}"));
    Ok(report)
}

pub(crate) fn gate_record(phase: &str, report: &dna_quality::QualityReport) -> GateResultRecord {
    GateResultRecord {
        phase: phase.to_owned(),
        gate: format!("{:?}", report.release_gate).to_ascii_uppercase(),
        report_timestamp: report.timestamp_utc.clone(),
    }
}

pub(crate) fn render_optimization(
    root: &Path,
    report: &OptimizationReport,
    preview: Option<&FixPlan>,
) -> Result<String, CliError> {
    report.validate()?;
    let markdown = OptimizationEngine::markdown(report, preview);
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?;
    let root_text = root.to_string_lossy();
    Ok(format!(
        "{}\n## JSON\n\n```json\n{}\n```\n\nRapor salt-okunur üretildi; proje ve `.mt-dna` değiştirilmedi.",
        markdown.replace(root_text.as_ref(), "<PROJECT>"),
        json.replace(root_text.as_ref(), "<PROJECT>")
    ))
}

pub(crate) fn save_and_render_optimization(
    root: &Path,
    report: &OptimizationReport,
    fix_id: &str,
    preview: Option<&FixPlan>,
) -> Result<String, CliError> {
    report.validate()?;
    let markdown = OptimizationEngine::markdown(report, preview);
    let json = serde_json::to_vec_pretty(report)
        .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?;
    let plan = MemoryBridge::plan_optimization_report(
        root,
        fix_id,
        json.clone(),
        markdown.clone().into_bytes(),
    )?;
    MemoryStore::apply(&plan)?;
    Ok(format!(
        "{markdown}\n## JSON\n\n```json\n{}\n```\n\nİşlem raporu `.mt-dna/reports/optimization/{fix_id}` altında kaydedildi.",
        String::from_utf8_lossy(&json)
    ))
}

pub(crate) fn read_optimization_report(
    root: &Path,
    fix_id: Option<&str>,
) -> Result<String, CliError> {
    let root = fs::canonicalize(root).map_err(|source| CliError::Io {
        operation: "Optimizasyon rapor kökünü çözümleme",
        path: root.to_path_buf(),
        source,
    })?;
    MemoryStore::load(&root)?;
    let reports = root.join(".mt-dna/reports/optimization");
    let path = if let Some(fix_id) = fix_id {
        if fix_id.is_empty()
            || !fix_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(CliError::InvalidOptimizationArtifact("fix-id".to_owned()));
        }
        reports.join(format!("{fix_id}.json"))
    } else {
        let mut candidates = fs::read_dir(&reports)
            .map_err(|source| CliError::Io {
                operation: "Optimizasyon raporlarını listeleme",
                path: reports.clone(),
                source,
            })?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.pop().ok_or_else(|| {
            CliError::InvalidOptimizationArtifact("kayıtlı optimizasyon raporu yok".to_owned())
        })?
    };
    let metadata = fs::symlink_metadata(&path).map_err(|source| CliError::Io {
        operation: "Optimizasyon rapor bilgisini okuma",
        path: path.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::SymlinkTarget(path));
    }
    let json = fs::read_to_string(&path).map_err(|source| CliError::Io {
        operation: "Optimizasyon JSON raporunu okuma",
        path: path.clone(),
        source,
    })?;
    let report: OptimizationReport = serde_json::from_str(&json)
        .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?;
    report.validate()?;
    let markdown_path = path.with_extension("md");
    let metadata = fs::symlink_metadata(&markdown_path).map_err(|source| CliError::Io {
        operation: "Optimizasyon Markdown rapor bilgisini okuma",
        path: markdown_path.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::SymlinkTarget(markdown_path));
    }
    fs::read_to_string(&markdown_path).map_err(|source| CliError::Io {
        operation: "Optimizasyon Markdown raporunu okuma",
        path: markdown_path,
        source,
    })
}

pub(crate) fn load_quality_policy(path: Option<&Path>) -> Result<QualityPolicy, CliError> {
    match path {
        Some(path) => {
            let contents = fs::read_to_string(path).map_err(|source| CliError::Io {
                operation: "Kalite politikası okuma",
                path: path.to_path_buf(),
                source,
            })?;
            Ok(QualityPolicy::from_json(&contents)?)
        }
        None => Ok(QualityPolicy::default()),
    }
}

pub(crate) fn performance_baseline(
    report: &dna_quality::QualityReport,
    repetitions: usize,
) -> Option<PerformanceBaseline> {
    let check = report
        .checks
        .iter()
        .find(|check| check.id == "performance_workload" && check.status == CheckStatus::Pass)?;
    Some(PerformanceBaseline {
        schema_version: QUALITY_SCHEMA_VERSION.to_owned(),
        environment: report.environment.clone(),
        command: check.command.clone(),
        repetitions,
        median_duration_ms: check.metrics.duration_ms,
        peak_rss_bytes: check.metrics.peak_rss_bytes,
    })
}

pub(crate) fn performance_regression_check(change_percent: i64) -> CheckResult {
    CheckResult {
        id: "performance_regression".to_owned(),
        category: CheckCategory::Performance,
        status: CheckStatus::Warn,
        required: false,
        exit_code: None,
        tool: ToolInfo {
            program: "dna-quality-baseline".to_owned(),
            available: true,
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        },
        command: Vec::new(),
        summary: format!("Medyan süre baseline'a göre %{change_percent} arttı."),
        evidence: "Yalnızca eşdeğer ortam, komut ve iş yükü karşılaştırıldı.".to_owned(),
        stdout_truncated: false,
        stderr_truncated: false,
        metrics: PerformanceMetrics::default(),
    }
}

pub(crate) fn quality_report_id(timestamp: &str, json: &[u8]) -> String {
    let timestamp = timestamp
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let hash = dna_memory::content_hash(json);
    format!(
        "{}-{}",
        timestamp.trim_matches('-'),
        &hash[hash.len() - 8..]
    )
}
