mod install;
mod optimization;
mod provider_status;

use clap::{Parser, Subcommand, ValueEnum};
use dna_core::{
    DevelopmentPlanner, FindingStatus, GateResultRecord, LanguageProfile, OptimizationError,
    OptimizationMode, OptimizationReport, Profile, ProfileLoadError, WorkflowError,
};
use dna_deploy::{
    apply_install as apply_deployment, detect_all, detect_provider_with_path,
    doctor_with_installation_path as deploy_doctor, format_diagnostics,
    format_plan as format_deployment_plan, plan_install_with_global_skill_path as plan_deployment,
    resolve_target_root, rollback as rollback_deployment, status as deployment_status,
    ConflictPolicy, DeployError, DeploymentReport, DiagnosticResult, InstallScope, ProviderKey,
};
use dna_memory::{
    format_memory_plan, AuditSeverity, DecisionInput, InitRequest, MemoryBridge, MemoryError,
    MemoryStore, ProjectAnalyzer, ProviderId, UpdateRequest,
};
use dna_quality::{
    BaselineStore, CancellationToken, CheckCategory, CheckResult, CheckStatus, GateEvaluator,
    GateStatus, PerformanceBaseline, PerformanceMetrics, QualityEngine, QualityError,
    QualityPolicy, QualityReporter, QualityRunKind, ToolInfo, QUALITY_SCHEMA_VERSION,
};
use dna_render::{render_package, Provider, ProviderPackage, RenderError, ReportRenderer};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use install::{
    apply_install, format_install_plan, plan_install, rollback_install, InstallChange, InstallPlan,
    InstallReport, InstallStatus, RollbackReport,
};
pub use optimization::{ApplyOutcome, FixPlan, OptimizationEngine};
pub use provider_status::provider_status_report;

mod quality_commands;
use quality_commands::*;
mod deploy_commands;
use deploy_commands::*;
mod export;
pub use export::*;
#[cfg(test)]
mod tests;

#[derive(Debug, Parser)]
#[command(
    name = "mt-dna",
    version,
    about = "Kişisel yazılım standartları ve provider paket üretim aracı"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Kurulu provider CLI sürümlerini ve çağırma biçimlerini salt-okunur denetler.
    Providers {
        #[command(subcommand)]
        command: Option<ProvidersCommand>,
    },
    /// Evrensel ve dile özgü profil dosyalarını listeler, gösterir veya doğrular.
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    /// JSON profilini yükler ve doğrular; dosya değiştirmez.
    Validate {
        #[arg(short, long)]
        profile: PathBuf,
    },
    /// Provider çıktılarını terminalde gösterir; dosya değiştirmez.
    Preview {
        #[arg(short, long)]
        profile: PathBuf,
        #[arg(short = 'P', long, value_enum, default_value_t = ProviderSelection::All)]
        provider: ProviderSelection,
    },
    /// Paketleri yeni provider alt klasörlerine yazar; var olan dosyaları ezmez.
    Export {
        #[arg(short, long)]
        profile: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short = 'P', long, value_enum, default_value_t = ProviderSelection::All)]
        provider: ProviderSelection,
    },
    /// Mevcut projeyi salt-okunur analiz eder ve deterministik geliştirme planı üretir.
    Plan {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        request: String,
        #[arg(long)]
        language: Option<String>,
        /// Planı mevcut `.mt-dna` hafızasına eklemeyi talep eder.
        #[arg(long)]
        save: bool,
        /// Hafıza diff'ini onaylayarak uygular; tek başına yazma yapmaz.
        #[arg(long)]
        confirm: bool,
    },
    /// Onaylı plan için provider ajanının uygulayacağı BUILD sözleşmesini üretir.
    Build {
        #[arg(long)]
        project: PathBuf,
        #[arg(long, default_value = "Onaylı geliştirme planını uygula")]
        request: String,
        #[arg(long)]
        language: Option<String>,
        /// Uygulama yetkisini açıkça verir; çekirdek yine kaynak dosyası yazmaz.
        #[arg(long)]
        approve: bool,
    },
    /// Projeyi varsayılan olarak salt-okunur inceler ve kanıtlı yapısal bulgular üretir.
    Review {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        language: Option<String>,
        /// Raporu mevcut `.mt-dna` hafızasına eklemeyi talep eder.
        #[arg(long)]
        save: bool,
        #[arg(long)]
        confirm: bool,
    },
    /// Dil adapter'ının test komutunu timeout ve sınırlı çıktıyla çalıştırır.
    Test(QualityArgs),
    /// Test, format ve lint kontrollerini salt-okunur biçimde çalıştırır.
    Quality(QualityArgs),
    /// Test iş yükünü sınırlı tekrarlarla ölçer; ölçülemeyen metrikleri uydurmaz.
    Performance {
        #[command(flatten)]
        args: QualityArgs,
        /// Karşılaştırılabilir sonucu `.mt-dna/baselines/performance.json` olarak planlar.
        #[arg(long)]
        save_baseline: bool,
    },
    /// Yerel sır taraması ve kurulu ekosistem güvenlik aracını çalıştırır.
    Security(QualityArgs),
    /// Test, kalite, build ve güvenlik kontrollerinden release gate hesaplar.
    ReleaseCheck(QualityArgs),
    /// Akıllı kod analizi, güvenli önizleme, onaylı Auto-Fix ve rollback çalıştırır.
    Optimize(OptimizeArgs),
    /// Tek bir bulguyu açık onayla uygular; optimize --mode apply kısayoludur.
    Fix(FixArgs),
    /// Kaydedilmiş bir V0.6 optimizasyon işlem raporunu gösterir.
    OptimizationReport {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        fix_id: Option<String>,
    },
    /// Universal provider deployment, sync, sürüm ve rollback motoru.
    Deploy {
        #[command(subcommand)]
        command: DeployCommand,
    },
    /// Provider, kurulum, registry, hafıza ve çağırma mekanizmasını denetler.
    Doctor {
        #[arg(short = 'P', long, value_enum, default_value_t = ProviderSelection::All)]
        provider: ProviderSelection,
        #[arg(long, value_enum, default_value_t = DeploymentScope::Project)]
        scope: DeploymentScope,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Provider executable veya kurulum klasörünü açıkça seçer.
        #[arg(long, value_name = "PATH")]
        provider_path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Provider dosyaları için plan/diff üretir; yalnızca açık onayla kurar.
    Install {
        #[arg(short, long)]
        profile: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(short = 'P', long, value_enum)]
        provider: ProviderSelection,
        /// Provider dosyalarını değiştirme niyetini açıkça bildirir.
        #[arg(long)]
        confirm: bool,
    },
    /// Önceki bir kurulum yedeğini açık onayla geri yükler.
    Rollback {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        backup: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Proje kökünde güvenli ve provider bağımsız `.mt-dna` hafızası başlatır.
    Init {
        #[arg(short, long)]
        profile: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        project_id: Option<String>,
        #[arg(long, default_value = "cli")]
        project_type: String,
        #[arg(long, value_delimiter = ',', default_value = "windows")]
        platform: Vec<String>,
        #[arg(long, value_enum)]
        provider: MemoryProvider,
        #[arg(long)]
        confirm: bool,
    },
    /// Doğrulanmış proje hafızasından kısa çalışma özeti çıkarır.
    Resume {
        #[arg(long)]
        project: PathBuf,
    },
    /// İlerleme ve isteğe bağlı mimari karar kaydını güvenli biçimde planlar/yazar.
    Update {
        #[arg(long)]
        project: PathBuf,
        #[arg(long, value_enum)]
        provider: MemoryProvider,
        #[arg(long)]
        completed: Vec<String>,
        #[arg(long)]
        in_progress: Vec<String>,
        #[arg(long)]
        next: Vec<String>,
        #[arg(long)]
        blocker: Vec<String>,
        #[arg(long)]
        test_result: Vec<String>,
        #[arg(long)]
        decision: Option<String>,
        #[arg(long)]
        decision_context: Option<String>,
        #[arg(long)]
        decision_option: Vec<String>,
        #[arg(long)]
        decision_reason: Option<String>,
        #[arg(long)]
        affected_file: Vec<String>,
        #[arg(long)]
        confirm: bool,
    },
    /// Ortak `.mt-dna` kaynağından hedef provider için kısa aktarım özeti üretir.
    Handoff {
        #[arg(long)]
        project: PathBuf,
        #[arg(long, value_enum)]
        provider: MemoryProvider,
        #[arg(long)]
        confirm: bool,
    },
    /// Proje hafızasını denetler veya önizlemeli/yedekli onarım uygular.
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
}

#[derive(Debug, clap::Args)]
pub struct QualityArgs {
    #[arg(long)]
    pub project: PathBuf,
    #[arg(long)]
    pub language: Option<String>,
    /// İsteğe bağlı V0.5 kalite politikası JSON dosyası.
    #[arg(long)]
    pub policy: Option<PathBuf>,
    /// JSON ve Markdown raporunu mevcut `.mt-dna` hafızasına eklemeyi planlar.
    #[arg(long)]
    pub save: bool,
    /// Hafıza diff'ini açıkça uygular.
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Debug, clap::Args)]
pub struct OptimizeArgs {
    #[arg(long)]
    pub project: PathBuf,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long, value_enum, default_value_t = OptimizeMode::Analyze)]
    pub mode: OptimizeMode,
    #[arg(long)]
    pub finding_id: Option<String>,
    #[arg(long)]
    pub fix_id: Option<String>,
    /// Kaynak dosyasını değiştirme veya rollback yetkisini açıkça verir.
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Debug, clap::Args)]
pub struct FixArgs {
    #[arg(long)]
    pub project: PathBuf,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub finding_id: String,
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OptimizeMode {
    Analyze,
    Preview,
    Apply,
    Rollback,
}

impl From<OptimizeMode> for OptimizationMode {
    fn from(value: OptimizeMode) -> Self {
        match value {
            OptimizeMode::Analyze => Self::Analyze,
            OptimizeMode::Preview => Self::Preview,
            OptimizeMode::Apply => Self::Apply,
            OptimizeMode::Rollback => Self::Rollback,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum MemoryCommand {
    /// Şema, dosya, hash, link, sır ve güncellik denetimi yapar.
    Check {
        #[arg(long)]
        project: PathBuf,
    },
    /// Onarım/migration planı gösterir; yalnızca açık onayla yazar.
    Repair {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProvidersCommand {
    /// Provider sürümü, kurulum durumu ve doğrulanmış çağırma biçimini listeler.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProfilesCommand {
    /// Yerleşik başlangıç profillerini listeler.
    List,
    /// Evrensel veya dil profilini doğrular.
    Validate {
        #[arg(long)]
        profile: PathBuf,
    },
    /// Doğrulanmış profil içeriğini gösterir; dosya değiştirmez.
    Show {
        #[arg(long)]
        profile: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeployCommand {
    /// Dosya bazında create/update/unchanged/conflict dry-run planı üretir.
    Plan(DeploymentArgs),
    /// Planı açık --yes onayıyla atomik olarak uygular.
    Install(DeploymentApplyArgs),
    /// Registry ve kurulu dosya hash durumunu gösterir.
    Status(DeploymentStatusArgs),
    /// Ortak kaynaktan yeniden üretir; varsayılan dry-run'dır.
    Sync(DeploymentSyncArgs),
    /// Seçilen provider'ı güncel paket sürümüne getirir.
    Update(DeploymentApplyArgs),
    /// Yalnız kayıtlı transaction dosyalarını açık onayla geri alır.
    Rollback {
        #[arg(long)]
        transaction: String,
        #[arg(long, value_enum, default_value_t = DeploymentScope::Project)]
        scope: DeploymentScope,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, clap::Args)]
pub struct DeploymentArgs {
    #[arg(long)]
    pub profile: PathBuf,
    #[arg(short = 'P', long, value_enum)]
    pub provider: ProviderSelection,
    #[arg(long, value_enum, default_value_t = DeploymentScope::Project)]
    pub scope: DeploymentScope,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    /// Provider executable veya kurulum klasörünü açıkça seçer.
    #[arg(long, value_name = "PATH")]
    pub provider_path: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct DeploymentApplyArgs {
    #[command(flatten)]
    pub deployment: DeploymentArgs,
    #[arg(long, value_enum, default_value_t = DeploymentConflict::Abort)]
    pub conflict: DeploymentConflict,
    /// Dosya değişikliğini açıkça onaylar.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct DeploymentSyncArgs {
    #[command(flatten)]
    pub deployment: DeploymentArgs,
    /// Dry-run planını uygular; ayrıca --yes gerekir.
    #[arg(long)]
    pub apply: bool,
    /// Salt-okunur planı açıkça seçer; varsayılan davranış zaten dry-run'dır.
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = DeploymentConflict::Abort)]
    pub conflict: DeploymentConflict,
}

#[derive(Debug, clap::Args)]
pub struct DeploymentStatusArgs {
    #[arg(long, value_enum, default_value_t = DeploymentScope::Project)]
    pub scope: DeploymentScope,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DeploymentScope {
    Project,
    Global,
}

impl From<DeploymentScope> for InstallScope {
    fn from(value: DeploymentScope) -> Self {
        match value {
            DeploymentScope::Project => Self::Project,
            DeploymentScope::Global => Self::Global,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DeploymentConflict {
    Abort,
    Skip,
    BackupAndReplace,
}

impl From<DeploymentConflict> for ConflictPolicy {
    fn from(value: DeploymentConflict) -> Self {
        match value {
            DeploymentConflict::Abort => Self::Abort,
            DeploymentConflict::Skip => Self::Skip,
            DeploymentConflict::BackupAndReplace => Self::BackupAndReplace,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum MemoryProvider {
    Codex,
    Claude,
    Gemini,
    Antigravity,
    Cursor,
    Cline,
    #[value(name = "opencode")]
    OpenCode,
}

impl From<MemoryProvider> for ProviderId {
    fn from(value: MemoryProvider) -> Self {
        match value {
            MemoryProvider::Codex => Self::Codex,
            MemoryProvider::Claude => Self::Claude,
            MemoryProvider::Gemini => Self::Gemini,
            MemoryProvider::Antigravity => Self::Antigravity,
            MemoryProvider::Cursor => Self::Cursor,
            MemoryProvider::Cline => Self::Cline,
            MemoryProvider::OpenCode => Self::OpenCode,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ProviderSelection {
    Codex,
    Claude,
    Gemini,
    Antigravity,
    Cursor,
    Cline,
    #[value(name = "opencode")]
    OpenCode,
    All,
}

impl ProviderSelection {
    fn providers(self) -> Vec<Provider> {
        match self {
            Self::Codex => vec![Provider::Codex],
            Self::Claude => vec![Provider::Claude],
            Self::Gemini => vec![Provider::Gemini],
            Self::Antigravity => vec![Provider::Antigravity],
            Self::Cursor => vec![Provider::Cursor],
            Self::Cline => vec![Provider::Cline],
            Self::OpenCode => vec![Provider::OpenCode],
            Self::All => Provider::ALL.to_vec(),
        }
    }
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Profile(#[from] ProfileLoadError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Memory(#[from] MemoryError),
    #[error(transparent)]
    Workflow(#[from] WorkflowError),
    #[error(transparent)]
    QualityEngine(#[from] QualityError),
    #[error(transparent)]
    Optimization(#[from] OptimizationError),
    #[error(transparent)]
    Deploy(#[from] DeployError),
    #[error("{operation} başarısız ({path}): {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Hedef zaten var; hiçbir dosya değiştirilmedi: {0}")]
    DestinationExists(PathBuf),
    #[error("Symlink/reparse hedeflerine export yapılmaz: {0}")]
    SymlinkTarget(PathBuf),
    #[error("Proje kökü mevcut bir klasör olmalıdır: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("Hedef normal bir dosya değil: {0}")]
    InvalidTarget(PathBuf),
    #[error("Paketler aynı hedef için farklı içerik üretti: {0}")]
    ConflictingOutput(String),
    #[error("Kurulum planından sonra hedef değişti; yeniden preview alın: {0}")]
    TargetChanged(PathBuf),
    #[error("Geçersiz yedek kimliği: {0}")]
    InvalidBackupId(String),
    #[error("Yedek manifesti geçersiz: {0}")]
    InvalidBackupManifest(String),
    #[error("Güvenli olmayan çıktı yolu reddedildi: {0}")]
    UnsafeOutputPath(String),
    #[error("install için açık onay gerekir; önce preview kullanın, sonra --confirm verin")]
    ConsentRequired,
    #[error("install ilk aşamada bilinçli olarak kapalıdır; diff, yedek, atomik yazma ve rollback tamamlanınca etkinleştirilecektir")]
    InstallNotAvailable,
    #[error("Optimizasyon bulgusu bulunamadı: {0}")]
    FindingNotFound(String),
    #[error("Bulgu güvenli otomatik düzeltmeye uygun değil: {0}")]
    FindingNotAutoFixable(String),
    #[error("APPLY/ROLLBACK için açık --confirm onayı gerekir")]
    OptimizationConsentRequired,
    #[error("Optimizasyon işlem kaydı zaten var: {0}")]
    OptimizationTransactionExists(String),
    #[error("Rollback hedefi işlemden sonra kullanıcı tarafından değiştirildi: {0}")]
    OptimizationRollbackConflict(PathBuf),
    #[error("Optimizasyon artifact'i geçersiz: {0}")]
    InvalidOptimizationArtifact(String),
    #[error("Deployment değişikliği için açık --yes onayı gerekir")]
    DeploymentConsentRequired,
    #[error("Seçilen provider kurulu değil; dosya oluşturulmadı: {0}")]
    ProviderNotDetected(String),
    #[error("Geçersiz deployment argümanları: {0}")]
    InvalidDeploymentArguments(String),
}

pub fn run(cli: Cli) -> Result<String, CliError> {
    match cli.command {
        Command::Providers { command } => match command {
            None => Ok(provider_status_report()),
            Some(ProvidersCommand::List { json }) => provider_list(json),
        },
        Command::Profiles { command } => run_profiles(command),
        Command::Validate { profile } => {
            let profile = Profile::load(profile)?;
            Ok(format!(
                "Profil geçerli: {} | Dil: {} | UI: {} | DB: {}",
                profile.name,
                profile.language.display_name(),
                profile.ui.display_name(),
                profile.database.display_name()
            ))
        }
        Command::Preview { profile, provider } => {
            let profile = Profile::load(profile)?;
            let packages = render_selected(&profile, provider)?;
            Ok(format_preview(&packages))
        }
        Command::Export {
            profile,
            output,
            provider,
        } => {
            let profile = Profile::load(profile)?;
            let packages = render_selected(&profile, provider)?;
            export_packages(&output, &packages)?;
            let names = packages
                .iter()
                .map(|package| package.provider.display_name())
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!(
                "Export tamamlandı: {} ({names}). Mevcut provider dosyaları değiştirilmedi.",
                output.display()
            ))
        }
        Command::Plan {
            project,
            request,
            language,
            save,
            confirm,
        } => {
            let analyzer = ProjectAnalyzer::default();
            let analysis = analyzer.analyze(&project, language.as_deref())?;
            let project_id = memory_context_if_present(&project)?
                .map(|context| context.project_id)
                .unwrap_or_else(|| infer_project_id(&project));
            let plan = DevelopmentPlanner::create(&project_id, &request, &analysis)?;
            let report = ReportRenderer::development_plan(&plan)?;
            if !save {
                Ok(format!(
                    "{report}\nPLAN salt-okunur tamamlandı; hiçbir dosya değiştirilmedi. Kaydetmek için önce `.mt-dna` hafızasını başlatın ve --save kullanın."
                ))
            } else {
                let memory_plan = MemoryBridge::plan_development_plan(&project, &plan, &report)?;
                let memory_result = apply_or_preview_memory(
                    &memory_plan,
                    confirm,
                    "Geliştirme planı proje hafızasına kaydedildi",
                )?;
                Ok(format!("{report}\n{memory_result}"))
            }
        }
        Command::Build {
            project,
            request,
            language,
            approve,
        } => {
            let analyzer = ProjectAnalyzer::default();
            let analysis = analyzer.analyze(&project, language.as_deref())?;
            let context = memory_context_if_present(&project)?;
            let project_id = context
                .as_ref()
                .map(|value| value.project_id.clone())
                .unwrap_or_else(|| infer_project_id(&project));
            let plan = match context.and_then(|value| value.current_plan) {
                Some(plan) => plan,
                None => DevelopmentPlanner::create(&project_id, &request, &analysis)?,
            };
            let contract = DevelopmentPlanner::build_contract(plan, approve)?;
            Ok(ReportRenderer::build_contract(&contract)?)
        }
        Command::Review {
            project,
            language,
            save,
            confirm,
        } => {
            let analyzer = ProjectAnalyzer::default();
            let analysis = analyzer.analyze(&project, language.as_deref())?;
            let project_id = memory_context_if_present(&project)?
                .map(|context| context.project_id)
                .unwrap_or_else(|| infer_project_id(&project));
            let review = analyzer.review(&project_id, &analysis)?;
            let report = ReportRenderer::review_report(&review)?;
            if !save {
                Ok(format!(
                    "{report}\nREVIEW salt-okunur tamamlandı; hiçbir dosya değiştirilmedi."
                ))
            } else {
                let memory_plan = MemoryBridge::plan_review(&project, &review, &report)?;
                let memory_result = apply_or_preview_memory(
                    &memory_plan,
                    confirm,
                    "Review raporu proje hafızasına kaydedildi",
                )?;
                Ok(format!("{report}\n{memory_result}"))
            }
        }
        Command::Test(args) => run_quality(args, QualityRunKind::Test, false),
        Command::Quality(args) => run_quality(args, QualityRunKind::Quality, false),
        Command::Performance {
            args,
            save_baseline,
        } => run_quality(args, QualityRunKind::Performance, save_baseline),
        Command::Security(args) => run_quality(args, QualityRunKind::Security, false),
        Command::ReleaseCheck(args) => run_quality(args, QualityRunKind::ReleaseCheck, false),
        Command::Optimize(args) => run_optimize(args),
        Command::Fix(args) => run_optimize(OptimizeArgs {
            project: args.project,
            language: args.language,
            mode: OptimizeMode::Apply,
            finding_id: Some(args.finding_id),
            fix_id: None,
            confirm: args.confirm,
        }),
        Command::OptimizationReport { project, fix_id } => {
            read_optimization_report(&project, fix_id.as_deref())
        }
        Command::Deploy { command } => run_deployment(command),
        Command::Doctor {
            provider,
            scope,
            path,
            provider_path,
            json,
        } => run_doctor(provider, scope, &path, provider_path.as_deref(), json),
        Command::Install {
            profile,
            project,
            provider,
            confirm,
        } => {
            let profile = Profile::load(profile)?;
            let packages = render_selected(&profile, provider)?;
            let plan = plan_install(&project, &packages)?;
            let preview = format_install_plan(&plan);
            if !confirm {
                Ok(format!(
                    "{preview}\nÖnizleme tamamlandı; hiçbir dosya değiştirilmedi. Kurmak için --confirm kullanın."
                ))
            } else {
                let report = apply_install(&plan)?;
                let backup = report
                    .backup_id
                    .as_deref()
                    .map(|id| format!(" Yedek kimliği: {id}."))
                    .unwrap_or_default();
                Ok(format!(
                    "{preview}\nKurulum tamamlandı: {} değişti, {} aynı kaldı.{backup}",
                    report.changed, report.unchanged
                ))
            }
        }
        Command::Rollback {
            project,
            backup,
            confirm,
        } => {
            if !confirm {
                return Err(CliError::ConsentRequired);
            }
            let report = rollback_install(&project, &backup)?;
            Ok(format!(
                "Rollback tamamlandı: {} dosya geri yüklendi, {} yeni dosya kaldırıldı. Yedek korundu: {}",
                report.restored,
                report.removed,
                report.backup_path.display()
            ))
        }
        Command::Init {
            profile,
            project,
            project_id,
            project_type,
            platform,
            provider,
            confirm,
        } => {
            let profile = Profile::load(profile)?;
            let language = memory_language(&profile)?;
            let project_id = project_id.unwrap_or_else(|| infer_project_id(&project));
            let request = InitRequest {
                project_id,
                name: profile.name.clone(),
                language,
                project_type,
                platforms: platform,
                provider: provider.into(),
                architecture_rules: profile.standards.architecture.clone(),
                quality_rules: profile.standards.quality.clone(),
                security_rules: profile.standards.security.clone(),
                performance_rules: profile.standards.performance_benchmarks.clone(),
            };
            let plan = MemoryStore::plan_initialize(&project, request)?;
            apply_or_preview_memory(&plan, confirm, "Proje hafızası başlatıldı")
        }
        Command::Resume { project } => {
            let report = MemoryStore::resume(&project)?;
            let mut output = report.summary;
            if report.issues.is_empty() {
                output.push_str("\nHafıza denetimi: SAĞLIKLI");
            } else {
                output.push_str("\nHafıza denetimi uyarıları:\n");
                append_audit_issues(&mut output, &report.issues);
            }
            Ok(output)
        }
        Command::Update {
            project,
            provider,
            completed,
            in_progress,
            next,
            blocker,
            test_result,
            decision,
            decision_context,
            decision_option,
            decision_reason,
            affected_file,
            confirm,
        } => {
            let decision = decision.map(|value| DecisionInput {
                context: decision_context.unwrap_or_else(|| "Bağlam kaydedilmedi.".to_owned()),
                options: decision_option,
                decision: value,
                rationale: decision_reason.unwrap_or_else(|| "Gerekçe kaydedilmedi.".to_owned()),
                affected_files: affected_file,
            });
            let plan = MemoryStore::plan_update(
                &project,
                UpdateRequest {
                    provider: provider.into(),
                    completed,
                    in_progress,
                    next,
                    blockers: blocker,
                    test_results: test_result,
                    decision,
                },
            )?;
            apply_or_preview_memory(&plan, confirm, "Proje hafızası güncellendi")
        }
        Command::Handoff {
            project,
            provider,
            confirm,
        } => {
            let plan = MemoryStore::plan_handoff(&project, provider.into())?;
            apply_or_preview_memory(&plan, confirm, "Provider aktarımı hazırlandı")
        }
        Command::Memory { command } => match command {
            MemoryCommand::Check { project } => {
                let report = MemoryStore::check(&project)?;
                if report.is_healthy() {
                    Ok("Proje hafızası denetimi: SAĞLIKLI".to_owned())
                } else {
                    let mut output = String::from("Proje hafızası denetimi: SORUN BULUNDU\n");
                    append_audit_issues(&mut output, &report.issues);
                    Ok(output)
                }
            }
            MemoryCommand::Repair { project, confirm } => {
                let plan = MemoryStore::plan_repair(&project)?;
                apply_or_preview_memory(&plan, confirm, "Proje hafızası onarıldı")
            }
        },
    }
}

fn apply_or_preview_memory(
    plan: &dna_memory::MemoryPlan,
    confirm: bool,
    completed_message: &str,
) -> Result<String, CliError> {
    let preview = format_memory_plan(plan);
    if !confirm {
        return Ok(format!(
            "{preview}\nÖnizleme tamamlandı; hiçbir dosya değiştirilmedi. Uygulamak için --confirm kullanın."
        ));
    }
    let report = MemoryStore::apply(plan)?;
    let backup = report
        .backup_id
        .as_deref()
        .map(|id| format!(" Yedek kimliği: {id}."))
        .unwrap_or_default();
    Ok(format!(
        "{preview}\n{completed_message}: {} değişti, {} aynı kaldı.{backup}",
        report.changed, report.unchanged
    ))
}

fn append_audit_issues(output: &mut String, issues: &[dna_memory::AuditIssue]) {
    for issue in issues {
        let severity = match issue.severity {
            AuditSeverity::Error => "HATA",
            AuditSeverity::Warning => "UYARI",
        };
        output.push_str(&format!(
            "- [{severity}] {} | {} | {}\n",
            issue.code, issue.path, issue.message
        ));
    }
}

fn memory_language(profile: &Profile) -> Result<String, MemoryError> {
    use dna_core::ProgrammingLanguage as Language;
    let value = match profile.language {
        Language::Auto | Language::AskUser => {
            return Err(MemoryError::InvalidField {
                field: "language",
                message: "hafıza başlatmadan önce kullanıcı dili seçmelidir".to_owned(),
            })
        }
        Language::Rust => "rust",
        Language::CSharp => "csharp",
        Language::Python => "python",
        Language::Cpp => "cpp",
        Language::Go => "go",
        Language::Java => "java",
        Language::TypeScript => "typescript",
    };
    Ok(value.to_owned())
}

fn infer_project_id(project: &Path) -> String {
    let raw = project
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");
    let mut id = String::new();
    let mut last_dash = false;
    for character in raw.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            id.push(character);
            last_dash = false;
        } else if !last_dash && !id.is_empty() {
            id.push('-');
            last_dash = true;
        }
    }
    while id.ends_with('-') {
        id.pop();
    }
    id.truncate(64);
    if id.is_empty() {
        "project".to_owned()
    } else {
        id
    }
}

fn memory_context_if_present(
    project: &Path,
) -> Result<Option<dna_memory::MemoryContext>, MemoryError> {
    if project.join(dna_memory::MEMORY_DIRECTORY).exists() {
        MemoryBridge::load(project).map(Some)
    } else {
        Ok(None)
    }
}
