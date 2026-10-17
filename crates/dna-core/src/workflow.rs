use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const WORKFLOW_SCHEMA_VERSION: &str = "0.4.0";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkflowMode {
    Plan,
    Build,
    Review,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingDecision {
    pub mode: WorkflowMode,
    pub confidence: RouteConfidence,
    pub explicit: bool,
    pub needs_confirmation: bool,
    pub rationale: String,
}

pub struct ModeRouter;

impl ModeRouter {
    pub fn route(request: &str) -> RoutingDecision {
        let normalized = request.trim().to_lowercase();
        if contains_word(&normalized, "plan") || normalized.contains("planla") {
            return route(
                WorkflowMode::Plan,
                RouteConfidence::High,
                true,
                false,
                "PLAN modu açıkça istendi.",
            );
        }
        if contains_word(&normalized, "review")
            || normalized.contains("incele")
            || normalized.contains("kod taraması")
        {
            return route(
                WorkflowMode::Review,
                RouteConfidence::High,
                true,
                false,
                "REVIEW modu veya inceleme açıkça istendi.",
            );
        }
        if contains_word(&normalized, "build")
            || normalized.contains("uygula")
            || normalized.contains("kodla")
            || normalized.contains("geliştir")
            || normalized.contains("düzelt")
        {
            return route(
                WorkflowMode::Build,
                RouteConfidence::High,
                true,
                false,
                "Kaynak kod uygulaması açıkça istendi.",
            );
        }
        if normalized.contains("analiz")
            || normalized.contains("eksiklerini raporla")
            || normalized.contains("değerlendir")
        {
            return route(
                WorkflowMode::Review,
                RouteConfidence::Medium,
                false,
                false,
                "İstek yalnızca analiz/raporlama davranışı içeriyor.",
            );
        }
        if normalized.contains("tasarla")
            || normalized.contains("mimari öner")
            || normalized.contains("yeni proje")
        {
            return route(
                WorkflowMode::Plan,
                RouteConfidence::Medium,
                false,
                false,
                "Yeni proje tasarımı veya mimari planlama istendi.",
            );
        }
        route(
            WorkflowMode::Review,
            RouteConfidence::Low,
            false,
            true,
            "Niyet belirsiz; güvenli salt-okunur REVIEW varsayıldı.",
        )
    }
}

fn route(
    mode: WorkflowMode,
    confidence: RouteConfidence,
    explicit: bool,
    needs_confirmation: bool,
    rationale: &str,
) -> RoutingDecision {
    RoutingDecision {
        mode,
        confidence,
        explicit,
        needs_confirmation,
        rationale: rationale.to_owned(),
    }
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|character: char| !character.is_alphanumeric() && character != '-')
        .any(|word| word == needle)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Missing,
    Healthy,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAnalysis {
    pub schema_version: String,
    pub project_name: String,
    pub selected_language: Option<String>,
    pub detected_languages: Vec<String>,
    pub project_type: String,
    pub build_systems: Vec<String>,
    pub dependency_files: Vec<String>,
    pub test_files: Vec<String>,
    pub entry_points: Vec<String>,
    pub source_files: Vec<String>,
    pub files_scanned: usize,
    pub max_files: usize,
    pub truncated: bool,
    pub source_fingerprint: String,
    pub memory_state: MemoryState,
    pub warnings: Vec<String>,
}

impl ProjectAnalysis {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        ensure_schema(&self.schema_version)?;
        if self.project_name.trim().is_empty() {
            return Err(WorkflowError::InvalidField("project_name"));
        }
        if self.files_scanned > self.max_files {
            return Err(WorkflowError::InvalidField("files_scanned"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RequirementKind {
    Functional,
    NonFunctional,
    Acceptance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub kind: RequirementKind,
    pub text: String,
    pub priority: Priority,
    pub acceptance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequirementSet {
    pub schema_version: String,
    pub selected_language: Option<String>,
    pub requirements: Vec<Requirement>,
    pub constraints: Vec<String>,
    pub open_questions: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct RequirementEngine;

impl RequirementEngine {
    pub fn extract(request: &str, preserved_language: Option<&str>) -> RequirementSet {
        let request = request.trim();
        let requested_language = detect_language(request);
        let selected_language = preserved_language
            .map(str::to_owned)
            .or_else(|| requested_language.map(str::to_owned));
        let mut constraints = vec![
            "Kullanıcının seçtiği programlama dili açık onay olmadan değiştirilmez.".to_owned(),
            "Ölçülmeyen performans değerleri kesin sonuç olarak raporlanmaz.".to_owned(),
        ];
        let mut warnings = Vec::new();
        if let (Some(preserved), Some(requested)) = (preserved_language, requested_language) {
            if preserved != requested {
                warnings.push(format!(
                    "İstekteki `{requested}` dili kayıtlı `{preserved}` diliyle çelişiyor; kayıtlı seçim korunuyor."
                ));
                constraints.push(format!("Etkin dil `{preserved}` olarak korunur."));
            }
        }
        let mut open_questions = Vec::new();
        if selected_language.is_none() {
            open_questions.push("Hangi programlama dili kullanılmalı?".to_owned());
        }
        if request.is_empty() {
            open_questions.push("Projenin temel hedefi nedir?".to_owned());
        }
        let text = if request.is_empty() {
            "Kullanıcı hedefi netleştirilecek.".to_owned()
        } else {
            request.to_owned()
        };
        let requirements = vec![
            Requirement {
                id: "REQ-001".to_owned(),
                kind: RequirementKind::Functional,
                text,
                priority: Priority::High,
                acceptance:
                    "İstenen kullanıcı davranışı gözlemlenebilir bir kabul testiyle doğrulanır."
                        .to_owned(),
            },
            Requirement {
                id: "REQ-002".to_owned(),
                kind: RequirementKind::NonFunctional,
                text: "Değişiklikler mevcut mimariyi ve geriye dönük uyumluluğu korur.".to_owned(),
                priority: Priority::High,
                acceptance:
                    "Mevcut regresyon testleri geçer; kırılan sözleşmeler açıkça raporlanır."
                        .to_owned(),
            },
            Requirement {
                id: "REQ-003".to_owned(),
                kind: RequirementKind::Acceptance,
                text: "Test, lint ve build sonuçları gerçek durumlarıyla raporlanır.".to_owned(),
                priority: Priority::High,
                acceptance:
                    "Başarısız, engelli ve çalıştırılmamış kontroller PASS olarak gösterilmez."
                        .to_owned(),
            },
        ];
        RequirementSet {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            selected_language,
            requirements,
            constraints,
            open_questions,
            warnings,
        }
    }
}

fn detect_language(request: &str) -> Option<&'static str> {
    let normalized = request.to_lowercase();
    [
        ("typescript", "typescript"),
        ("c++", "cpp"),
        ("c#", "csharp"),
        ("python", "python"),
        ("rust", "rust"),
        ("golang", "go"),
        ("java", "java"),
    ]
    .into_iter()
    .find_map(|(needle, id)| normalized.contains(needle).then_some(id))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureRecommendation {
    pub style: String,
    pub components: Vec<String>,
    pub rationale: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct ArchitectureAdvisor;

impl ArchitectureAdvisor {
    pub fn advise(language: Option<&str>, project_type: &str) -> ArchitectureRecommendation {
        let language = language.unwrap_or("belirlenmedi");
        let style = if project_type.contains("cli") {
            "Katmanları sınırlı modüler CLI"
        } else if project_type.contains("desktop") || project_type.contains("wpf") {
            "Sunum ve iş mantığı ayrılmış modüler uygulama"
        } else if project_type.contains("web") {
            "İstemci, uygulama ve veri sınırları ayrılmış web mimarisi"
        } else {
            "En küçük sürdürülebilir modüler mimari"
        };
        ArchitectureRecommendation {
            style: style.to_owned(),
            components: vec![
                "Domain ve doğrulama kuralları".to_owned(),
                "I/O ve dış bağımlılık adaptörleri".to_owned(),
                "Test ve doğrulama sınırı".to_owned(),
            ],
            rationale: vec![
                format!("Seçili dil: {language}."),
                "KISS ve sorumluluk ayrımı; gereksiz soyutlama yok.".to_owned(),
            ],
            warnings: if language == "belirlenmedi" {
                vec![
                    "Mimari kesinleştirilmeden önce dil kullanıcı tarafından seçilmelidir."
                        .to_owned(),
                ]
            } else {
                Vec::new()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanTask {
    pub id: String,
    pub title: String,
    pub depends_on: Vec<String>,
    pub status: TaskStatus,
    pub acceptance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub project_id: String,
    pub mode: WorkflowMode,
    pub language: Option<String>,
    pub project_type: String,
    pub source_fingerprint: String,
    pub requirements: Vec<Requirement>,
    pub constraints: Vec<String>,
    pub open_questions: Vec<String>,
    pub architecture: ArchitectureRecommendation,
    pub risks: Vec<String>,
    pub tasks: Vec<PlanTask>,
    pub verification_commands: Vec<String>,
}

impl DevelopmentPlan {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        ensure_schema(&self.schema_version)?;
        if self.mode != WorkflowMode::Plan || self.plan_id.trim().is_empty() {
            return Err(WorkflowError::InvalidField("mode/plan_id"));
        }
        let mut ids = BTreeSet::new();
        for task in &self.tasks {
            if !ids.insert(&task.id) {
                return Err(WorkflowError::DuplicateId(task.id.clone()));
            }
            if task
                .depends_on
                .iter()
                .any(|dependency| dependency == &task.id)
            {
                return Err(WorkflowError::SelfDependency(task.id.clone()));
            }
        }
        for task in &self.tasks {
            if let Some(missing) = task.depends_on.iter().find(|item| !ids.contains(*item)) {
                return Err(WorkflowError::MissingDependency(missing.clone()));
            }
        }
        Ok(())
    }
}

pub struct DevelopmentPlanner;

impl DevelopmentPlanner {
    pub fn create(
        project_id: &str,
        request: &str,
        analysis: &ProjectAnalysis,
    ) -> Result<DevelopmentPlan, WorkflowError> {
        analysis.validate()?;
        let requirements =
            RequirementEngine::extract(request, analysis.selected_language.as_deref());
        let architecture = ArchitectureAdvisor::advise(
            requirements.selected_language.as_deref(),
            &analysis.project_type,
        );
        let verification_commands =
            verification_commands(requirements.selected_language.as_deref());
        let seed = format!(
            "{project_id}|{}|{}|{}",
            requirements
                .selected_language
                .as_deref()
                .unwrap_or("unspecified"),
            analysis.source_fingerprint,
            request.trim()
        );
        let plan = DevelopmentPlan {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            plan_id: format!("PLAN-{:016x}", stable_hash(seed.as_bytes())),
            project_id: project_id.to_owned(),
            mode: WorkflowMode::Plan,
            language: requirements.selected_language.clone(),
            project_type: analysis.project_type.clone(),
            source_fingerprint: analysis.source_fingerprint.clone(),
            requirements: requirements.requirements,
            constraints: requirements.constraints,
            open_questions: requirements.open_questions,
            architecture,
            risks: analysis.warnings.clone(),
            tasks: vec![
                task(
                    "TASK-001",
                    "Mevcut proje kanıtını doğrula",
                    &[],
                    "Analiz sınırları ve kısmi tarama durumu raporlanır.",
                ),
                task(
                    "TASK-002",
                    "Gereksinimleri ve açık soruları onayla",
                    &["TASK-001"],
                    "Dil ve kapsam kullanıcı seçimiyle uyumludur.",
                ),
                task(
                    "TASK-003",
                    "En küçük bütünlüklü değişikliği uygula",
                    &["TASK-002"],
                    "Çalışan kod gereksiz yeniden yazılmaz.",
                ),
                task(
                    "TASK-004",
                    "Test, lint ve build doğrulamasını çalıştır",
                    &["TASK-003"],
                    "Her komut geçti, kaldı veya çalıştırılmadı olarak kaydedilir.",
                ),
                task(
                    "TASK-005",
                    "Son review ve handoff raporunu hazırla",
                    &["TASK-004"],
                    "Kalan riskler ve sonraki adım açıkça belirtilir.",
                ),
            ],
            verification_commands,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn build_contract(
        plan: DevelopmentPlan,
        approved: bool,
    ) -> Result<BuildContract, WorkflowError> {
        plan.validate()?;
        Ok(BuildContract {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            mode: WorkflowMode::Build,
            plan,
            approved,
            agent_execution_required: true,
            source_changes_performed: false,
            permission_boundary: if approved {
                "Plan kapsamındaki geri alınabilir adımlar provider izinleri içinde uygulanabilir; yıkıcı adımlar ayrıca onay ister."
            } else {
                "Uygulama onaylanmadı; kaynak dosyalar değiştirilmez."
            }
            .to_owned(),
        })
    }
}

fn task(id: &str, title: &str, depends_on: &[&str], acceptance: &str) -> PlanTask {
    PlanTask {
        id: id.to_owned(),
        title: title.to_owned(),
        depends_on: depends_on.iter().map(|value| (*value).to_owned()).collect(),
        status: TaskStatus::Pending,
        acceptance: vec![acceptance.to_owned()],
    }
}

fn verification_commands(language: Option<&str>) -> Vec<String> {
    match language {
        Some("rust") => vec![
            "cargo fmt --all -- --check",
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo test --workspace",
            "cargo build --workspace --release",
        ],
        Some("csharp") => vec!["dotnet test", "dotnet build -c Release"],
        Some("python") => vec!["python -m pytest"],
        Some("cpp") => vec!["cmake --build build", "ctest --test-dir build"],
        Some("go") => vec!["go test ./...", "go build ./..."],
        Some("java") => vec!["mvn test", "mvn package"],
        Some("typescript") => vec!["npm test", "npm run build"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildContract {
    pub schema_version: String,
    pub mode: WorkflowMode,
    pub plan: DevelopmentPlan,
    pub approved: bool,
    pub agent_execution_required: bool,
    pub source_changes_performed: bool,
    pub permission_boundary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    NotRun,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VerificationResult {
    pub command: String,
    pub status: VerificationStatus,
    pub summary: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildReport {
    pub schema_version: String,
    pub plan_id: String,
    pub language: Option<String>,
    pub approved: bool,
    pub changed_files: Vec<String>,
    pub verifications: Vec<VerificationResult>,
    pub partial: bool,
    pub notes: Vec<String>,
}

impl BuildReport {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        ensure_schema(&self.schema_version)?;
        for result in &self.verifications {
            if result.status == VerificationStatus::Passed && result.evidence.trim().is_empty() {
                return Err(WorkflowError::MissingEvidence(result.command.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewFinding {
    pub id: String,
    pub severity: FindingSeverity,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub title: String,
    pub evidence: String,
    pub impact: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewReport {
    pub schema_version: String,
    pub project_id: String,
    pub mode: WorkflowMode,
    pub language: Option<String>,
    pub source_fingerprint: String,
    pub scope: String,
    pub findings: Vec<ReviewFinding>,
    pub assumptions: Vec<String>,
    pub files_scanned: usize,
    pub truncated: bool,
}

impl ReviewReport {
    pub fn validate(&self) -> Result<(), WorkflowError> {
        ensure_schema(&self.schema_version)?;
        if self.mode != WorkflowMode::Review {
            return Err(WorkflowError::InvalidField("mode"));
        }
        let mut ids = BTreeSet::new();
        if self.findings.iter().any(|finding| !ids.insert(&finding.id)) {
            return Err(WorkflowError::DuplicateId("review finding".to_owned()));
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    #[error("Desteklenmeyen workflow şeması: {found}; beklenen: {expected}")]
    UnsupportedSchema { found: String, expected: String },
    #[error("Geçersiz workflow alanı: {0}")]
    InvalidField(&'static str),
    #[error("Yinelenen kararlı kimlik: {0}")]
    DuplicateId(String),
    #[error("Görev kendisine bağımlı olamaz: {0}")]
    SelfDependency(String),
    #[error("Görev bağımlılığı bulunamadı: {0}")]
    MissingDependency(String),
    #[error("Geçti olarak işaretlenen doğrulamanın kanıtı yok: {0}")]
    MissingEvidence(String),
}

fn ensure_schema(value: &str) -> Result<(), WorkflowError> {
    if value == WORKFLOW_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(WorkflowError::UnsupportedSchema {
            found: value.to_owned(),
            expected: WORKFLOW_SCHEMA_VERSION.to_owned(),
        })
    }
}

fn stable_hash(contents: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in contents {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis(language: Option<&str>) -> ProjectAnalysis {
        ProjectAnalysis {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            project_name: "sample".to_owned(),
            selected_language: language.map(str::to_owned),
            detected_languages: language.into_iter().map(str::to_owned).collect(),
            project_type: "cli".to_owned(),
            build_systems: vec!["cargo".to_owned()],
            dependency_files: vec!["Cargo.toml".to_owned()],
            test_files: Vec::new(),
            entry_points: vec!["src/main.rs".to_owned()],
            source_files: vec!["src/main.rs".to_owned()],
            files_scanned: 2,
            max_files: 512,
            truncated: false,
            source_fingerprint: "hash".to_owned(),
            memory_state: MemoryState::Missing,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn routes_explicit_and_ambiguous_requests_safely() {
        assert_eq!(
            ModeRouter::route("plan Rust projesi").mode,
            WorkflowMode::Plan
        );
        assert_eq!(ModeRouter::route("build").mode, WorkflowMode::Build);
        assert_eq!(
            ModeRouter::route("bu projeyi incele").mode,
            WorkflowMode::Review
        );
        let ambiguous = ModeRouter::route("bu projeye bak");
        assert_eq!(ambiguous.mode, WorkflowMode::Review);
        assert!(ambiguous.needs_confirmation);
    }

    #[test]
    fn preserves_recorded_language_on_conflict() {
        let requirements = RequirementEngine::extract("C# ile yeniden yaz", Some("rust"));
        assert_eq!(requirements.selected_language.as_deref(), Some("rust"));
        assert!(!requirements.warnings.is_empty());
    }

    #[test]
    fn plan_is_deterministic_and_schema_validated() {
        let first =
            DevelopmentPlanner::create("sample", "Rust CLI geliştir", &analysis(Some("rust")))
                .unwrap();
        let second =
            DevelopmentPlanner::create("sample", "Rust CLI geliştir", &analysis(Some("rust")))
                .unwrap();
        assert_eq!(first, second);
        first.validate().unwrap();
    }

    #[test]
    fn passed_verification_requires_real_evidence() {
        let report = BuildReport {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            plan_id: "PLAN-1".to_owned(),
            language: Some("rust".to_owned()),
            approved: true,
            changed_files: Vec::new(),
            verifications: vec![VerificationResult {
                command: "cargo test".to_owned(),
                status: VerificationStatus::Passed,
                summary: "geçti".to_owned(),
                evidence: String::new(),
            }],
            partial: false,
            notes: Vec::new(),
        };
        assert!(matches!(
            report.validate(),
            Err(WorkflowError::MissingEvidence(_))
        ));
    }

    #[test]
    fn build_requires_approval_and_preserves_failed_test_state() {
        let plan =
            DevelopmentPlanner::create("sample", "Rust CLI geliştir", &analysis(Some("rust")))
                .unwrap();
        let contract = DevelopmentPlanner::build_contract(plan.clone(), false).unwrap();
        assert!(!contract.approved);
        assert!(!contract.source_changes_performed);
        assert!(contract.permission_boundary.contains("değiştirilmez"));

        let report = BuildReport {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            plan_id: plan.plan_id,
            language: plan.language,
            approved: true,
            changed_files: vec!["src/main.rs".to_owned()],
            verifications: vec![VerificationResult {
                command: "cargo test".to_owned(),
                status: VerificationStatus::Failed,
                summary: "1 test başarısız".to_owned(),
                evidence: "exit code 101".to_owned(),
            }],
            partial: true,
            notes: Vec::new(),
        };
        report.validate().unwrap();
        assert_eq!(report.verifications[0].status, VerificationStatus::Failed);
    }
}
