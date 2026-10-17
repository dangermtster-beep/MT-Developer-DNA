use crate::security::{checked_path, validate_project_root};
use crate::store::{build_plan, snapshot_files};
use crate::{content_hash, MemoryError, MemoryPlan, MemoryStore, SecretRedactor};
use dna_core::{
    DevelopmentPlan, FindingSeverity, MemoryState, ProjectAnalysis, ReviewFinding, ReviewReport,
    WorkflowError, WorkflowMode, WORKFLOW_SCHEMA_VERSION,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisLimits {
    pub max_files: usize,
    pub max_depth: usize,
}

impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            max_files: 512,
            max_depth: 12,
        }
    }
}

pub struct ProjectAnalyzer {
    limits: AnalysisLimits,
}

impl Default for ProjectAnalyzer {
    fn default() -> Self {
        Self::new(AnalysisLimits::default())
    }
}

impl ProjectAnalyzer {
    pub const fn new(limits: AnalysisLimits) -> Self {
        Self { limits }
    }

    pub fn analyze(
        &self,
        root: &Path,
        requested_language: Option<&str>,
    ) -> Result<ProjectAnalysis, MemoryError> {
        let root = validate_project_root(root)?;
        let mut pending = vec![(PathBuf::new(), 0usize)];
        let mut paths = Vec::new();
        let mut fingerprints = Vec::new();
        let mut truncated = false;
        let mut entries_seen = 0usize;

        while let Some((relative_dir, depth)) = pending.pop() {
            if depth > self.limits.max_depth {
                truncated = true;
                continue;
            }
            let directory = root.join(&relative_dir);
            let mut entries = fs::read_dir(&directory)
                .map_err(|source| MemoryError::Io {
                    operation: "V0.4 proje taraması",
                    path: directory.clone(),
                    source,
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|source| MemoryError::Io {
                    operation: "V0.4 proje girdisini okuma",
                    path: directory.clone(),
                    source,
                })?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                if entries_seen >= self.limits.max_files {
                    truncated = true;
                    break;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if ignored(&name) {
                    continue;
                }
                entries_seen += 1;
                let relative = relative_dir.join(&name);
                let relative_text = normalized(&relative);
                let path = checked_path(&root, &relative_text)?;
                let metadata = fs::metadata(&path).map_err(|source| MemoryError::Io {
                    operation: "V0.4 proje girdisi bilgisini okuma",
                    path: path.clone(),
                    source,
                })?;
                if metadata.is_dir() {
                    pending.push((relative, depth + 1));
                } else {
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|value| value.as_nanos())
                        .unwrap_or_default();
                    fingerprints.push(format!("{relative_text}:{}:{modified}", metadata.len()));
                    paths.push(relative_text);
                }
            }
            if entries_seen >= self.limits.max_files {
                break;
            }
        }
        paths.sort();
        fingerprints.sort();

        let (memory_state, recorded_language, memory_warning) = if root.join(".mt-dna").exists() {
            match MemoryStore::load(&root) {
                Ok(snapshot) => (MemoryState::Healthy, Some(snapshot.project.language), None),
                Err(error) => (
                    MemoryState::Invalid,
                    None,
                    Some(format!("Proje hafızası geçersiz: {error}")),
                ),
            }
        } else {
            (MemoryState::Missing, None, None)
        };

        let detected = detected_languages(&paths);
        let mut warnings = memory_warning.into_iter().collect::<Vec<_>>();
        let requested = requested_language.map(normalize_language).transpose()?;
        let selected_language = recorded_language
            .clone()
            .or(requested.clone())
            .or_else(|| (detected.len() == 1).then(|| detected[0].clone()));
        if let (Some(recorded), Some(requested)) = (&recorded_language, &requested) {
            if recorded != requested {
                warnings.push(format!(
                    "İstenen `{requested}` dili kayıtlı `{recorded}` diliyle çelişiyor; kayıtlı seçim korundu."
                ));
            }
        }
        if truncated {
            warnings.push(format!(
                "Tarama {} dosya veya {} derinlik sınırında kısmi kaldı.",
                self.limits.max_files, self.limits.max_depth
            ));
        }

        let source_files = paths
            .iter()
            .filter(|path| language_for(path).is_some())
            .cloned()
            .collect::<Vec<_>>();
        let analysis = ProjectAnalysis {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            project_name: root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_owned(),
            selected_language,
            detected_languages: detected,
            project_type: project_type(&paths),
            build_systems: build_systems(&paths),
            dependency_files: paths
                .iter()
                .filter(|path| is_dependency_file(path))
                .cloned()
                .collect(),
            test_files: paths
                .iter()
                .filter(|path| is_test_file(path))
                .cloned()
                .collect(),
            entry_points: paths
                .iter()
                .filter(|path| is_entry_point(path))
                .cloned()
                .collect(),
            source_files,
            files_scanned: paths.len(),
            max_files: self.limits.max_files,
            truncated,
            source_fingerprint: content_hash(fingerprints.join("\n").as_bytes()),
            memory_state,
            warnings,
        };
        analysis.validate().map_err(workflow_error)?;
        Ok(analysis)
    }

    pub fn review(
        &self,
        project_id: &str,
        analysis: &ProjectAnalysis,
    ) -> Result<ReviewReport, MemoryError> {
        analysis.validate().map_err(workflow_error)?;
        let mut findings = Vec::new();
        if analysis.memory_state == MemoryState::Invalid {
            findings.push(finding(
                FindingSeverity::High,
                Some(".mt-dna"),
                "Proje hafızası doğrulanamadı",
                "MemoryStore şema veya bütünlük denetimi başarısız oldu.",
                "Provider aktarımı eski ya da hatalı bağlam kullanabilir.",
                "`memory check` çalıştırın; onarım planını önizleyip onaylayın.",
            ));
        }
        if analysis.truncated {
            findings.push(finding(
                FindingSeverity::Medium,
                None,
                "Proje taraması kısmi kaldı",
                "Yapılandırılmış dosya veya derinlik sınırına ulaşıldı.",
                "İncelenmeyen dosyalarda ek riskler bulunabilir.",
                "Sınırları kontrollü yükseltin veya kapsamı daraltın.",
            ));
        }
        if analysis.test_files.is_empty() {
            findings.push(finding(
                FindingSeverity::Medium,
                None,
                "Test dosyası kanıtı bulunamadı",
                "Sınırlı taramada bilinen test yolu veya dosya adı görülmedi.",
                "Regresyonlar otomatik olarak yakalanamayabilir.",
                "Mevcut stack için en küçük kritik regresyon testlerini ekleyin.",
            ));
        }
        if analysis.selected_language.is_none() {
            findings.push(finding(
                FindingSeverity::High,
                None,
                "Programlama dili kesinleşmedi",
                "Hafıza, açık istek veya tekil kaynak dili seçimi sağlamadı.",
                "BUILD doğrulama komutları güvenilir biçimde seçilemez.",
                "Kullanıcıdan desteklenen dillerden birini seçmesini isteyin.",
            ));
        }
        for (index, finding) in findings.iter_mut().enumerate() {
            finding.id = format!("REVIEW-{:03}", index + 1);
        }
        let report = ReviewReport {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            project_id: project_id.to_owned(),
            mode: WorkflowMode::Review,
            language: analysis.selected_language.clone(),
            source_fingerprint: analysis.source_fingerprint.clone(),
            scope: "Sınırlı, ignore-aware yapısal inceleme; semantik kod incelemesi provider ajanına aittir."
                .to_owned(),
            findings,
            assumptions: analysis.warnings.clone(),
            files_scanned: analysis.files_scanned,
            truncated: analysis.truncated,
        };
        report.validate().map_err(workflow_error)?;
        Ok(report)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryContext {
    pub project_id: String,
    pub selected_language: String,
    pub current_plan: Option<DevelopmentPlan>,
}

pub struct MemoryBridge;

impl MemoryBridge {
    pub fn load(root: &Path) -> Result<MemoryContext, MemoryError> {
        let root = validate_project_root(root)?;
        let snapshot = MemoryStore::load(&root)?;
        let plan_path = checked_path(&root, ".mt-dna/plans/current.json")?;
        let current_plan = if plan_path.is_file() {
            let contents = fs::read_to_string(&plan_path).map_err(|source| MemoryError::Io {
                operation: "V0.4 planını okuma",
                path: plan_path.clone(),
                source,
            })?;
            let plan: DevelopmentPlan =
                serde_json::from_str(&contents).map_err(|source| MemoryError::Json {
                    path: plan_path.clone(),
                    source,
                })?;
            plan.validate().map_err(workflow_error)?;
            Some(plan)
        } else {
            None
        };
        Ok(MemoryContext {
            project_id: snapshot.project.project_id,
            selected_language: snapshot.project.language,
            current_plan,
        })
    }

    pub fn plan_development_plan(
        root: &Path,
        plan: &DevelopmentPlan,
        markdown: &str,
    ) -> Result<MemoryPlan, MemoryError> {
        plan.validate().map_err(workflow_error)?;
        let json = serde_json::to_vec_pretty(plan).map_err(|source| MemoryError::Json {
            path: root.join(".mt-dna/plans/current.json"),
            source,
        })?;
        Self::plan_artifacts(
            root,
            vec![
                ("plans/current.json".to_owned(), json),
                (
                    "plans/current.md".to_owned(),
                    SecretRedactor::redact(markdown).into_bytes(),
                ),
            ],
        )
    }

    pub fn plan_review(
        root: &Path,
        report: &ReviewReport,
        markdown: &str,
    ) -> Result<MemoryPlan, MemoryError> {
        report.validate().map_err(workflow_error)?;
        let json = serde_json::to_vec_pretty(report).map_err(|source| MemoryError::Json {
            path: root.join(".mt-dna/reports/latest-review.json"),
            source,
        })?;
        Self::plan_artifacts(
            root,
            vec![
                ("reports/latest-review.json".to_owned(), json),
                (
                    "reports/latest-review.md".to_owned(),
                    SecretRedactor::redact(markdown).into_bytes(),
                ),
            ],
        )
    }

    pub fn plan_quality_report(
        root: &Path,
        category: &str,
        report_id: &str,
        json: Vec<u8>,
        markdown: Vec<u8>,
    ) -> Result<MemoryPlan, MemoryError> {
        if !matches!(category, "quality" | "performance" | "security" | "testing")
            || !safe_artifact_id(report_id)
        {
            return Err(MemoryError::InvalidField {
                field: "quality_report_path",
                message: "kategori veya rapor kimliği güvenli değil".to_owned(),
            });
        }
        Self::plan_artifacts(
            root,
            vec![
                (format!("reports/{category}/{report_id}.json"), json),
                (format!("reports/{category}/{report_id}.md"), markdown),
            ],
        )
    }

    pub fn plan_quality_report_with_baseline(
        root: &Path,
        category: &str,
        report_id: &str,
        json: Vec<u8>,
        markdown: Vec<u8>,
        baseline: Vec<u8>,
    ) -> Result<MemoryPlan, MemoryError> {
        if !matches!(category, "quality" | "performance" | "security" | "testing")
            || !safe_artifact_id(report_id)
        {
            return Err(MemoryError::InvalidField {
                field: "quality_report_path",
                message: "kategori veya rapor kimliği güvenli değil".to_owned(),
            });
        }
        Self::plan_artifacts(
            root,
            vec![
                (format!("reports/{category}/{report_id}.json"), json),
                (format!("reports/{category}/{report_id}.md"), markdown),
                ("baselines/performance.json".to_owned(), baseline),
            ],
        )
    }

    pub fn plan_performance_baseline(
        root: &Path,
        json: Vec<u8>,
    ) -> Result<MemoryPlan, MemoryError> {
        Self::plan_artifacts(root, vec![("baselines/performance.json".to_owned(), json)])
    }

    pub fn plan_optimization_report(
        root: &Path,
        fix_id: &str,
        json: Vec<u8>,
        markdown: Vec<u8>,
    ) -> Result<MemoryPlan, MemoryError> {
        if !safe_artifact_id(fix_id) {
            return Err(MemoryError::InvalidField {
                field: "optimization_report_path",
                message: "fix kimliği güvenli değil".to_owned(),
            });
        }
        Self::plan_artifacts(
            root,
            vec![
                (format!("reports/optimization/{fix_id}.json"), json),
                (format!("reports/optimization/{fix_id}.md"), markdown),
            ],
        )
    }

    fn plan_artifacts(
        root: &Path,
        artifacts: Vec<(String, Vec<u8>)>,
    ) -> Result<MemoryPlan, MemoryError> {
        let root = validate_project_root(root)?;
        let mut snapshot = MemoryStore::load(&root)?;
        let scan = crate::ProjectScanner::scan(&root)?;
        let generation = snapshot.manifest.generation;
        snapshot.project.updated_at = crate::time::utc_now();
        let mut files = snapshot_files(&root, &snapshot)?;
        for (path, contents) in artifacts {
            if contents.len() > 2 * 1024 * 1024 {
                return Err(MemoryError::InvalidField {
                    field: "workflow_artifact",
                    message: "tek artifact 2 MiB sınırını aşıyor".to_owned(),
                });
            }
            if SecretRedactor::contains_suspected_secret(&String::from_utf8_lossy(&contents)) {
                return Err(MemoryError::InvalidField {
                    field: "workflow_artifact",
                    message: "şüpheli gizli bilgi içeriyor".to_owned(),
                });
            }
            files.insert(path, contents);
        }
        build_plan(root, Some(generation), files, scan.fingerprint, false)
    }
}

fn safe_artifact_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn finding(
    severity: FindingSeverity,
    file: Option<&str>,
    title: &str,
    evidence: &str,
    impact: &str,
    recommendation: &str,
) -> ReviewFinding {
    ReviewFinding {
        id: String::new(),
        severity,
        file: file.map(str::to_owned),
        line: None,
        title: title.to_owned(),
        evidence: evidence.to_owned(),
        impact: impact.to_owned(),
        recommendation: recommendation.to_owned(),
    }
}

fn ignored(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | ".mt-dna"
            | "target"
            | "node_modules"
            | ".venv"
            | "venv"
            | "bin"
            | "obj"
            | "dist"
            | "build"
            | "coverage"
    ) || name.eq_ignore_ascii_case(".env")
        || name.starts_with(".env.")
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn detected_languages(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| language_for(path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn language_for(path: &str) -> Option<&'static str> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "rs" => Some("rust"),
        "cs" | "csproj" => Some("csharp"),
        "py" => Some("python"),
        "cpp" | "cc" | "cxx" | "h" | "hpp" => Some("cpp"),
        "go" => Some("go"),
        "java" => Some("java"),
        "ts" | "tsx" => Some("typescript"),
        _ => None,
    }
}

fn normalize_language(value: &str) -> Result<String, MemoryError> {
    let normalized = match value.trim().to_ascii_lowercase().as_str() {
        "rust" => "rust",
        "c#" | "csharp" => "csharp",
        "python" => "python",
        "c++" | "cpp" => "cpp",
        "go" | "golang" => "go",
        "java" => "java",
        "typescript" | "ts" => "typescript",
        _ => {
            return Err(MemoryError::InvalidField {
                field: "language",
                message: "rust, csharp, python, cpp, go, java veya typescript olmalıdır".to_owned(),
            })
        }
    };
    Ok(normalized.to_owned())
}

fn is_dependency_file(path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    matches!(
        name,
        "Cargo.toml"
            | "Cargo.lock"
            | "package.json"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "CMakeLists.txt"
    ) || path.ends_with(".csproj")
        || path.ends_with(".sln")
}

fn build_systems(paths: &[String]) -> Vec<String> {
    let mut systems = BTreeSet::new();
    for path in paths {
        let name = Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        match name {
            "Cargo.toml" => {
                systems.insert("cargo");
            }
            "package.json" => {
                systems.insert("npm");
            }
            "pyproject.toml" | "requirements.txt" => {
                systems.insert("python");
            }
            "go.mod" => {
                systems.insert("go");
            }
            "pom.xml" => {
                systems.insert("maven");
            }
            "build.gradle" => {
                systems.insert("gradle");
            }
            "CMakeLists.txt" => {
                systems.insert("cmake");
            }
            _ if path.ends_with(".csproj") || path.ends_with(".sln") => {
                systems.insert("dotnet");
            }
            _ => {}
        }
    }
    systems.into_iter().map(str::to_owned).collect()
}

fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/tests/")
        || lower.starts_with("tests/")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.py")
        || lower.ends_with(".tests.cs")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
}

fn is_entry_point(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "src/main.rs" | "main.py" | "main.go" | "src/main/java" | "src/index.ts"
    ) || lower.ends_with("/program.cs")
        || lower == "program.cs"
}

fn project_type(paths: &[String]) -> String {
    if paths.iter().any(|path| is_entry_point(path)) {
        "cli".to_owned()
    } else if paths.iter().any(|path| path.ends_with("package.json")) {
        "web".to_owned()
    } else {
        "library".to_owned()
    }
}

fn workflow_error(error: WorkflowError) -> MemoryError {
    MemoryError::InvalidField {
        field: "workflow",
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static ID: AtomicU64 = AtomicU64::new(0);

    fn temporary_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mt-dna-v04-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn empty_project_analysis_is_bounded_and_read_only() {
        let root = temporary_project();
        let analysis = ProjectAnalyzer::default().analyze(&root, None).unwrap();
        assert_eq!(analysis.files_scanned, 0);
        assert_eq!(analysis.memory_state, MemoryState::Missing);
        assert!(!root.join(".mt-dna").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_build_outputs_and_reports_partial_scan() {
        let root = temporary_project();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("target/ignored.rs"), "secret").unwrap();
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("extra.rs"), "fn extra() {}").unwrap();
        let analyzer = ProjectAnalyzer::new(AnalysisLimits {
            max_files: 1,
            max_depth: 2,
        });
        let analysis = analyzer.analyze(&root, Some("rust")).unwrap();
        assert!(analysis.truncated);
        assert!(analysis
            .source_files
            .iter()
            .all(|path| !path.contains("target")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rust_and_csharp_reviews_do_not_change_sources() {
        for (file, language) in [("main.rs", "rust"), ("Program.cs", "csharp")] {
            let root = temporary_project();
            fs::write(root.join(file), "source").unwrap();
            let before = fs::read(root.join(file)).unwrap();
            let analyzer = ProjectAnalyzer::default();
            let analysis = analyzer.analyze(&root, Some(language)).unwrap();
            let report = analyzer.review("sample", &analysis).unwrap();
            assert_eq!(report.mode, WorkflowMode::Review);
            assert_eq!(fs::read(root.join(file)).unwrap(), before);
            assert!(!root.join(".mt-dna").exists());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
