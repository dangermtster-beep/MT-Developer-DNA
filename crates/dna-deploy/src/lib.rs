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

pub fn detect_provider(provider: ProviderKey) -> ProviderDetection {
    detect_provider_with_path(provider, None).unwrap_or_else(|error| {
        unavailable_detection(provider, format!("Provider discovery başarısız: {error}"))
    })
}

pub fn detect_provider_with_path(
    provider: ProviderKey,
    selected_path: Option<&Path>,
) -> Result<ProviderDetection, DeployError> {
    let installations = if let Some(path) = selected_path {
        validate_selected_installations(provider, path)?
    } else if provider == ProviderKey::Antigravity {
        discover_antigravity()
    } else if provider == ProviderKey::Cline {
        discover_cline()
    } else if supports_npm_runners(provider) {
        discover_npm_runners(provider)
    } else {
        discover_command_installation(provider)
    };
    let installed = !installations.is_empty();
    let version = installations
        .iter()
        .filter_map(|installation| installation.version.as_ref())
        .max_by(|left, right| compare_version_text(left, right))
        .cloned();
    let descriptor = dna_render::provider_descriptor(provider.provider());
    let capability = |state| match state {
        dna_render::CapabilityState::Supported => CapabilityState::Supported,
        dna_render::CapabilityState::Unsupported => CapabilityState::Unsupported,
        dna_render::CapabilityState::Unverified | dna_render::CapabilityState::Deprecated => {
            CapabilityState::Unverified
        }
    };
    let global_skill_path = if provider == ProviderKey::Antigravity
        && !installations
            .iter()
            .any(|installation| installation.kind == InstallationKind::Desktop)
        && installations
            .iter()
            .any(|installation| installation.kind == InstallationKind::Cli)
    {
        ".gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md".to_owned()
    } else {
        descriptor.global_skill_path.to_owned()
    };
    let evidence = if installed {
        let mut evidence = installations
            .iter()
            .map(|installation| {
                format!(
                    "{:?}: {} ({})",
                    installation.kind,
                    installation.path.display(),
                    installation.source
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        if provider == ProviderKey::Cline {
            evidence.push_str(
                "; extension paketi doğrulandı; etkin profil/enable durumu canlı VS Code oturumunda doğrulanmadı",
            );
        }
        evidence
    } else if provider == ProviderKey::Cline {
        "`saoudrizwan.claude-dev` VS Code extension paketi standart, Insiders veya özel dizinlerde bulunamadı."
            .to_owned()
    } else if provider.executable().is_none() {
        "Bu provider IDE eklentisi olarak çalışır; standalone executable probe uygulanamaz."
            .to_owned()
    } else {
        format!(
            "`{}` executable ve doğrulanmış kurulum yolu bulunamadı.",
            provider.executable().unwrap_or("bilinmiyor")
        )
    };
    Ok(ProviderDetection {
        provider,
        installed,
        version,
        invocation: provider.invocation().to_owned(),
        slash_command: if installed {
            provider.slash_support()
        } else {
            CapabilityState::Unverified
        },
        skill: capability(descriptor.skill),
        rules: capability(descriptor.rules),
        project_install: capability(descriptor.project_install),
        global_install: capability(descriptor.global_install),
        installations,
        global_skill_path,
        evidence,
    })
}

pub fn detect_all() -> Vec<ProviderDetection> {
    ProviderKey::ALL.into_iter().map(detect_provider).collect()
}

fn command_version(command: &str) -> Option<String> {
    command_version_path(Path::new(command))
}

fn command_version_path(command: &Path) -> Option<String> {
    let extension = command
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let output = match extension.as_str() {
        "ps1" => Command::new("powershell.exe")
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
            .arg(command)
            .arg("--version")
            .output()
            .ok()?,
        "cmd" => {
            let command_text = safe_cmd_version_invocation(command)?;
            cmd_script_version_output(&command_text)?
        }
        _ => Command::new(command).arg("--version").output().ok()?,
    };
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .chain(String::from_utf8_lossy(&output.stderr).lines())
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("WARNING:"))
        .map(ToOwned::to_owned)
}

#[cfg(windows)]
fn cmd_script_version_output(command_text: &str) -> Option<std::process::Output> {
    use std::os::windows::process::CommandExt as _;
    Command::new("cmd.exe")
        .args(["/D", "/S", "/C"])
        .raw_arg(command_text)
        .output()
        .ok()
}

#[cfg(not(windows))]
fn cmd_script_version_output(command_text: &str) -> Option<std::process::Output> {
    Command::new("cmd.exe")
        .args(["/D", "/S", "/C", command_text])
        .output()
        .ok()
}

fn safe_cmd_version_invocation(command: &Path) -> Option<String> {
    let path = command.to_str()?;
    if path.chars().any(|character| {
        matches!(
            character,
            '"' | '&' | '|' | '<' | '>' | '^' | '%' | '\r' | '\n'
        )
    }) {
        return None;
    }
    Some(format!("\"\"{path}\" --version\""))
}

fn unavailable_detection(provider: ProviderKey, evidence: String) -> ProviderDetection {
    let descriptor = dna_render::provider_descriptor(provider.provider());
    ProviderDetection {
        provider,
        installed: false,
        version: None,
        invocation: provider.invocation().to_owned(),
        slash_command: CapabilityState::Unverified,
        skill: CapabilityState::Unverified,
        rules: CapabilityState::Unverified,
        project_install: CapabilityState::Unverified,
        global_install: CapabilityState::Unverified,
        installations: Vec::new(),
        global_skill_path: descriptor.global_skill_path.to_owned(),
        evidence,
    }
}

fn discover_command_installation(provider: ProviderKey) -> Vec<ProviderInstallation> {
    let Some(command) = provider.executable() else {
        return Vec::new();
    };
    let Some(version) = command_version(command) else {
        return Vec::new();
    };
    vec![ProviderInstallation {
        kind: InstallationKind::Cli,
        path: command_path(command).unwrap_or_else(|| PathBuf::from(command)),
        version: Some(version),
        source: "PATH executable --version".to_owned(),
    }]
}

fn discover_antigravity() -> Vec<ProviderInstallation> {
    let mut installations = discover_command_installation(ProviderKey::Antigravity);
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        installations.extend(discover_antigravity_in_local_app_data(Path::new(
            &local_app_data,
        )));
    }
    if let Some(home) = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")) {
        installations.extend(discover_antigravity_under(Path::new(&home)));
    }
    deduplicate_installations(&mut installations);
    installations
}

fn discover_antigravity_under(home: &Path) -> Vec<ProviderInstallation> {
    discover_antigravity_in_local_app_data(&home.join("AppData/Local"))
}

fn discover_antigravity_in_local_app_data(local_app_data: &Path) -> Vec<ProviderInstallation> {
    let candidates = [
        (
            InstallationKind::Cli,
            local_app_data.join("agy/bin/agy.exe"),
            "Antigravity CLI Windows kullanıcı kurulum dizini",
        ),
        (
            InstallationKind::Desktop,
            local_app_data.join("Programs/Antigravity IDE/Antigravity IDE.exe"),
            "Antigravity IDE Windows kullanıcı kurulum dizini",
        ),
    ];
    candidates
        .into_iter()
        .filter_map(|(kind, path, source)| {
            regular_non_link_file(&path).then(|| ProviderInstallation {
                kind,
                version: (kind == InstallationKind::Cli)
                    .then(|| command_version_path(&path))
                    .flatten(),
                path,
                source: source.to_owned(),
            })
        })
        .collect()
}

fn discover_cline() -> Vec<ProviderInstallation> {
    let mut roots = Vec::<(PathBuf, &'static str)>::new();
    if let Some(home) = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")) {
        let home = PathBuf::from(home);
        roots.push((
            home.join(".vscode/extensions"),
            "VS Code kullanıcı extension dizini (tüm profillerin paket deposu)",
        ));
        roots.push((
            home.join(".vscode-insiders/extensions"),
            "VS Code Insiders kullanıcı extension dizini (tüm profillerin paket deposu)",
        ));
    }
    if let Some(custom) = env::var_os("VSCODE_EXTENSIONS") {
        roots.push((
            PathBuf::from(custom),
            "VSCODE_EXTENSIONS özel extension dizini",
        ));
    }

    let mut installations = roots
        .into_iter()
        .flat_map(|(root, source)| discover_cline_in_extension_root(&root, source))
        .collect::<Vec<_>>();
    deduplicate_installations(&mut installations);
    installations
}

/// npm ile kurulabilen ve Windows'ta `.cmd`/`.ps1` shim'i üzerinden çalışan provider'lar.
/// `Command::new("<ad>")` bu shim'leri çözmediği için PATH dizinleri doğrudan taranır.
fn supports_npm_runners(provider: ProviderKey) -> bool {
    matches!(provider, ProviderKey::Codex | ProviderKey::OpenCode)
}

fn npm_runner_names(provider: ProviderKey) -> [String; 3] {
    let command = provider.executable().unwrap_or_default();
    ["exe", "cmd", "ps1"].map(|extension| format!("{command}.{extension}"))
}

fn discover_npm_runners(provider: ProviderKey) -> Vec<ProviderInstallation> {
    let Some(path) = env::var_os("PATH") else {
        return Vec::new();
    };
    discover_npm_runners_in_path(provider, env::split_paths(&path))
}

fn discover_npm_runners_in_path(
    provider: ProviderKey,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Vec<ProviderInstallation> {
    let display_name = dna_render::provider_descriptor(provider.provider()).display_name;
    let mut installations = Vec::new();
    for directory in directories {
        for file_name in npm_runner_names(provider) {
            let candidate = directory.join(&file_name);
            if !regular_non_link_file(&candidate) {
                continue;
            }
            let is_executable = file_name.ends_with(".exe");
            installations.push(ProviderInstallation {
                kind: if is_executable {
                    InstallationKind::Cli
                } else {
                    InstallationKind::NpmShim
                },
                version: command_version_path(&candidate),
                path: candidate,
                source: if is_executable {
                    format!("PATH üzerindeki {display_name} Windows executable")
                } else if file_name.ends_with(".cmd") {
                    "PATH üzerindeki npm CMD shim; cmd.exe sürüm probe'u".to_owned()
                } else {
                    "PATH üzerindeki npm PowerShell shim; execution policy korunarak sürüm probe'u"
                        .to_owned()
                },
            });
        }
    }
    deduplicate_installations(&mut installations);
    installations
}

fn discover_cline_in_extension_root(root: &Path, source: &str) -> Vec<ProviderInstallation> {
    if !regular_non_link_dir(root) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut installations = entries
        .take(MAX_EXTENSION_DIRECTORIES)
        .filter_map(Result::ok)
        .filter_map(|entry| cline_extension_installation(&entry.path(), source))
        .collect::<Vec<_>>();
    installations.sort_by(|left, right| left.path.cmp(&right.path));
    installations
}

fn cline_extension_installation(path: &Path, source: &str) -> Option<ProviderInstallation> {
    if !regular_non_link_dir(path) {
        return None;
    }
    let manifest_path = path.join("package.json");
    let metadata = fs::symlink_metadata(&manifest_path).ok()?;
    if !metadata.is_file() || is_link(&metadata) || metadata.len() > MAX_EXTENSION_MANIFEST_BYTES {
        return None;
    }
    let manifest = fs::read(&manifest_path).ok()?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).ok()?;
    let publisher = manifest.get("publisher")?.as_str()?;
    let name = manifest.get("name")?.as_str()?;
    if publisher != CLINE_EXTENSION_PUBLISHER || name != CLINE_EXTENSION_NAME {
        return None;
    }
    let version = manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    Some(ProviderInstallation {
        kind: InstallationKind::VsCodeExtension,
        path: path.to_path_buf(),
        version,
        source: source.to_owned(),
    })
}

fn validate_selected_installations(
    provider: ProviderKey,
    selected: &Path,
) -> Result<Vec<ProviderInstallation>, DeployError> {
    if provider == ProviderKey::Cline {
        if let Some(installation) = cline_extension_installation(
            selected,
            "kullanıcı tarafından seçilen ve manifest kimliği doğrulanan Cline extension dizini",
        ) {
            return Ok(vec![installation]);
        }
        let installations = discover_cline_in_extension_root(
            selected,
            "kullanıcı tarafından seçilen ve manifest kimliği doğrulanan extension kökü",
        );
        return (!installations.is_empty())
            .then_some(installations)
            .ok_or_else(|| DeployError::InvalidProviderPath(selected.to_path_buf()));
    }
    if supports_npm_runners(provider) {
        let display_name = dna_render::provider_descriptor(provider.provider()).display_name;
        let installations = if selected.is_dir() {
            discover_npm_runners_in_path(provider, [selected.to_path_buf()])
        } else if regular_non_link_file(selected) && is_npm_runner(provider, selected) {
            vec![ProviderInstallation {
                kind: if selected
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                {
                    InstallationKind::Cli
                } else {
                    InstallationKind::NpmShim
                },
                version: command_version_path(selected),
                path: selected.to_path_buf(),
                source: format!(
                    "kullanıcı tarafından seçilen ve adı doğrulanan {display_name} çalıştırıcısı"
                ),
            }]
        } else {
            Vec::new()
        };
        return (!installations.is_empty())
            .then_some(installations)
            .ok_or_else(|| DeployError::InvalidProviderPath(selected.to_path_buf()));
    }
    let candidate = if selected.is_dir() && provider == ProviderKey::Antigravity {
        [
            selected.join("Antigravity IDE.exe"),
            selected.join("agy.exe"),
            selected.join("bin/agy.exe"),
        ]
        .into_iter()
        .find(|path| regular_non_link_file(path))
        .ok_or_else(|| DeployError::InvalidProviderPath(selected.to_path_buf()))?
    } else {
        selected.to_path_buf()
    };
    if !regular_non_link_file(&candidate) {
        return Err(DeployError::InvalidProviderPath(candidate));
    }
    let file_name = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = if provider == ProviderKey::Antigravity {
        match file_name.as_str() {
            "agy" | "agy.exe" => InstallationKind::Cli,
            "antigravity ide" | "antigravity ide.exe" => InstallationKind::Desktop,
            _ => return Err(DeployError::InvalidProviderPath(candidate)),
        }
    } else {
        let expected = provider.executable().unwrap_or_default();
        if file_name != expected && file_name != format!("{expected}.exe") {
            return Err(DeployError::InvalidProviderPath(candidate));
        }
        InstallationKind::Cli
    };
    let version = (kind == InstallationKind::Cli)
        .then(|| command_version_path(&candidate))
        .flatten();
    Ok(vec![ProviderInstallation {
        kind,
        path: candidate,
        version,
        source: "kullanıcı tarafından seçilen ve doğrulanan kurulum yolu".to_owned(),
    }])
}

fn is_npm_runner(provider: ProviderKey, path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            npm_runner_names(provider)
                .iter()
                .any(|runner| name.eq_ignore_ascii_case(runner))
        })
}

fn regular_non_link_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_file() && !is_link(&metadata))
        .unwrap_or(false)
}

fn regular_non_link_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_link(&metadata))
        .unwrap_or(false)
}

fn compare_version_text(left: &str, right: &str) -> std::cmp::Ordering {
    let numeric = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    numeric(left)
        .cmp(&numeric(right))
        .then_with(|| left.cmp(right))
}

fn deduplicate_installations(installations: &mut Vec<ProviderInstallation>) {
    installations.sort_by(|left, right| left.path.cmp(&right.path));
    installations.dedup_by(|left, right| left.path == right.path);
}

fn command_path(command: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let output = Command::new("where.exe").arg(command).output().ok()?;
    #[cfg(not(windows))]
    let output = Command::new("which").arg(command).output().ok()?;
    output.status.success().then_some(()).and_then(|_| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(PathBuf::from)
    })
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

fn read_registry(root: &Path) -> Result<InstallRegistry, DeployError> {
    let path = state_path(root, "registry.json")?;
    if !path.exists() {
        return Ok(InstallRegistry {
            schema_version: DEPLOY_SCHEMA_VERSION,
            entries: Vec::new(),
        });
    }
    let registry: InstallRegistry = read_json(&path)?;
    if registry.schema_version != DEPLOY_SCHEMA_VERSION {
        return Err(DeployError::InvalidManifest(format!(
            "Desteklenmeyen registry şeması: {}",
            registry.schema_version
        )));
    }
    Ok(registry)
}

fn write_registry(root: &Path, registry: &InstallRegistry) -> Result<(), DeployError> {
    atomic_write_json(root, &state_path(root, "registry.json")?, registry)
}

fn verify_plan(plan: &InstallPlan) -> Result<(), DeployError> {
    for change in &plan.changes {
        let target = checked_target(&plan.target_root, &change.path)?;
        let current = if target.exists() {
            Some(sha256_hex(&read_regular_file(&target)?))
        } else {
            None
        };
        if current != change.current_sha256 {
            return Err(DeployError::Conflict(format!(
                "Plan sonrası değişti: {}",
                change.path
            )));
        }
    }
    Ok(())
}

fn restore_applied(root: &Path, backup_root: &Path, tx: &TransactionRecord, applied: &[String]) {
    for path in applied.iter().rev() {
        if let Some(file) = tx.files.iter().find(|file| &file.path == path) {
            if file.had_original {
                if let Ok(contents) = read_regular_file(&backup_root.join("files").join(path)) {
                    let _ = atomic_write(root, &root.join(path), &contents);
                }
            } else {
                let _ = fs::remove_file(root.join(path));
            }
        }
    }
}

struct DeploymentLock {
    path: PathBuf,
}

impl DeploymentLock {
    fn acquire(root: &Path) -> Result<Self, DeployError> {
        let path = state_path(root, "install.lock")?;
        if let Some(parent) = path.parent() {
            create_directory_tree(root, parent)?;
        }
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    DeployError::Locked(path.clone())
                } else {
                    io("Deployment lock oluşturma", &path, source)
                }
            })?;
        Ok(Self { path })
    }
}

impl Drop for DeploymentLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn validate_root(root: &Path) -> Result<PathBuf, DeployError> {
    let metadata =
        fs::symlink_metadata(root).map_err(|_| DeployError::InvalidRoot(root.to_path_buf()))?;
    if !metadata.is_dir() {
        return Err(DeployError::InvalidRoot(root.to_path_buf()));
    }
    if is_link(&metadata) {
        return Err(DeployError::LinkTarget(root.to_path_buf()));
    }
    fs::canonicalize(root).map_err(|source| io("Deployment kökü çözümleme", root, source))
}

fn state_path(root: &Path, suffix: &str) -> Result<PathBuf, DeployError> {
    checked_target(root, &format!("{STATE_DIRECTORY}/{suffix}"))
}

fn checked_target(root: &Path, relative: &str) -> Result<PathBuf, DeployError> {
    let relative_path = Path::new(relative);
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || !relative_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(DeployError::UnsafePath(relative.to_owned()));
    }
    let target = root.join(relative_path);
    ensure_no_links(root, &target)?;
    Ok(target)
}

fn ensure_no_links(root: &Path, target: &Path) -> Result<(), DeployError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| DeployError::UnsafePath(target.display().to_string()))?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|source| io("Yol bilgisi okuma", &current, source))?;
            if is_link(&metadata) {
                return Err(DeployError::LinkTarget(current));
            }
        }
    }
    Ok(())
}

fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>, DeployError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| io("Dosya bilgisi okuma", path, source))?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(DeployError::LinkTarget(path.to_path_buf()));
    }
    fs::read(path).map_err(|source| io("Dosya okuma", path, source))
}

fn create_directory_tree(root: &Path, path: &Path) -> Result<(), DeployError> {
    ensure_no_links(root, path)?;
    fs::create_dir_all(path).map_err(|source| io("Klasör oluşturma", path, source))
}

fn write_new_synced(root: &Path, path: &Path, contents: &[u8]) -> Result<(), DeployError> {
    let parent = path
        .parent()
        .ok_or_else(|| DeployError::UnsafePath(path.display().to_string()))?;
    create_directory_tree(root, parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io("Yeni dosya oluşturma", path, source))?;
    file.write_all(contents)
        .map_err(|source| io("Dosya yazma", path, source))?;
    file.sync_all()
        .map_err(|source| io("Dosya eşitleme", path, source))
}

fn write_json_new<T: Serialize>(root: &Path, path: &Path, value: &T) -> Result<(), DeployError> {
    let data = serde_json::to_vec_pretty(value)
        .map_err(|e| DeployError::InvalidManifest(e.to_string()))?;
    write_new_synced(root, path, &data)
}

fn atomic_write_json<T: Serialize>(root: &Path, path: &Path, value: &T) -> Result<(), DeployError> {
    let data = serde_json::to_vec_pretty(value)
        .map_err(|e| DeployError::InvalidManifest(e.to_string()))?;
    atomic_write(root, path, &data)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, DeployError> {
    let data = read_regular_file(path)?;
    serde_json::from_slice(&data).map_err(|e| DeployError::InvalidManifest(e.to_string()))
}

fn atomic_write(root: &Path, target: &Path, contents: &[u8]) -> Result<(), DeployError> {
    ensure_no_links(root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| DeployError::UnsafePath(target.display().to_string()))?;
    create_directory_tree(root, parent)?;
    let name = target
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| DeployError::UnsafePath(target.display().to_string()))?;
    let staging = parent.join(format!(
        ".{name}.mt-dna-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    write_new_synced(root, &staging, contents)?;
    let result =
        replace_path(&staging, target).map_err(|source| io("Atomik değiştirme", target, source));
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

#[cfg(windows)]
fn replace_path(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_path(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

fn contains_secret(contents: &[u8]) -> bool {
    let text = String::from_utf8_lossy(contents).to_ascii_lowercase();
    [
        "api_key=",
        "apikey=",
        "secret=",
        "token=",
        "-----begin private key-----",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn new_transaction_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("tx-{nanos}-{}", std::process::id())
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn io(operation: &'static str, path: &Path, source: std::io::Error) -> DeployError {
    DeployError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, bytes) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|value| format!("{value:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_root(name: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/v07-tests")
            .join(format!(
                "mt-dna-v07-{name}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn package_with_body(body: &str) -> ProviderPackage {
        ProviderPackage {
            provider: Provider::Codex,
            files: vec![RenderedFile {
                relative_path: ".agents/skills/mt-developer-dna/SKILL.md".to_owned(),
                contents: format!("---\nname: mt-developer-dna\ndescription: test\n---\n{body}\n"),
            }],
            warnings: vec![],
        }
    }

    fn package() -> ProviderPackage {
        package_with_body("body")
    }

    fn write_cline_extension(root: &Path, version: &str) -> PathBuf {
        let extension = root.join(format!("saoudrizwan.claude-dev-{version}"));
        fs::create_dir_all(&extension).unwrap();
        fs::write(
            extension.join("package.json"),
            format!(
                r#"{{"publisher":"saoudrizwan","name":"claude-dev","displayName":"Cline","version":"{version}"}}"#
            ),
        )
        .unwrap();
        extension
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn install_is_idempotent_and_rollback_is_safe() {
        let root = temp_root("roundtrip");
        let plan = plan_install(&root, &package(), InstallScope::Project).unwrap();
        assert_eq!(plan.changes[0].kind, ChangeKind::Create);
        let report = apply_install(&plan, ConflictPolicy::Abort).unwrap();
        let tx = report.transaction_id.unwrap();
        let second = plan_install(&root, &package(), InstallScope::Project).unwrap();
        assert_eq!(second.changes[0].kind, ChangeKind::Unchanged);
        rollback(&root, &tx).unwrap();
        assert!(!root
            .join(".agents/skills/mt-developer-dna/SKILL.md")
            .exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unowned_and_user_modified_files_are_conflicts() {
        let root = temp_root("conflict");
        let target = root.join(".agents/skills/mt-developer-dna/SKILL.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "user").unwrap();
        let plan = plan_install(&root, &package(), InstallScope::Project).unwrap();
        assert_eq!(plan.changes[0].kind, ChangeKind::Conflict);
        assert!(matches!(
            apply_install(&plan, ConflictPolicy::Abort),
            Err(DeployError::Conflict(_))
        ));
        assert_eq!(fs::read_to_string(target).unwrap(), "user");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_stops_after_user_change() {
        let root = temp_root("rollback-conflict");
        let plan = plan_install(&root, &package(), InstallScope::Project).unwrap();
        let tx = apply_install(&plan, ConflictPolicy::Abort)
            .unwrap()
            .transaction_id
            .unwrap();
        let target = root.join(".agents/skills/mt-developer-dna/SKILL.md");
        fs::write(&target, "user change").unwrap();
        assert!(matches!(
            rollback(&root, &tx),
            Err(DeployError::RollbackConflict(_))
        ));
        assert_eq!(fs::read_to_string(target).unwrap(), "user change");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn update_rollback_restores_previous_registry_and_contents() {
        let root = temp_root("update-rollback");
        let first = plan_install(&root, &package_with_body("v1"), InstallScope::Project).unwrap();
        apply_install(&first, ConflictPolicy::Abort).unwrap();
        let second = plan_install(&root, &package_with_body("v2"), InstallScope::Project).unwrap();
        assert_eq!(second.changes[0].kind, ChangeKind::Update);
        let tx = apply_install(&second, ConflictPolicy::Abort)
            .unwrap()
            .transaction_id
            .unwrap();
        rollback(&root, &tx).unwrap();
        let target = root.join(".agents/skills/mt-developer-dna/SKILL.md");
        assert!(fs::read_to_string(target).unwrap().contains("v1"));
        assert!(status(&root)
            .unwrap()
            .iter()
            .all(|result| result.status == DiagnosticStatus::Pass));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn secret_conflict_is_not_copied_to_backup() {
        let root = temp_root("secret");
        let target = root.join(".agents/skills/mt-developer-dna/SKILL.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "api_key=do-not-copy").unwrap();
        let plan = plan_install(&root, &package(), InstallScope::Project).unwrap();
        assert!(matches!(
            apply_install(&plan, ConflictPolicy::BackupAndReplace),
            Err(DeployError::SecretBackupRefused(_))
        ));
        assert_eq!(fs::read_to_string(target).unwrap(), "api_key=do-not-copy");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn future_registry_schema_is_rejected() {
        let root = temp_root("future-registry");
        let registry = root.join(".mt-dna/deploy/registry.json");
        fs::create_dir_all(registry.parent().unwrap()).unwrap();
        fs::write(&registry, r#"{"schema_version":99,"entries":[]}"#).unwrap();
        assert!(matches!(
            plan_install(&root, &package(), InstallScope::Project),
            Err(DeployError::InvalidManifest(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_install_lock_is_respected() {
        let root = temp_root("lock");
        let plan = plan_install(&root, &package(), InstallScope::Project).unwrap();
        let lock = state_path(&root, "install.lock").unwrap();
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(&lock, "busy").unwrap();
        assert!(matches!(
            apply_install(&plan, ConflictPolicy::Abort),
            Err(DeployError::Locked(_))
        ));
        assert!(!root
            .join(".agents/skills/mt-developer-dna/SKILL.md")
            .exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn v08_registry_detects_every_provider_without_mutation() {
        let detections = detect_all();
        assert_eq!(detections.len(), 7);
        assert!(detections
            .iter()
            .any(|item| item.provider == ProviderKey::Antigravity));
        assert!(detections
            .iter()
            .any(|item| item.provider == ProviderKey::OpenCode));
        let cline = detections
            .iter()
            .find(|item| item.provider == ProviderKey::Cline)
            .unwrap();
        if cline.installed {
            assert!(cline
                .installations
                .iter()
                .all(|item| item.kind == InstallationKind::VsCodeExtension));
            assert!(cline.evidence.contains("etkin profil/enable durumu"));
        } else {
            assert!(cline.evidence.contains("saoudrizwan.claude-dev"));
        }
        let formatted = format_diagnostics(&[DiagnosticResult {
            provider: Some(ProviderKey::OpenCode),
            check: "probe".to_owned(),
            status: DiagnosticStatus::NotRun,
            detail: "canlı test yok".to_owned(),
        }]);
        assert!(formatted.contains("provider=opencode"));
    }

    #[test]
    fn global_antigravity_install_uses_documented_config_path() {
        let root = temp_root("antigravity-global");
        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/profile-rust.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::Antigravity).unwrap();
        let plan = plan_install(&root, &package, InstallScope::Global).unwrap();
        assert_eq!(plan.changes.len(), 1 + dna_render::SKILL_REFERENCES.len());
        assert_eq!(
            plan.changes[0].path,
            ".gemini/config/skills/mt-developer-dna/SKILL.md"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn global_skill_path_override_moves_references_with_the_skill() {
        let root = temp_root("references-override");
        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/universal.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::Antigravity).unwrap();
        let override_path = ".gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md";
        let plan = plan_install_with_global_skill_path(
            &root,
            &package,
            InstallScope::Global,
            Some(override_path),
        )
        .unwrap();

        assert_eq!(plan.changes.len(), 1 + dna_render::SKILL_REFERENCES.len());
        assert!(plan.changes.iter().all(|change| change
            .path
            .starts_with(".gemini/antigravity-cli/skills/mt-developer-dna/")));
        assert!(plan.changes.iter().any(|change| change.path
            == ".gemini/antigravity-cli/skills/mt-developer-dna/references/languages/csharp.json"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn antigravity_discovers_windows_cli_and_desktop_installations() {
        let home = temp_root("antigravity-discovery");
        let cli = home.join("AppData/Local/agy/bin/agy.exe");
        let desktop = home.join("AppData/Local/Programs/Antigravity IDE/Antigravity IDE.exe");
        fs::create_dir_all(cli.parent().unwrap()).unwrap();
        fs::create_dir_all(desktop.parent().unwrap()).unwrap();
        fs::write(&cli, b"fixture").unwrap();
        fs::write(&desktop, b"fixture").unwrap();

        let installations = discover_antigravity_under(&home);

        assert_eq!(installations.len(), 2);
        assert!(installations
            .iter()
            .any(|item| { item.kind == InstallationKind::Cli && item.path == cli }));
        assert!(installations
            .iter()
            .any(|item| { item.kind == InstallationKind::Desktop && item.path == desktop }));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn selected_antigravity_path_controls_global_skill_target() {
        let root = temp_root("antigravity-selected");
        let cli = root.join("agy.exe");
        fs::write(&cli, b"fixture").unwrap();
        let detection = detect_provider_with_path(ProviderKey::Antigravity, Some(&cli)).unwrap();
        assert!(detection.installed);
        assert_eq!(detection.installations[0].kind, InstallationKind::Cli);
        assert_eq!(
            detection.global_skill_path,
            ".gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md"
        );

        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/profile-rust.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::Antigravity).unwrap();
        let plan = plan_install_with_global_skill_path(
            &root,
            &package,
            InstallScope::Global,
            Some(&detection.global_skill_path),
        )
        .unwrap();
        assert_eq!(
            plan.changes[0].path,
            ".gemini/antigravity-cli/skills/mt-developer-dna/SKILL.md"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selected_antigravity_desktop_and_invalid_paths_are_distinguished() {
        let root = temp_root("antigravity-explicit");
        let desktop = root.join("Antigravity IDE.exe");
        let invalid = root.join("antigravity.exe");
        fs::write(&desktop, b"fixture").unwrap();
        fs::write(&invalid, b"fixture").unwrap();

        let detection =
            detect_provider_with_path(ProviderKey::Antigravity, Some(&desktop)).unwrap();
        assert_eq!(detection.installations[0].kind, InstallationKind::Desktop);
        assert_eq!(
            detection.global_skill_path,
            ".gemini/config/skills/mt-developer-dna/SKILL.md"
        );
        let directory_detection =
            detect_provider_with_path(ProviderKey::Antigravity, Some(&root)).unwrap();
        assert_eq!(
            directory_detection.installations[0].kind,
            InstallationKind::Desktop
        );
        assert!(matches!(
            detect_provider_with_path(ProviderKey::Antigravity, Some(&invalid)),
            Err(DeployError::InvalidProviderPath(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cline_extension_discovery_validates_manifest_identity_and_versions() {
        let root = temp_root("cline-extension-root");
        let older = write_cline_extension(&root, "4.1.24");
        let newer = write_cline_extension(&root, "4.1.25");
        let imitation = root.join("saoudrizwan.claude-dev-99.0.0");
        fs::create_dir_all(&imitation).unwrap();
        fs::write(
            imitation.join("package.json"),
            r#"{"publisher":"someone-else","name":"claude-dev","version":"99.0.0"}"#,
        )
        .unwrap();

        let installations = discover_cline_in_extension_root(&root, "test extension root");

        assert_eq!(installations.len(), 2);
        assert!(installations.iter().any(|item| item.path == older));
        assert!(installations.iter().any(|item| item.path == newer));
        assert!(installations
            .iter()
            .all(|item| item.kind == InstallationKind::VsCodeExtension));
        let detection = detect_provider_with_path(ProviderKey::Cline, Some(&root)).unwrap();
        assert_eq!(detection.version.as_deref(), Some("4.1.25"));
        assert!(detection.installed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selected_cline_extension_directory_and_invalid_manifest_are_distinguished() {
        let root = temp_root("cline-selected-extension");
        let extension = write_cline_extension(&root, "4.2.0");
        let detection = detect_provider_with_path(ProviderKey::Cline, Some(&extension)).unwrap();
        assert_eq!(detection.version.as_deref(), Some("4.2.0"));
        assert_eq!(
            detection.installations[0].kind,
            InstallationKind::VsCodeExtension
        );

        let invalid = root.join("invalid-extension");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(
            invalid.join("package.json"),
            r#"{"publisher":"saoudrizwan","name":"not-cline","version":"1.0.0"}"#,
        )
        .unwrap();
        assert!(matches!(
            detect_provider_with_path(ProviderKey::Cline, Some(&invalid)),
            Err(DeployError::InvalidProviderPath(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn format_plan_lists_changes_without_claiming_apply_outcome() {
        let root = temp_root("format-plan-outcome");
        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/universal.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::Cline).unwrap();
        let plan = plan_install(&root, &package, InstallScope::Project).unwrap();

        let text = format_plan(&plan);

        assert!(text.starts_with("Deployment planı: provider=cline"));
        assert!(text.contains(".cline/skills/mt-developer-dna/SKILL.md"));
        assert!(
            !text.contains("değiştirilmedi") && !text.contains("Dry-run"),
            "plan çıktısı uygulama sonucunu bilmez; sonucu çağıran yazmalı: {text}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cline_universal_profile_supports_project_and_global_dry_run_targets() {
        let root = temp_root("cline-universal-plan");
        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/universal.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::Cline).unwrap();

        let project = plan_install(&root, &package, InstallScope::Project).unwrap();
        assert!(project
            .changes
            .iter()
            .any(|item| item.path == ".cline/skills/mt-developer-dna/SKILL.md"));
        assert!(project
            .changes
            .iter()
            .any(|item| item.path == ".clinerules/mt-developer-dna.md"));

        let global = plan_install(&root, &package, InstallScope::Global).unwrap();
        assert_eq!(global.changes.len(), 1 + dna_render::SKILL_REFERENCES.len());
        assert_eq!(
            global.changes[0].path,
            ".cline/skills/mt-developer-dna/SKILL.md"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn opencode_discovers_windows_npm_shims_and_executable_from_path() {
        let root = temp_root("opencode-path");
        fs::write(root.join("opencode.exe"), b"fixture").unwrap();
        fs::write(root.join("opencode.cmd"), b"@echo 9.8.7\r\n").unwrap();
        fs::write(root.join("opencode.ps1"), b"Write-Output '9.8.7'\r\n").unwrap();

        let installations = discover_npm_runners_in_path(ProviderKey::OpenCode, [root.clone()]);

        assert_eq!(installations.len(), 3);
        assert!(installations
            .iter()
            .any(|item| item.path.ends_with("opencode.exe") && item.kind == InstallationKind::Cli));
        assert!(installations.iter().any(|item| {
            item.path.ends_with("opencode.cmd") && item.kind == InstallationKind::NpmShim
        }));
        assert!(installations.iter().any(|item| {
            item.path.ends_with("opencode.ps1") && item.kind == InstallationKind::NpmShim
        }));
        #[cfg(windows)]
        assert!(installations.iter().any(|item| {
            item.path.ends_with("opencode.cmd") && item.version.as_deref() == Some("9.8.7")
        }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selected_opencode_runner_and_unsafe_cmd_path_are_handled_safely() {
        let root = temp_root("opencode-selected");
        let runner = root.join("opencode.ps1");
        fs::write(&runner, b"Write-Output '1.2.3'\r\n").unwrap();
        let detection = detect_provider_with_path(ProviderKey::OpenCode, Some(&runner)).unwrap();
        assert!(detection.installed);
        assert_eq!(detection.installations[0].kind, InstallationKind::NpmShim);

        let invalid = root.join("not-opencode.cmd");
        fs::write(&invalid, b"@echo 1.0.0\r\n").unwrap();
        assert!(matches!(
            detect_provider_with_path(ProviderKey::OpenCode, Some(&invalid)),
            Err(DeployError::InvalidProviderPath(_))
        ));
        assert!(safe_cmd_version_invocation(Path::new("C:/safe/opencode.cmd")).is_some());
        assert!(safe_cmd_version_invocation(Path::new("C:/unsafe&path/opencode.cmd")).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn codex_discovers_windows_npm_shims_from_path() {
        let root = temp_root("codex-path");
        fs::write(root.join("codex.cmd"), b"@echo codex-cli 0.99.0\r\n").unwrap();
        fs::write(
            root.join("codex.ps1"),
            b"Write-Output 'codex-cli 0.99.0'\r\n",
        )
        .unwrap();
        // Uzantısız sh shim'i ve başka provider'ın shim'i Codex sayılmamalı.
        fs::write(root.join("codex"), b"#!/bin/sh\n").unwrap();
        fs::write(root.join("opencode.cmd"), b"@echo 1.0.0\r\n").unwrap();

        let installations = discover_npm_runners_in_path(ProviderKey::Codex, [root.clone()]);

        assert_eq!(installations.len(), 2);
        assert!(installations
            .iter()
            .all(|item| item.kind == InstallationKind::NpmShim));
        assert!(installations
            .iter()
            .any(|item| item.path.ends_with("codex.cmd")));
        assert!(installations
            .iter()
            .any(|item| item.path.ends_with("codex.ps1")));
        #[cfg(windows)]
        assert!(installations.iter().any(|item| {
            item.path.ends_with("codex.cmd") && item.version.as_deref() == Some("codex-cli 0.99.0")
        }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selected_codex_cmd_shim_is_accepted_and_foreign_runner_rejected() {
        let root = temp_root("codex-selected");
        let runner = root.join("codex.cmd");
        fs::write(&runner, b"@echo codex-cli 0.99.0\r\n").unwrap();
        let detection = detect_provider_with_path(ProviderKey::Codex, Some(&runner)).unwrap();
        assert!(detection.installed);
        assert_eq!(detection.installations[0].kind, InstallationKind::NpmShim);
        assert!(detection.installations[0].source.contains("Codex CLI"));

        let from_dir = detect_provider_with_path(ProviderKey::Codex, Some(&root)).unwrap();
        assert!(from_dir.installed);

        let foreign = root.join("opencode.cmd");
        fs::write(&foreign, b"@echo 1.0.0\r\n").unwrap();
        assert!(matches!(
            detect_provider_with_path(ProviderKey::Codex, Some(&foreign)),
            Err(DeployError::InvalidProviderPath(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn opencode_universal_profile_supports_project_and_global_dry_run_targets() {
        let root = temp_root("opencode-universal-plan");
        let profile = dna_core::Profile::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/universal.json"),
        )
        .unwrap();
        let package = dna_render::render_package(&profile, Provider::OpenCode).unwrap();

        let project = plan_install(&root, &package, InstallScope::Project).unwrap();
        assert!(project
            .changes
            .iter()
            .any(|item| item.path == ".opencode/skills/mt-developer-dna/SKILL.md"));

        let global = plan_install(&root, &package, InstallScope::Global).unwrap();
        assert_eq!(global.changes.len(), 1 + dna_render::SKILL_REFERENCES.len());
        assert_eq!(
            global.changes[0].path,
            ".config/opencode/skills/mt-developer-dna/SKILL.md"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rust_profile_can_migrate_to_universal_and_rollback() {
        let root = temp_root("universal-migration");
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rust_profile =
            dna_core::Profile::load(workspace.join("tests/fixtures/profile-rust.json")).unwrap();
        let universal = dna_core::Profile::load(workspace.join("profiles/universal.json")).unwrap();
        let rust_package = dna_render::render_package(&rust_profile, Provider::Codex).unwrap();
        let initial = plan_install(&root, &rust_package, InstallScope::Global).unwrap();
        apply_install(&initial, ConflictPolicy::Abort).unwrap();

        let universal_package = dna_render::render_package(&universal, Provider::Codex).unwrap();
        let migration = plan_install(&root, &universal_package, InstallScope::Global).unwrap();
        assert_eq!(migration.changes[0].kind, ChangeKind::Update);
        let transaction = apply_install(&migration, ConflictPolicy::Abort)
            .unwrap()
            .transaction_id
            .unwrap();
        let target = root.join(".agents/skills/mt-developer-dna/SKILL.md");
        assert!(fs::read_to_string(&target)
            .unwrap()
            .contains("Programlama dili seçilmedi"));
        rollback(&root, &transaction).unwrap();
        assert!(fs::read_to_string(&target).unwrap().contains("Rust"));
        let _ = fs::remove_dir_all(root);
    }
}
