use crate::CliError;
use dna_render::ProviderPackage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const BACKUP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatus {
    Create,
    Update,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallChange {
    pub relative_path: String,
    pub status: InstallStatus,
    pub diff: String,
    old_contents: Option<Vec<u8>>,
    new_contents: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub project_root: PathBuf,
    pub changes: Vec<InstallChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub changed: usize,
    pub unchanged: usize,
    pub backup_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackReport {
    pub restored: usize,
    pub removed: usize,
    pub backup_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    schema_version: u32,
    backup_id: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupEntry {
    relative_path: String,
    had_original: bool,
}

pub fn plan_install(
    project_root: &Path,
    packages: &[ProviderPackage],
) -> Result<InstallPlan, CliError> {
    let project_root = validate_project_root(project_root)?;
    let mut outputs = BTreeMap::<String, Vec<u8>>::new();

    for package in packages {
        for file in &package.files {
            if !file.has_safe_relative_path() {
                return Err(CliError::UnsafeOutputPath(file.relative_path.clone()));
            }
            match outputs.get(&file.relative_path) {
                Some(existing) if existing != file.contents.as_bytes() => {
                    return Err(CliError::ConflictingOutput(file.relative_path.clone()));
                }
                Some(_) => {}
                None => {
                    outputs.insert(
                        file.relative_path.clone(),
                        file.contents.as_bytes().to_vec(),
                    );
                }
            }
        }
    }

    let mut changes = Vec::with_capacity(outputs.len());
    for (relative_path, new_contents) in outputs {
        let target = checked_target(&project_root, &relative_path)?;
        let old_contents = if target.exists() {
            let metadata = fs::symlink_metadata(&target).map_err(|source| CliError::Io {
                operation: "Hedef bilgisi okunamadı",
                path: target.clone(),
                source,
            })?;
            if is_link_or_reparse(&metadata) {
                return Err(CliError::SymlinkTarget(target));
            }
            if !metadata.is_file() {
                return Err(CliError::InvalidTarget(target));
            }
            Some(fs::read(&target).map_err(|source| CliError::Io {
                operation: "Mevcut dosya okunamadı",
                path: target.clone(),
                source,
            })?)
        } else {
            None
        };

        let status = match old_contents.as_deref() {
            Some(old) if old == new_contents.as_slice() => InstallStatus::Unchanged,
            Some(_) => InstallStatus::Update,
            None => InstallStatus::Create,
        };
        let diff = make_diff(old_contents.as_deref(), &new_contents);
        changes.push(InstallChange {
            relative_path,
            status,
            diff,
            old_contents,
            new_contents,
        });
    }

    Ok(InstallPlan {
        project_root,
        changes,
    })
}

pub fn format_install_plan(plan: &InstallPlan) -> String {
    let mut output = String::from("Kurulum planı:\n");
    for change in &plan.changes {
        let status = match change.status {
            InstallStatus::Create => "YENİ",
            InstallStatus::Update => "GÜNCELLE",
            InstallStatus::Unchanged => "AYNI",
        };
        output.push_str(&format!("\n[{status}] {}\n", change.relative_path));
        if change.status != InstallStatus::Unchanged {
            output.push_str(&change.diff);
        }
    }
    output
}

pub fn apply_install(plan: &InstallPlan) -> Result<InstallReport, CliError> {
    let changed = plan
        .changes
        .iter()
        .filter(|change| change.status != InstallStatus::Unchanged)
        .count();
    let unchanged = plan.changes.len() - changed;
    if changed == 0 {
        return Ok(InstallReport {
            changed,
            unchanged,
            backup_id: None,
        });
    }

    let changed_entries = plan
        .changes
        .iter()
        .filter(|change| change.status != InstallStatus::Unchanged)
        .collect::<Vec<_>>();
    for change in &changed_entries {
        verify_target_still_matches_plan(&plan.project_root, change)?;
    }

    let backup_id = new_backup_id();
    let backup_root = checked_target(
        &plan.project_root,
        &format!(".mt-developer-dna/backups/{backup_id}"),
    )?;
    create_new_directory_tree(&plan.project_root, &backup_root)?;

    for change in &changed_entries {
        if let Some(old) = &change.old_contents {
            let backup_target = backup_root.join("files").join(&change.relative_path);
            write_new_synced(&plan.project_root, &backup_target, old)?;
        }
    }

    let manifest = BackupManifest {
        schema_version: BACKUP_SCHEMA_VERSION,
        backup_id: backup_id.clone(),
        entries: changed_entries
            .iter()
            .map(|change| BackupEntry {
                relative_path: change.relative_path.clone(),
                had_original: change.old_contents.is_some(),
            })
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))?;
    write_new_synced(
        &plan.project_root,
        &backup_root.join("manifest.json"),
        &manifest_bytes,
    )?;

    let mut applied = Vec::new();
    for change in &changed_entries {
        let target = checked_target(&plan.project_root, &change.relative_path)?;
        if let Err(error) = atomic_write(&plan.project_root, &target, &change.new_contents) {
            for applied_change in applied.into_iter().rev() {
                let _ = restore_change(&plan.project_root, applied_change);
            }
            return Err(error);
        }
        applied.push(*change);
    }

    Ok(InstallReport {
        changed,
        unchanged,
        backup_id: Some(backup_id),
    })
}

fn verify_target_still_matches_plan(root: &Path, change: &InstallChange) -> Result<(), CliError> {
    let target = checked_target(root, &change.relative_path)?;
    match &change.old_contents {
        Some(expected) => {
            let current = fs::read(&target).map_err(|source| CliError::Io {
                operation: "Hedef yeniden okunamadı",
                path: target.clone(),
                source,
            })?;
            if &current != expected {
                return Err(CliError::TargetChanged(target));
            }
        }
        None if target.exists() => return Err(CliError::TargetChanged(target)),
        None => {}
    }
    Ok(())
}

pub fn rollback_install(project_root: &Path, backup_id: &str) -> Result<RollbackReport, CliError> {
    if !is_safe_backup_id(backup_id) {
        return Err(CliError::InvalidBackupId(backup_id.to_owned()));
    }
    let project_root = validate_project_root(project_root)?;
    let backup_root = checked_target(
        &project_root,
        &format!(".mt-developer-dna/backups/{backup_id}"),
    )?;
    let manifest_path = backup_root.join("manifest.json");
    let source = fs::read_to_string(&manifest_path).map_err(|source| CliError::Io {
        operation: "Yedek manifesti okunamadı",
        path: manifest_path.clone(),
        source,
    })?;
    let manifest: BackupManifest = serde_json::from_str(&source)
        .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))?;
    if manifest.schema_version != BACKUP_SCHEMA_VERSION || manifest.backup_id != backup_id {
        return Err(CliError::InvalidBackupManifest(
            "Sürüm veya yedek kimliği eşleşmiyor.".to_owned(),
        ));
    }

    let mut restored = 0;
    let mut removed = 0;
    for entry in &manifest.entries {
        let target = checked_target(&project_root, &entry.relative_path)?;
        if entry.had_original {
            let backup_file = backup_root.join("files").join(&entry.relative_path);
            let contents = fs::read(&backup_file).map_err(|source| CliError::Io {
                operation: "Yedek dosya okunamadı",
                path: backup_file,
                source,
            })?;
            atomic_write(&project_root, &target, &contents)?;
            restored += 1;
        } else if target.exists() {
            ensure_path_has_no_links(&project_root, &target)?;
            fs::remove_file(&target).map_err(|source| CliError::Io {
                operation: "Rollback yeni dosyayı kaldıramadı",
                path: target,
                source,
            })?;
            removed += 1;
        }
    }

    Ok(RollbackReport {
        restored,
        removed,
        backup_path: backup_root,
    })
}

fn validate_project_root(project_root: &Path) -> Result<PathBuf, CliError> {
    let metadata = fs::symlink_metadata(project_root)
        .map_err(|_| CliError::InvalidProjectRoot(project_root.to_path_buf()))?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err(if is_link_or_reparse(&metadata) {
            CliError::SymlinkTarget(project_root.to_path_buf())
        } else {
            CliError::InvalidProjectRoot(project_root.to_path_buf())
        });
    }
    fs::canonicalize(project_root).map_err(|source| CliError::Io {
        operation: "Proje kökü çözümlenemedi",
        path: project_root.to_path_buf(),
        source,
    })
}

fn checked_target(root: &Path, relative_path: &str) -> Result<PathBuf, CliError> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(CliError::UnsafeOutputPath(relative_path.to_owned()));
    }
    let target = root.join(relative);
    ensure_path_has_no_links(root, &target)?;
    Ok(target)
}

fn ensure_path_has_no_links(root: &Path, target: &Path) -> Result<(), CliError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| CliError::UnsafeOutputPath(target.display().to_string()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current).map_err(|source| CliError::Io {
                operation: "Yol bilgisi okunamadı",
                path: current.clone(),
                source,
            })?;
            if is_link_or_reparse(&metadata) {
                return Err(CliError::SymlinkTarget(current));
            }
        }
    }
    Ok(())
}

fn create_new_directory_tree(root: &Path, directory: &Path) -> Result<(), CliError> {
    ensure_path_has_no_links(root, directory)?;
    fs::create_dir_all(directory).map_err(|source| CliError::Io {
        operation: "Klasör oluşturma",
        path: directory.to_path_buf(),
        source,
    })
}

fn write_new_synced(root: &Path, path: &Path, contents: &[u8]) -> Result<(), CliError> {
    let parent = path
        .parent()
        .ok_or_else(|| CliError::UnsafeOutputPath(path.display().to_string()))?;
    create_new_directory_tree(root, parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| CliError::Io {
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

fn atomic_write(root: &Path, target: &Path, contents: &[u8]) -> Result<(), CliError> {
    ensure_path_has_no_links(root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| CliError::UnsafeOutputPath(target.display().to_string()))?;
    create_new_directory_tree(root, parent)?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CliError::UnsafeOutputPath(target.display().to_string()))?;
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".{file_name}.mt-dna-{}-{counter}.tmp",
        std::process::id()
    ));
    write_new_synced(root, &staging, contents)?;

    let result = replace_path(&staging, target).map_err(|source| CliError::Io {
        operation: "Atomik dosya değiştirme",
        path: target.to_path_buf(),
        source,
    });
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
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
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

fn restore_change(root: &Path, change: &InstallChange) -> Result<(), CliError> {
    let target = checked_target(root, &change.relative_path)?;
    if let Some(old) = &change.old_contents {
        atomic_write(root, &target, old)
    } else if target.exists() {
        fs::remove_file(&target).map_err(|source| CliError::Io {
            operation: "Başarısız kurulum geri alınamadı",
            path: target,
            source,
        })
    } else {
        Ok(())
    }
}

fn make_diff(old: Option<&[u8]>, new: &[u8]) -> String {
    let mut output = String::new();
    match old {
        Some(old) => {
            output.push_str("--- mevcut\n+++ üretilecek\n");
            for line in String::from_utf8_lossy(old).lines() {
                output.push_str(&format!("-{line}\n"));
            }
        }
        None => output.push_str("--- /dev/null\n+++ üretilecek\n"),
    }
    for line in String::from_utf8_lossy(new).lines() {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

fn new_backup_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

fn is_safe_backup_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
