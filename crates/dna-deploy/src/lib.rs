use dna_render::{
    deployment_relative_path, remap_skill_file, Provider, ProviderPackage, RenderedFile,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod discovery;
pub use discovery::*;
mod fsutil;
pub use fsutil::*;
#[cfg(test)]
mod tests;

pub const DEPLOY_SCHEMA_VERSION: u32 = 1;
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const STATE_DIRECTORY: &str = ".mt-dna/deploy";
const CLINE_EXTENSION_PUBLISHER: &str = "saoudrizwan";
const CLINE_EXTENSION_NAME: &str = "claude-dev";
const MAX_EXTENSION_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_EXTENSION_DIRECTORIES: usize = 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKey {
    Codex,
    Claude,
    Gemini,
    Antigravity,
    Cursor,
    Cline,
    OpenCode,
}

impl ProviderKey {
    pub const ALL: [Self; 7] = [
        Self::Codex,
        Self::Claude,
        Self::Gemini,
        Self::Antigravity,
        Self::Cursor,
        Self::Cline,
        Self::OpenCode,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
            Self::Cursor => "cursor",
            Self::Cline => "cline",
            Self::OpenCode => "opencode",
        }
    }

    pub const fn executable(self) -> Option<&'static str> {
        match self {
            Self::Codex => Some("codex"),
            Self::Claude => Some("claude"),
            Self::Gemini => Some("gemini"),
            Self::Antigravity => Some("agy"),
            Self::Cursor => Some("cursor"),
            Self::Cline => None,
            Self::OpenCode => Some("opencode"),
        }
    }

    pub const fn invocation(self) -> &'static str {
        match self {
            Self::Codex => "$mt-developer-dna",
            Self::Claude
            | Self::Gemini
            | Self::Antigravity
            | Self::Cursor
            | Self::Cline
            | Self::OpenCode => "/mt-developer-dna",
        }
    }

    pub const fn slash_support(self) -> CapabilityState {
        match self {
            Self::Codex => CapabilityState::Unsupported,
            Self::Claude | Self::Gemini | Self::Cursor | Self::Cline | Self::OpenCode => {
                CapabilityState::Supported
            }
            Self::Antigravity => CapabilityState::Supported,
        }
    }

    pub const fn provider(self) -> Provider {
        match self {
            Self::Codex => Provider::Codex,
            Self::Claude => Provider::Claude,
            Self::Gemini => Provider::Gemini,
            Self::Antigravity => Provider::Antigravity,
            Self::Cursor => Provider::Cursor,
            Self::Cline => Provider::Cline,
            Self::OpenCode => Provider::OpenCode,
        }
    }
}

impl From<Provider> for ProviderKey {
    fn from(value: Provider) -> Self {
        match value {
            Provider::Codex => Self::Codex,
            Provider::Claude => Self::Claude,
            Provider::Gemini => Self::Gemini,
            Provider::Antigravity => Self::Antigravity,
            Provider::Cursor => Self::Cursor,
            Provider::Cline => Self::Cline,
            Provider::OpenCode => Self::OpenCode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityState {
    Supported,
    Unsupported,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallationKind {
    Cli,
    Desktop,
    VsCodeExtension,
    NpmShim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInstallation {
    pub kind: InstallationKind,
    pub path: PathBuf,
    pub version: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDetection {
    pub provider: ProviderKey,
    pub installed: bool,
    pub version: Option<String>,
    pub invocation: String,
    pub slash_command: CapabilityState,
    pub skill: CapabilityState,
    pub rules: CapabilityState,
    pub project_install: CapabilityState,
    pub global_install: CapabilityState,
    pub installations: Vec<ProviderInstallation>,
    pub global_skill_path: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallScope {
    Project,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    Abort,
    Skip,
    BackupAndReplace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Create,
    Update,
    Unchanged,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub package: String,
    pub version: String,
    pub provider: ProviderKey,
    pub scope: InstallScope,
    pub adapter_version: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentChange {
    pub path: String,
    pub kind: ChangeKind,
    pub current_sha256: Option<String>,
    pub desired_sha256: String,
    desired: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub target_root: PathBuf,
    pub provider: ProviderKey,
    pub scope: InstallScope,
    pub manifest: PackageManifest,
    pub changes: Vec<DeploymentChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub provider: ProviderKey,
    pub scope: InstallScope,
    pub package_version: String,
    pub adapter_version: String,
    pub manifest_sha256: String,
    pub transaction_id: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InstallRegistry {
    schema_version: u32,
    entries: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TransactionFile {
    path: String,
    had_original: bool,
    before_sha256: Option<String>,
    after_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TransactionRecord {
    schema_version: u32,
    transaction_id: String,
    provider: ProviderKey,
    scope: InstallScope,
    status: String,
    previous_registry: Option<RegistryEntry>,
    files: Vec<TransactionFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentReport {
    pub transaction_id: Option<String>,
    pub changed: usize,
    pub unchanged: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticStatus {
    Pass,
    Warn,
    Fail,
    NotRun,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub provider: Option<ProviderKey>,
    pub check: String,
    pub status: DiagnosticStatus,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("Geçersiz deployment kökü: {0}")]
    InvalidRoot(PathBuf),
    #[error("Provider kurulum yolu doğrulanamadı veya beklenen executable değil: {0}")]
    InvalidProviderPath(PathBuf),
    #[error("Güvenli olmayan deployment yolu: {0}")]
    UnsafePath(String),
    #[error("Symlink/junction/reparse hedefi reddedildi: {0}")]
    LinkTarget(PathBuf),
    #[error("Provider paketi eşleşmiyor")]
    ProviderMismatch,
    #[error("Kurulum çatışmaları çözülmeden APPLY yapılamaz: {0}")]
    Conflict(String),
    #[error("Deployment işlemi zaten çalışıyor: {0}")]
    Locked(PathBuf),
    #[error("Deployment manifesti veya registry geçersiz: {0}")]
    InvalidManifest(String),
    #[error("Rollback hedefi kullanıcı tarafından değiştirilmiş: {0}")]
    RollbackConflict(PathBuf),
    #[error("Yedeklenecek provider dosyasında olası gizli bilgi bulundu: {0}")]
    SecretBackupRefused(PathBuf),
    #[error("Global kullanıcı dizini çözümlenemedi")]
    HomeUnavailable,
    #[error("{operation} başarısız ({path}): {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn resolve_target_root(scope: InstallScope, project: &Path) -> Result<PathBuf, DeployError> {
    let root = match scope {
        InstallScope::Project => project.to_path_buf(),
        InstallScope::Global => env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from)
            .ok_or(DeployError::HomeUnavailable)?,
    };
    validate_root(&root)
}

pub fn plan_install(
    target_root: &Path,
    package: &ProviderPackage,
    scope: InstallScope,
) -> Result<InstallPlan, DeployError> {
    plan_install_with_global_skill_path(target_root, package, scope, None)
}

pub fn plan_install_with_global_skill_path(
    target_root: &Path,
    package: &ProviderPackage,
    scope: InstallScope,
    global_skill_path: Option<&str>,
) -> Result<InstallPlan, DeployError> {
    let target_root = validate_root(target_root)?;
    let provider = ProviderKey::from(package.provider);
    let registry = read_registry(&target_root)?;
    let registered = registry
        .entries
        .iter()
        .find(|entry| entry.provider == provider && entry.scope == scope);
    let registered_files = registered
        .map(|entry| {
            entry
                .files
                .iter()
                .map(|file| (file.path.as_str(), file.sha256.as_str()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let files = deployable_files(package, scope, global_skill_path);
    let mut changes = Vec::with_capacity(files.len());
    for (relative_path, file) in files {
        if !file.has_safe_relative_path() || !safe_relative_string(&relative_path) {
            return Err(DeployError::UnsafePath(relative_path));
        }
        let target = checked_target(&target_root, &relative_path)?;
        let desired = file.contents.as_bytes().to_vec();
        let desired_sha256 = sha256_hex(&desired);
        let current = if target.exists() {
            Some(read_regular_file(&target)?)
        } else {
            None
        };
        let current_sha256 = current.as_deref().map(sha256_hex);
        let kind = match current_sha256.as_deref() {
            None => ChangeKind::Create,
            Some(hash) if hash == desired_sha256 => ChangeKind::Unchanged,
            Some(hash) if registered_files.get(relative_path.as_str()) == Some(&hash) => {
                ChangeKind::Update
            }
            Some(_) => ChangeKind::Conflict,
        };
        changes.push(DeploymentChange {
            path: relative_path,
            kind,
            current_sha256,
            desired_sha256,
            desired,
        });
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = PackageManifest {
        schema_version: DEPLOY_SCHEMA_VERSION,
        package: "mt-developer-dna".to_owned(),
        version: PACKAGE_VERSION.to_owned(),
        provider,
        scope,
        adapter_version: ADAPTER_VERSION.to_owned(),
        files: changes
            .iter()
            .map(|change| ManifestFile {
                path: change.path.clone(),
                sha256: change.desired_sha256.clone(),
            })
            .collect(),
    };
    Ok(InstallPlan {
        target_root,
        provider,
        scope,
        manifest,
        changes,
    })
}

fn deployable_files<'a>(
    package: &'a ProviderPackage,
    scope: InstallScope,
    global_skill_path: Option<&str>,
) -> Vec<(String, &'a RenderedFile)> {
    let descriptor = dna_render::provider_descriptor(package.provider);
    package
        .files
        .iter()
        .filter_map(|file| {
            let override_path = global_skill_path.filter(|_| scope == InstallScope::Global);
            let path = if let Some(target_skill_path) = override_path {
                remap_skill_file(
                    &file.relative_path,
                    descriptor.project_skill_path,
                    target_skill_path,
                )
                .or_else(|| deployment_relative_path(package.provider, &file.relative_path, true))
            } else {
                deployment_relative_path(
                    package.provider,
                    &file.relative_path,
                    scope == InstallScope::Global,
                )
            };
            path.map(|path| (path, file))
        })
        .collect()
}

fn safe_relative_string(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub fn format_plan(plan: &InstallPlan) -> String {
    let mut output = format!(
        "Deployment planı: provider={} scope={:?} version={}\n",
        plan.provider.as_str(),
        plan.scope,
        plan.manifest.version
    );
    for change in &plan.changes {
        output.push_str(&format!(
            "- {:?}: {} [önce={} -> sonra={}]\n",
            change.kind,
            change.path,
            change.current_sha256.as_deref().unwrap_or("YOK"),
            change.desired_sha256
        ));
    }
    output
}

pub fn apply_install(
    plan: &InstallPlan,
    policy: ConflictPolicy,
) -> Result<DeploymentReport, DeployError> {
    let conflicts = plan
        .changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Conflict)
        .map(|change| change.path.as_str())
        .collect::<Vec<_>>();
    if !conflicts.is_empty() && policy == ConflictPolicy::Abort {
        return Err(DeployError::Conflict(conflicts.join(", ")));
    }
    verify_plan(plan)?;
    let _lock = DeploymentLock::acquire(&plan.target_root)?;
    let selected = plan
        .changes
        .iter()
        .filter(|change| match change.kind {
            ChangeKind::Create | ChangeKind::Update => true,
            ChangeKind::Conflict => policy == ConflictPolicy::BackupAndReplace,
            ChangeKind::Unchanged => false,
        })
        .collect::<Vec<_>>();
    let skipped = plan
        .changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Conflict && policy == ConflictPolicy::Skip)
        .count();
    if selected.is_empty() {
        return Ok(DeploymentReport {
            transaction_id: None,
            changed: 0,
            unchanged: plan.changes.len() - skipped,
            skipped,
        });
    }

    let transaction_id = new_transaction_id();
    let registry_before = read_registry(&plan.target_root)?;
    let previous_registry = registry_before
        .entries
        .iter()
        .find(|entry| entry.provider == plan.provider && entry.scope == plan.scope)
        .cloned();
    for change in &selected {
        let target = checked_target(&plan.target_root, &change.path)?;
        if target.exists() && contains_secret(&read_regular_file(&target)?) {
            return Err(DeployError::SecretBackupRefused(target));
        }
    }
    let transaction_root = state_path(&plan.target_root, &format!("backups/{transaction_id}"))?;
    create_directory_tree(&plan.target_root, &transaction_root)?;
    let mut records = Vec::new();
    for change in &selected {
        let target = checked_target(&plan.target_root, &change.path)?;
        let old = if target.exists() {
            Some(read_regular_file(&target)?)
        } else {
            None
        };
        if let Some(contents) = &old {
            let backup = transaction_root.join("files").join(&change.path);
            write_new_synced(&plan.target_root, &backup, contents)?;
        }
        records.push(TransactionFile {
            path: change.path.clone(),
            had_original: old.is_some(),
            before_sha256: old.as_deref().map(sha256_hex),
            after_sha256: change.desired_sha256.clone(),
        });
    }
    let mut transaction = TransactionRecord {
        schema_version: DEPLOY_SCHEMA_VERSION,
        transaction_id: transaction_id.clone(),
        provider: plan.provider,
        scope: plan.scope,
        status: "PREPARED".to_owned(),
        previous_registry,
        files: records,
    };
    write_json_new(
        &plan.target_root,
        &transaction_root.join("transaction.json"),
        &transaction,
    )?;

    let mut applied = Vec::new();
    for change in &selected {
        let target = checked_target(&plan.target_root, &change.path)?;
        if let Err(error) = atomic_write(&plan.target_root, &target, &change.desired) {
            restore_applied(&plan.target_root, &transaction_root, &transaction, &applied);
            return Err(error);
        }
        applied.push(change.path.clone());
    }

    let installed_files = plan
        .changes
        .iter()
        .filter(|change| {
            change.kind != ChangeKind::Conflict || policy == ConflictPolicy::BackupAndReplace
        })
        .map(|change| ManifestFile {
            path: change.path.clone(),
            sha256: change.desired_sha256.clone(),
        })
        .collect::<Vec<_>>();
    let installed_manifest = PackageManifest {
        files: installed_files.clone(),
        ..plan.manifest.clone()
    };
    let manifest_path = state_path(
        &plan.target_root,
        &format!("manifests/{}-{:?}.json", plan.provider.as_str(), plan.scope),
    )?;
    atomic_write_json(&plan.target_root, &manifest_path, &installed_manifest)?;
    let manifest_bytes = serde_json::to_vec(&installed_manifest)
        .map_err(|e| DeployError::InvalidManifest(e.to_string()))?;
    let mut registry = read_registry(&plan.target_root)?;
    registry
        .entries
        .retain(|entry| !(entry.provider == plan.provider && entry.scope == plan.scope));
    registry.entries.push(RegistryEntry {
        provider: plan.provider,
        scope: plan.scope,
        package_version: PACKAGE_VERSION.to_owned(),
        adapter_version: ADAPTER_VERSION.to_owned(),
        manifest_sha256: sha256_hex(&manifest_bytes),
        transaction_id: transaction_id.clone(),
        files: installed_files,
    });
    registry
        .entries
        .sort_by_key(|entry| (entry.provider.as_str(), format!("{:?}", entry.scope)));
    write_registry(&plan.target_root, &registry)?;

    transaction.status = "APPLIED".to_owned();
    atomic_write_json(
        &plan.target_root,
        &transaction_root.join("transaction.json"),
        &transaction,
    )?;
    Ok(DeploymentReport {
        transaction_id: Some(transaction_id),
        changed: selected.len(),
        unchanged: plan
            .changes
            .iter()
            .filter(|change| change.kind == ChangeKind::Unchanged)
            .count(),
        skipped,
    })
}

pub fn rollback(target_root: &Path, transaction_id: &str) -> Result<DeploymentReport, DeployError> {
    if !safe_id(transaction_id) {
        return Err(DeployError::InvalidManifest(
            "Geçersiz transaction kimliği".to_owned(),
        ));
    }
    let target_root = validate_root(target_root)?;
    let _lock = DeploymentLock::acquire(&target_root)?;
    let transaction_root = state_path(&target_root, &format!("backups/{transaction_id}"))?;
    let path = transaction_root.join("transaction.json");
    let mut transaction: TransactionRecord = read_json(&path)?;
    if transaction.schema_version != DEPLOY_SCHEMA_VERSION
        || transaction.transaction_id != transaction_id
        || transaction.status != "APPLIED"
    {
        return Err(DeployError::InvalidManifest(
            "Rollback transaction durumu geçersiz".to_owned(),
        ));
    }
    for file in &transaction.files {
        let target = checked_target(&target_root, &file.path)?;
        let current = read_regular_file(&target)?;
        if sha256_hex(&current) != file.after_sha256 {
            return Err(DeployError::RollbackConflict(target));
        }
        if file.had_original {
            let backup = transaction_root.join("files").join(&file.path);
            let contents = read_regular_file(&backup)?;
            if file.before_sha256.as_deref() != Some(sha256_hex(&contents).as_str()) {
                return Err(DeployError::InvalidManifest(
                    "Yedek hash'i eşleşmiyor".to_owned(),
                ));
            }
        }
    }
    let mut restored = 0;
    let mut removed = 0;
    for file in &transaction.files {
        let target = checked_target(&target_root, &file.path)?;
        if file.had_original {
            let contents = read_regular_file(&transaction_root.join("files").join(&file.path))?;
            atomic_write(&target_root, &target, &contents)?;
            restored += 1;
        } else {
            fs::remove_file(&target)
                .map_err(|source| io("Rollback dosya silme", &target, source))?;
            removed += 1;
        }
    }
    let mut registry = read_registry(&target_root)?;
    registry.entries.retain(|entry| {
        !(entry.provider == transaction.provider && entry.scope == transaction.scope)
    });
    if let Some(previous) = transaction.previous_registry.clone() {
        registry.entries.push(previous);
    }
    write_registry(&target_root, &registry)?;
    transaction.status = "ROLLED_BACK".to_owned();
    atomic_write_json(&target_root, &path, &transaction)?;
    Ok(DeploymentReport {
        transaction_id: Some(transaction_id.to_owned()),
        changed: restored + removed,
        unchanged: 0,
        skipped: 0,
    })
}

pub fn status(target_root: &Path) -> Result<Vec<DiagnosticResult>, DeployError> {
    let target_root = validate_root(target_root)?;
    let registry = read_registry(&target_root)?;
    if registry.entries.is_empty() {
        return Ok(vec![DiagnosticResult {
            provider: None,
            check: "registry".to_owned(),
            status: DiagnosticStatus::Warn,
            detail: "Kurulum kaydı bulunamadı.".to_owned(),
        }]);
    }
    let mut results = Vec::new();
    for entry in registry.entries {
        for file in entry.files {
            let target = checked_target(&target_root, &file.path)?;
            let (state, detail) = if !target.exists() {
                (DiagnosticStatus::Fail, format!("Eksik: {}", file.path))
            } else if sha256_hex(&read_regular_file(&target)?) == file.sha256 {
                (
                    DiagnosticStatus::Pass,
                    format!("Hash eşleşiyor: {}", file.path),
                )
            } else {
                (
                    DiagnosticStatus::Warn,
                    format!("user-modified: {}", file.path),
                )
            };
            results.push(DiagnosticResult {
                provider: Some(entry.provider),
                check: "installed-file".to_owned(),
                status: state,
                detail,
            });
        }
    }
    Ok(results)
}

pub fn doctor(
    target_root: &Path,
    providers: &[ProviderKey],
) -> Result<Vec<DiagnosticResult>, DeployError> {
    doctor_with_installation_path(target_root, providers, None)
}

pub fn doctor_with_installation_path(
    target_root: &Path,
    providers: &[ProviderKey],
    selected_path: Option<&Path>,
) -> Result<Vec<DiagnosticResult>, DeployError> {
    let target_root = validate_root(target_root)?;
    if selected_path.is_some() && providers.len() != 1 {
        return Err(DeployError::InvalidProviderPath(
            selected_path.unwrap_or_else(|| Path::new("")).to_path_buf(),
        ));
    }
    let mut results = Vec::new();
    for provider in providers {
        let detection = detect_provider_with_path(*provider, selected_path)?;
        results.push(DiagnosticResult {
            provider: Some(*provider),
            check: "provider-executable".to_owned(),
            status: if detection.installed {
                DiagnosticStatus::Pass
            } else {
                DiagnosticStatus::Unsupported
            },
            detail: detection.evidence,
        });
        results.push(DiagnosticResult {
            provider: Some(*provider),
            check: "live-invocation".to_owned(),
            status: DiagnosticStatus::NotRun,
            detail: format!(
                "{} canlı interaktif oturumda çalıştırılmadı.",
                provider.invocation()
            ),
        });
    }
    results.extend(status(&target_root)?);
    let memory = target_root.join(".mt-dna/project.json");
    results.push(DiagnosticResult {
        provider: None,
        check: "project-memory".to_owned(),
        status: if memory.is_file() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Warn
        },
        detail: if memory.is_file() {
            "V0.3 proje hafızası bulundu; ayrıntılı şema kontrolü `memory check` ile yapılır."
                .to_owned()
        } else {
            "`.mt-dna/project.json` bulunamadı.".to_owned()
        },
    });
    Ok(results)
}

pub fn format_diagnostics(results: &[DiagnosticResult]) -> String {
    let mut output = String::from("MT Developer DNA Doctor\n");
    for result in results {
        output.push_str(&format!(
            "- {:?} | provider={} | {} | {}\n",
            result.status,
            result.provider.map(ProviderKey::as_str).unwrap_or("core"),
            result.check,
            result.detail
        ));
    }
    output
}
