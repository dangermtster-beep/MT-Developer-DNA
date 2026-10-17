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
    ConflictPolicy, DeployError, DiagnosticResult, InstallScope, ProviderKey,
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

fn run_quality(
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

fn run_optimize(args: OptimizeArgs) -> Result<String, CliError> {
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

fn optimization_quality_gate(
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

fn gate_record(phase: &str, report: &dna_quality::QualityReport) -> GateResultRecord {
    GateResultRecord {
        phase: phase.to_owned(),
        gate: format!("{:?}", report.release_gate).to_ascii_uppercase(),
        report_timestamp: report.timestamp_utc.clone(),
    }
}

fn render_optimization(
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

fn save_and_render_optimization(
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

fn read_optimization_report(root: &Path, fix_id: Option<&str>) -> Result<String, CliError> {
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

fn load_quality_policy(path: Option<&Path>) -> Result<QualityPolicy, CliError> {
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

fn performance_baseline(
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

fn performance_regression_check(change_percent: i64) -> CheckResult {
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

fn quality_report_id(timestamp: &str, json: &[u8]) -> String {
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

fn run_profiles(command: ProfilesCommand) -> Result<String, CliError> {
    match command {
        ProfilesCommand::List => Ok(concat!(
            "Yerleşik profiller:\n",
            "- universal: profiles/universal.json (dil kullanıcıdan sorulur)\n",
            "- rust, csharp, python, cpp, go, java, typescript: mt-developer-dna/references/languages/<id>.json\n"
        ).to_owned()),
        ProfilesCommand::Validate { profile } => {
            let contents = fs::read_to_string(&profile).map_err(|source| CliError::Io {
                operation: "Profil okuma",
                path: profile.clone(),
                source,
            })?;
            if let Ok(value) = Profile::from_json_str(&contents) {
                return Ok(format!("Evrensel profil geçerli: {} | Dil: {}", value.name, value.language.display_name()));
            }
            let value = LanguageProfile::from_json_str(&contents)
                .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))?;
            Ok(format!("Dil profili geçerli: {} ({})", value.display_name, value.id))
        }
        ProfilesCommand::Show { profile } => {
            run_profiles(ProfilesCommand::Validate { profile: profile.clone() })?;
            fs::read_to_string(&profile).map_err(|source| CliError::Io {
                operation: "Profil okuma",
                path: profile,
                source,
            })
        }
    }
}

fn provider_list(json: bool) -> Result<String, CliError> {
    let detections = detect_all();
    if json {
        return serde_json::to_string_pretty(&detections)
            .map_err(|error| CliError::InvalidBackupManifest(error.to_string()));
    }
    let mut output = String::from("Provider keşfi (salt-okunur):\n");
    for detection in detections {
        output.push_str(&format!(
            "- {}: {} | sürüm={} | çağrı={} | slash={:?} | {}\n",
            detection.provider.as_str(),
            if detection.installed {
                "ALGILANDI"
            } else {
                "NOT_FOUND"
            },
            detection.version.as_deref().unwrap_or("UNAVAILABLE"),
            detection.invocation,
            detection.slash_command,
            detection.evidence
        ));
    }
    Ok(output)
}

fn run_deployment(command: DeployCommand) -> Result<String, CliError> {
    match command {
        DeployCommand::Plan(args) => {
            execute_deployment_plan(&args, false, false, DeploymentConflict::Abort)
        }
        DeployCommand::Install(args) | DeployCommand::Update(args) => {
            if !args.yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            execute_deployment_plan(&args.deployment, true, true, args.conflict)
        }
        DeployCommand::Sync(args) => {
            if args.apply && args.dry_run {
                return Err(CliError::InvalidDeploymentArguments(
                    "--apply ve --dry-run birlikte kullanılamaz".to_owned(),
                ));
            }
            if args.apply && !args.yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            execute_deployment_plan(&args.deployment, args.apply, args.yes, args.conflict)
        }
        DeployCommand::Status(args) => {
            let root = resolve_target_root(args.scope.into(), &args.path)?;
            let results = deployment_status(&root)?;
            format_diagnostic_output(&results, args.json)
        }
        DeployCommand::Rollback {
            transaction,
            scope,
            path,
            yes,
        } => {
            if !yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            let root = resolve_target_root(scope.into(), &path)?;
            let report = rollback_deployment(&root, &transaction)?;
            Ok(format!(
                "Deployment rollback tamamlandı: transaction={}, değişen={}, yedek korundu.",
                transaction, report.changed
            ))
        }
    }
}

fn execute_deployment_plan(
    args: &DeploymentArgs,
    apply: bool,
    approved: bool,
    conflict: DeploymentConflict,
) -> Result<String, CliError> {
    if apply && !approved {
        return Err(CliError::DeploymentConsentRequired);
    }
    if args.provider == ProviderSelection::All && args.provider_path.is_some() {
        return Err(CliError::InvalidDeploymentArguments(
            "--provider-path yalnızca tek bir --provider seçimiyle kullanılabilir.".to_owned(),
        ));
    }
    let profile = Profile::load(&args.profile)?;
    let scope: InstallScope = args.scope.into();
    let root = resolve_target_root(scope, &args.path)?;
    let detections = if args.provider == ProviderSelection::All {
        detect_all()
    } else {
        let provider = args
            .provider
            .providers()
            .into_iter()
            .next()
            .ok_or_else(|| CliError::ProviderNotDetected("all".to_owned()))?;
        vec![detect_provider_with_path(
            ProviderKey::from(provider),
            args.provider_path.as_deref(),
        )?]
    };
    let mut providers = args.provider.providers();
    if args.provider == ProviderSelection::All {
        providers.retain(|provider| {
            let key = ProviderKey::from(*provider);
            detections
                .iter()
                .any(|detection| detection.provider == key && detection.installed)
        });
    } else if let Some(provider) = providers.first() {
        let key = ProviderKey::from(*provider);
        if !detections
            .iter()
            .any(|detection| detection.provider == key && detection.installed)
        {
            return Err(CliError::ProviderNotDetected(key.as_str().to_owned()));
        }
    }
    if providers.is_empty() {
        return Err(CliError::ProviderNotDetected("all".to_owned()));
    }

    let mut output = String::new();
    for provider in providers {
        let detection = detections
            .iter()
            .find(|detection| detection.provider == ProviderKey::from(provider))
            .ok_or_else(|| {
                CliError::ProviderNotDetected(ProviderKey::from(provider).as_str().to_owned())
            })?;
        let package = render_package(&profile, provider)?;
        let plan = plan_deployment(&root, &package, scope, Some(&detection.global_skill_path))?;
        output.push_str(&format_deployment_plan(&plan));
        if apply {
            let report = apply_deployment(&plan, conflict.into())?;
            output.push_str(&format!(
                "Uygulandı: changed={}, unchanged={}, skipped={}, transaction={}\n",
                report.changed,
                report.unchanged,
                report.skipped,
                report.transaction_id.as_deref().unwrap_or("yok")
            ));
        }
    }
    if !apply {
        output.push_str(
            "Senkronizasyon/kurulum uygulanmadı. Yazmak için ilgili komutta --yes kullanın.\n",
        );
    }
    Ok(output)
}

fn run_doctor(
    selection: ProviderSelection,
    scope: DeploymentScope,
    path: &Path,
    provider_path: Option<&Path>,
    json: bool,
) -> Result<String, CliError> {
    if selection == ProviderSelection::All && provider_path.is_some() {
        return Err(CliError::InvalidDeploymentArguments(
            "--provider-path yalnızca tek bir --provider seçimiyle kullanılabilir.".to_owned(),
        ));
    }
    let root = resolve_target_root(scope.into(), path)?;
    let providers = selection
        .providers()
        .into_iter()
        .map(ProviderKey::from)
        .collect::<Vec<_>>();
    let results = deploy_doctor(&root, &providers, provider_path)?;
    format_diagnostic_output(&results, json)
}

fn format_diagnostic_output(results: &[DiagnosticResult], json: bool) -> Result<String, CliError> {
    if json {
        serde_json::to_string_pretty(results)
            .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))
    } else {
        Ok(format_diagnostics(results))
    }
}

fn render_selected(
    profile: &Profile,
    selection: ProviderSelection,
) -> Result<Vec<ProviderPackage>, RenderError> {
    selection
        .providers()
        .into_iter()
        .map(|provider| render_package(profile, provider))
        .collect()
}

fn format_preview(packages: &[ProviderPackage]) -> String {
    let mut output = String::new();
    for package in packages {
        output.push_str(&format!(
            "===== {} =====\n",
            package.provider.display_name()
        ));
        for warning in &package.warnings {
            output.push_str(&format!("UYARI: {warning}\n"));
        }
        for file in &package.files {
            output.push_str(&format!("--- {} ---\n", file.relative_path));
            output.push_str(&file.contents);
        }
    }
    output
}

pub fn export_packages(root: &Path, packages: &[ProviderPackage]) -> Result<(), CliError> {
    ensure_root_is_safe(root)?;
    create_dir_if_missing(root)?;

    for package in packages {
        let destination = root.join(package.provider.directory_name());
        if destination.exists() {
            return Err(CliError::DestinationExists(destination));
        }
    }

    for package in packages {
        export_one_package(root, package)?;
    }
    Ok(())
}

fn ensure_root_is_safe(root: &Path) -> Result<(), CliError> {
    if root.exists() {
        let metadata = fs::symlink_metadata(root).map_err(|source| CliError::Io {
            operation: "Hedef bilgisi okunamadı",
            path: root.to_path_buf(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(CliError::SymlinkTarget(root.to_path_buf()));
        }
    }
    Ok(())
}

fn create_dir_if_missing(path: &Path) -> Result<(), CliError> {
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|source| CliError::Io {
        operation: "Klasör oluşturma",
        path: path.to_path_buf(),
        source,
    })
}

fn export_one_package(root: &Path, package: &ProviderPackage) -> Result<(), CliError> {
    let destination = root.join(package.provider.directory_name());
    let staging = root.join(format!(
        ".mt-dna-tmp-{}-{}",
        package.provider.directory_name(),
        std::process::id()
    ));

    if staging.exists() {
        return Err(CliError::DestinationExists(staging));
    }
    create_dir_if_missing(&staging)?;

    let result = (|| {
        for file in &package.files {
            if !file.has_safe_relative_path() {
                return Err(CliError::UnsafeOutputPath(file.relative_path.clone()));
            }
            let target = staging.join(&file.relative_path);
            if let Some(parent) = target.parent() {
                create_dir_if_missing(parent)?;
            }
            write_new_file(&target, file.contents.as_bytes())?;
        }
        fs::rename(&staging, &destination).map_err(|source| CliError::Io {
            operation: "Atomik paket taşıma",
            path: destination.clone(),
            source,
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path).map_err(|source| CliError::Io {
        operation: "Yeni dosya oluşturma",
        path: path.to_path_buf(),
        source,
    })?;
    file.write_all(contents).map_err(|source| CliError::Io {
        operation: "Dosya yazma",
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| CliError::Io {
        operation: "Dosyayı diske eşitleme",
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_argument_definition_has_no_conflicts() {
        Cli::command().debug_assert();
    }

    #[test]
    fn v07_documented_command_shapes_parse() {
        for args in [
            vec!["mt-dna", "providers", "list"],
            vec![
                "mt-dna",
                "deploy",
                "sync",
                "--profile",
                "profile.json",
                "--provider",
                "codex",
                "--dry-run",
            ],
            vec![
                "mt-dna",
                "deploy",
                "rollback",
                "--transaction",
                "tx-1",
                "--yes",
            ],
            vec!["mt-dna", "doctor", "--provider", "all", "--json"],
        ] {
            Cli::try_parse_from(args).expect("V0.7 belgelenen komut biçimi parse edilmeli");
        }
    }

    #[test]
    fn v08_profile_and_provider_commands_parse() {
        for args in [
            vec!["mt-dna", "profiles", "list"],
            vec![
                "mt-dna",
                "profiles",
                "validate",
                "--profile",
                "profiles/universal.json",
            ],
            vec!["mt-dna", "doctor", "--provider", "antigravity"],
            vec![
                "mt-dna",
                "doctor",
                "--provider",
                "antigravity",
                "--provider-path",
                "C:/Tools/Antigravity IDE.exe",
            ],
            vec![
                "mt-dna",
                "deploy",
                "plan",
                "--profile",
                "profiles/universal.json",
                "--provider",
                "antigravity",
                "--scope",
                "global",
                "--provider-path",
                "C:/Tools/agy.exe",
            ],
            vec![
                "mt-dna",
                "preview",
                "--profile",
                "profiles/universal.json",
                "--provider",
                "opencode",
            ],
            vec![
                "mt-dna",
                "handoff",
                "--project",
                ".",
                "--provider",
                "cursor",
            ],
        ] {
            Cli::try_parse_from(args).expect("V0.8 komut biçimi parse edilmeli");
        }
    }
}
