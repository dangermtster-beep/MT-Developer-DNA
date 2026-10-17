use super::*;

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

pub(crate) fn command_version(command: &str) -> Option<String> {
    command_version_path(Path::new(command))
}

pub(crate) fn command_version_path(command: &Path) -> Option<String> {
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
pub(crate) fn cmd_script_version_output(command_text: &str) -> Option<std::process::Output> {
    use std::os::windows::process::CommandExt as _;
    Command::new("cmd.exe")
        .args(["/D", "/S", "/C"])
        .raw_arg(command_text)
        .output()
        .ok()
}

#[cfg(not(windows))]
pub(crate) fn cmd_script_version_output(command_text: &str) -> Option<std::process::Output> {
    Command::new("cmd.exe")
        .args(["/D", "/S", "/C", command_text])
        .output()
        .ok()
}

pub(crate) fn safe_cmd_version_invocation(command: &Path) -> Option<String> {
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

pub(crate) fn unavailable_detection(provider: ProviderKey, evidence: String) -> ProviderDetection {
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

pub(crate) fn discover_command_installation(provider: ProviderKey) -> Vec<ProviderInstallation> {
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

pub(crate) fn discover_antigravity() -> Vec<ProviderInstallation> {
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

pub(crate) fn discover_antigravity_under(home: &Path) -> Vec<ProviderInstallation> {
    discover_antigravity_in_local_app_data(&home.join("AppData/Local"))
}

pub(crate) fn discover_antigravity_in_local_app_data(
    local_app_data: &Path,
) -> Vec<ProviderInstallation> {
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

pub(crate) fn discover_cline() -> Vec<ProviderInstallation> {
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
pub(crate) fn supports_npm_runners(provider: ProviderKey) -> bool {
    matches!(provider, ProviderKey::Codex | ProviderKey::OpenCode)
}

pub(crate) fn npm_runner_names(provider: ProviderKey) -> [String; 3] {
    let command = provider.executable().unwrap_or_default();
    ["exe", "cmd", "ps1"].map(|extension| format!("{command}.{extension}"))
}

pub(crate) fn discover_npm_runners(provider: ProviderKey) -> Vec<ProviderInstallation> {
    let Some(path) = env::var_os("PATH") else {
        return Vec::new();
    };
    discover_npm_runners_in_path(provider, env::split_paths(&path))
}

pub(crate) fn discover_npm_runners_in_path(
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

pub(crate) fn discover_cline_in_extension_root(
    root: &Path,
    source: &str,
) -> Vec<ProviderInstallation> {
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

pub(crate) fn cline_extension_installation(
    path: &Path,
    source: &str,
) -> Option<ProviderInstallation> {
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

pub(crate) fn validate_selected_installations(
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

pub(crate) fn is_npm_runner(provider: ProviderKey, path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            npm_runner_names(provider)
                .iter()
                .any(|runner| name.eq_ignore_ascii_case(runner))
        })
}

pub(crate) fn regular_non_link_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_file() && !is_link(&metadata))
        .unwrap_or(false)
}

pub(crate) fn regular_non_link_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_link(&metadata))
        .unwrap_or(false)
}

pub(crate) fn compare_version_text(left: &str, right: &str) -> std::cmp::Ordering {
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

pub(crate) fn deduplicate_installations(installations: &mut Vec<ProviderInstallation>) {
    installations.sort_by(|left, right| left.path.cmp(&right.path));
    installations.dedup_by(|left, right| left.path == right.path);
}

pub(crate) fn command_path(command: &str) -> Option<PathBuf> {
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
