use crate::model::*;
use crate::render::{
    append_decision, architecture_document, initial_decisions, initial_progress, resume_summary,
    update_progress, HandoffRenderer,
};
use crate::scanner::ProjectScanner;
use crate::security::{checked_path, ensure_no_links, validate_project_root, SecretRedactor};
use crate::time::utc_now;
use crate::{content_hash, MemoryError};
use dna_core::DevelopmentPlan;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const REQUIRED_FILES: [&str; 7] = [
    "project.json",
    "architecture.md",
    "decisions.md",
    "standards.json",
    "progress.md",
    "handoff.md",
    "manifest.json",
];

pub struct SchemaValidator;

impl SchemaValidator {
    pub fn project(project: &ProjectMetadata) -> Result<(), MemoryError> {
        if project.schema_version != MEMORY_SCHEMA_VERSION {
            return Err(MemoryError::UnsupportedSchema {
                found: project.schema_version.clone(),
                expected: MEMORY_SCHEMA_VERSION.to_owned(),
            });
        }
        if !safe_id(&project.project_id) {
            return Err(MemoryError::InvalidField {
                field: "project_id",
                message: "küçük ASCII harf, rakam ve tire içeren 1-64 karakter olmalıdır"
                    .to_owned(),
            });
        }
        validate_text("name", &project.name, 1, 120)?;
        if !supported_language(&project.language) {
            return Err(MemoryError::InvalidField {
                field: "language",
                message: "desteklenen ve kullanıcı tarafından seçilmiş bir dil olmalıdır"
                    .to_owned(),
            });
        }
        validate_text("project_type", &project.project_type, 1, 60)?;
        if project.platforms.is_empty()
            || project.platforms.iter().any(|item| item.trim().is_empty())
        {
            return Err(MemoryError::InvalidField {
                field: "platforms",
                message: "en az bir boş olmayan platform içermelidir".to_owned(),
            });
        }
        for (field, value) in [
            ("created_at", project.created_at.as_str()),
            ("updated_at", project.updated_at.as_str()),
        ] {
            if !looks_like_utc_timestamp(value) {
                return Err(MemoryError::InvalidField {
                    field,
                    message: "UTC ISO 8601 biçiminde olmalıdır".to_owned(),
                });
            }
        }
        Ok(())
    }

    pub fn standards(standards: &MemoryStandards, language: &str) -> Result<(), MemoryError> {
        if standards.schema_version != MEMORY_SCHEMA_VERSION {
            return Err(MemoryError::UnsupportedSchema {
                found: standards.schema_version.clone(),
                expected: MEMORY_SCHEMA_VERSION.to_owned(),
            });
        }
        if standards.language != language {
            return Err(MemoryError::InvalidField {
                field: "standards.language",
                message: "project.json içindeki seçili dille aynı olmalıdır".to_owned(),
            });
        }
        for (field, rules) in [
            ("architecture", &standards.architecture),
            ("quality", &standards.quality),
            ("security", &standards.security),
            ("performance", &standards.performance),
        ] {
            if rules.iter().any(|rule| rule.trim().is_empty()) {
                return Err(MemoryError::InvalidField {
                    field,
                    message: "boş kural içeremez".to_owned(),
                });
            }
        }
        Ok(())
    }

    pub fn manifest(manifest: &MemoryManifest) -> Result<(), MemoryError> {
        if manifest.schema_version != MEMORY_SCHEMA_VERSION {
            return Err(MemoryError::UnsupportedSchema {
                found: manifest.schema_version.clone(),
                expected: MEMORY_SCHEMA_VERSION.to_owned(),
            });
        }
        let mut paths = BTreeSet::new();
        for file in &manifest.files {
            if file.schema_version != MEMORY_SCHEMA_VERSION || !safe_memory_relative(&file.path) {
                return Err(MemoryError::InvalidField {
                    field: "manifest.files",
                    message: format!("geçersiz yönetilen dosya: {}", file.path),
                });
            }
            if !paths.insert(&file.path) {
                return Err(MemoryError::InvalidField {
                    field: "manifest.files",
                    message: format!("yinelenen dosya: {}", file.path),
                });
            }
        }
        Ok(())
    }
}

pub struct ConflictDetector;

impl ConflictDetector {
    fn verify(plan: &MemoryPlan) -> Result<(), MemoryError> {
        if let Some(expected) = plan.expected_generation {
            let manifest_path = checked_path(&plan.project_root, ".mt-dna/manifest.json")?;
            let current: MemoryManifest = read_json(&manifest_path)?;
            if current.generation != expected {
                return Err(MemoryError::Conflict(format!(
                    "manifest nesli {} yerine {}",
                    expected, current.generation
                )));
            }
        } else if plan.initialize && plan.project_root.join(MEMORY_DIRECTORY).exists() {
            return Err(MemoryError::AlreadyInitialized(
                plan.project_root.join(MEMORY_DIRECTORY),
            ));
        }

        for (relative, expected) in &plan.expected_hashes {
            let path = checked_path(&plan.project_root, relative)?;
            let actual = if path.exists() {
                Some(content_hash(&read_bytes(&path)?))
            } else {
                None
            };
            if &actual != expected {
                return Err(MemoryError::Conflict(relative.clone()));
            }
        }
        Ok(())
    }
}

pub struct MemoryStore;

impl MemoryStore {
    pub fn plan_initialize(root: &Path, request: InitRequest) -> Result<MemoryPlan, MemoryError> {
        let root = validate_project_root(root)?;
        let memory_root = checked_path(&root, MEMORY_DIRECTORY)?;
        if memory_root.exists() {
            return Err(MemoryError::AlreadyInitialized(memory_root));
        }
        let timestamp = utc_now();
        let project = ProjectMetadata {
            schema_version: MEMORY_SCHEMA_VERSION.to_owned(),
            project_id: request.project_id,
            name: SecretRedactor::redact(&request.name),
            language: request.language,
            project_type: SecretRedactor::redact(&request.project_type),
            platforms: request
                .platforms
                .iter()
                .map(|item| SecretRedactor::redact(item))
                .collect(),
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
            last_provider: request.provider,
        };
        SchemaValidator::project(&project)?;
        let standards = MemoryStandards {
            schema_version: MEMORY_SCHEMA_VERSION.to_owned(),
            language: project.language.clone(),
            architecture: redact_values(request.architecture_rules),
            quality: redact_values(request.quality_rules),
            security: redact_values(request.security_rules),
            performance: redact_values(request.performance_rules),
        };
        SchemaValidator::standards(&standards, &project.language)?;
        let scan = ProjectScanner::scan(&root)?;
        let report_path = format!("reports/{}-review.md", &timestamp[..10]);
        let mut files = BTreeMap::new();
        insert_json(&mut files, "project.json", &project)?;
        insert_text(
            &mut files,
            "architecture.md",
            architecture_document(&project, &scan),
        );
        insert_text(&mut files, "decisions.md", initial_decisions());
        insert_json(&mut files, "standards.json", &standards)?;
        insert_text(&mut files, "progress.md", initial_progress());
        insert_text(
            &mut files,
            "handoff.md",
            "# Provider aktarımı\n\nHenüz aktarım hazırlanmadı.\n".to_owned(),
        );
        insert_text(
            &mut files,
            &report_path,
            format!(
                "# Hafıza incelemesi — {}\n\n- Durum: Başlatıldı\n- Test durumu: Test edilmedi.\n- Kaynak parmak izi: `{}`\n",
                &timestamp[..10], scan.fingerprint
            ),
        );
        build_plan(root, None, files, scan.fingerprint, true)
    }

    pub fn load(root: &Path) -> Result<MemorySnapshot, MemoryError> {
        let root = validate_project_root(root)?;
        require_initialized(&root)?;
        let project_path = checked_path(&root, ".mt-dna/project.json")?;
        let standards_path = checked_path(&root, ".mt-dna/standards.json")?;
        let manifest_path = checked_path(&root, ".mt-dna/manifest.json")?;
        let project: ProjectMetadata = read_json(&project_path)?;
        SchemaValidator::project(&project)?;
        let standards: MemoryStandards = read_json(&standards_path)?;
        SchemaValidator::standards(&standards, &project.language)?;
        let manifest: MemoryManifest = read_json(&manifest_path)?;
        SchemaValidator::manifest(&manifest)?;
        Ok(MemorySnapshot {
            project,
            standards,
            manifest,
            architecture: read_text(&checked_path(&root, ".mt-dna/architecture.md")?)?,
            decisions: read_text(&checked_path(&root, ".mt-dna/decisions.md")?)?,
            progress: read_text(&checked_path(&root, ".mt-dna/progress.md")?)?,
            handoff: read_text(&checked_path(&root, ".mt-dna/handoff.md")?)?,
        })
    }

    pub fn plan_update(root: &Path, request: UpdateRequest) -> Result<MemoryPlan, MemoryError> {
        let root = validate_project_root(root)?;
        let mut snapshot = Self::load(&root)?;
        let timestamp = utc_now();
        snapshot.project.updated_at = timestamp.clone();
        snapshot.project.last_provider = request.provider;
        if let Some(decision) = &request.decision {
            snapshot.decisions = append_decision(&snapshot.decisions, decision, &timestamp);
        }
        snapshot.progress = update_progress(&request);
        let scan = ProjectScanner::scan(&root)?;
        let files = snapshot_files(&root, &snapshot)?;
        build_plan(
            root,
            Some(snapshot.manifest.generation),
            files,
            scan.fingerprint,
            false,
        )
    }

    pub fn plan_handoff(root: &Path, target: ProviderId) -> Result<MemoryPlan, MemoryError> {
        let root = validate_project_root(root)?;
        let mut snapshot = Self::load(&root)?;
        let timestamp = utc_now();
        snapshot.project.updated_at = timestamp.clone();
        snapshot.project.last_provider = target;
        let plan_path = checked_path(&root, ".mt-dna/plans/current.json")?;
        let current_plan = if plan_path.is_file() {
            let plan: DevelopmentPlan = read_json(&plan_path)?;
            plan.validate().map_err(|error| MemoryError::InvalidField {
                field: "workflow",
                message: error.to_string(),
            })?;
            Some(plan)
        } else {
            None
        };
        let quality_summary = latest_quality_summary(&root, &snapshot.manifest)?;
        snapshot.handoff = HandoffRenderer::render_with_context(
            &snapshot,
            target,
            &timestamp,
            current_plan.as_ref(),
            quality_summary.as_deref(),
        );
        let scan = ProjectScanner::scan(&root)?;
        let files = snapshot_files(&root, &snapshot)?;
        build_plan(
            root,
            Some(snapshot.manifest.generation),
            files,
            scan.fingerprint,
            false,
        )
    }

    pub fn resume(root: &Path) -> Result<ResumeReport, MemoryError> {
        let snapshot = Self::load(root)?;
        let audit = Self::check(root)?;
        Ok(ResumeReport {
            summary: resume_summary(&snapshot),
            issues: audit.issues,
        })
    }

    pub fn check(root: &Path) -> Result<AuditReport, MemoryError> {
        let root = validate_project_root(root)?;
        require_initialized(&root)?;
        let mut issues = Vec::new();
        for relative in REQUIRED_FILES {
            let full = checked_path(&root, &format!(".mt-dna/{relative}"))?;
            if !full.is_file() {
                issues.push(issue(
                    AuditSeverity::Error,
                    "missing_file",
                    relative,
                    "Zorunlu hafıza dosyası eksik.",
                ));
            }
        }

        let project = audit_json::<ProjectMetadata>(&root, "project.json", &mut issues);
        let standards = audit_json::<MemoryStandards>(&root, "standards.json", &mut issues);
        let manifest = audit_json::<MemoryManifest>(&root, "manifest.json", &mut issues);

        if let Some(project) = &project {
            if let Err(error) = SchemaValidator::project(project) {
                issues.push(issue(
                    AuditSeverity::Error,
                    "project_schema",
                    "project.json",
                    &error.to_string(),
                ));
            }
        }
        if let (Some(project), Some(standards)) = (&project, &standards) {
            if let Err(error) = SchemaValidator::standards(standards, &project.language) {
                issues.push(issue(
                    AuditSeverity::Error,
                    "standards_schema",
                    "standards.json",
                    &error.to_string(),
                ));
            }
        }
        if let Some(manifest) = &manifest {
            if let Err(error) = SchemaValidator::manifest(manifest) {
                issues.push(issue(
                    AuditSeverity::Error,
                    "manifest_schema",
                    "manifest.json",
                    &error.to_string(),
                ));
            }
            for entry in &manifest.files {
                let path = checked_path(&root, &format!(".mt-dna/{}", entry.path))?;
                if !path.is_file() {
                    issues.push(issue(
                        AuditSeverity::Error,
                        "missing_managed_file",
                        &entry.path,
                        "Manifestteki yönetilen dosya bulunamadı.",
                    ));
                    continue;
                }
                let bytes = read_bytes(&path)?;
                if content_hash(&bytes) != entry.hash {
                    issues.push(issue(
                        AuditSeverity::Error,
                        "hash_mismatch",
                        &entry.path,
                        "Dosya hash'i manifest ile eşleşmiyor.",
                    ));
                }
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    if SecretRedactor::contains_suspected_secret(text) {
                        issues.push(issue(
                            AuditSeverity::Error,
                            "suspected_secret",
                            &entry.path,
                            "Şüpheli gizli bilgi bulundu; otomatik güvenlik garantisi değildir.",
                        ));
                    }
                    audit_links(&root, &entry.path, text, &mut issues)?;
                }
            }
            let current = ProjectScanner::scan(&root)?;
            if current.fingerprint != manifest.source_fingerprint {
                issues.push(issue(
                    AuditSeverity::Warning,
                    "stale_source",
                    "manifest.json",
                    "Kaynak yapısı hafıza güncellemesinden sonra değişmiş olabilir.",
                ));
            }
        }
        Ok(AuditReport { issues })
    }

    pub fn plan_repair(root: &Path) -> Result<MemoryPlan, MemoryError> {
        let root = validate_project_root(root)?;
        require_initialized(&root)?;
        let project_path = checked_path(&root, ".mt-dna/project.json")?;
        let mut project_value: serde_json::Value = read_json(&project_path)?;
        migrate_schema_value(&mut project_value)?;
        let project: ProjectMetadata =
            serde_json::from_value(project_value).map_err(|source| MemoryError::Json {
                path: project_path.clone(),
                source,
            })?;
        SchemaValidator::project(&project)?;

        let standards_path = checked_path(&root, ".mt-dna/standards.json")?;
        let mut standards_value: serde_json::Value = read_json(&standards_path)?;
        migrate_schema_value(&mut standards_value)?;
        let standards: MemoryStandards =
            serde_json::from_value(standards_value).map_err(|source| MemoryError::Json {
                path: standards_path.clone(),
                source,
            })?;
        SchemaValidator::standards(&standards, &project.language)?;

        let old_manifest =
            read_json::<MemoryManifest>(&checked_path(&root, ".mt-dna/manifest.json")?).ok();
        let scan = ProjectScanner::scan(&root)?;
        let mut files = BTreeMap::new();
        insert_json(&mut files, "project.json", &project)?;
        insert_json(&mut files, "standards.json", &standards)?;
        for (relative, fallback) in [
            ("architecture.md", architecture_document(&project, &scan)),
            ("decisions.md", initial_decisions()),
            ("progress.md", initial_progress()),
            (
                "handoff.md",
                "# Provider aktarımı\n\nHenüz aktarım hazırlanmadı.\n".to_owned(),
            ),
        ] {
            let path = checked_path(&root, &format!(".mt-dna/{relative}"))?;
            let value = if path.is_file() {
                SecretRedactor::redact(&read_text(&path)?)
            } else {
                fallback
            };
            insert_text(&mut files, relative, value);
        }
        if let Some(manifest) = &old_manifest {
            for entry in &manifest.files {
                if entry.path.starts_with("reports/") && safe_memory_relative(&entry.path) {
                    let path = checked_path(&root, &format!(".mt-dna/{}", entry.path))?;
                    if path.is_file() {
                        insert_text(
                            &mut files,
                            &entry.path,
                            SecretRedactor::redact(&read_text(&path)?),
                        );
                    }
                }
            }
        }
        let generation = old_manifest.as_ref().map(|item| item.generation);
        build_plan(root, generation, files, scan.fingerprint, false)
    }

    pub fn apply(plan: &MemoryPlan) -> Result<ApplyReport, MemoryError> {
        let changed = plan
            .changes
            .iter()
            .filter(|change| change.status != ChangeStatus::Unchanged)
            .count();
        let unchanged = plan.changes.len() - changed;
        if changed == 0 {
            return Ok(ApplyReport {
                changed,
                unchanged,
                backup_id: None,
            });
        }

        if plan.initialize {
            ConflictDetector::verify(plan)?;
            let memory_root = checked_path(&plan.project_root, MEMORY_DIRECTORY)?;
            fs::create_dir(&memory_root).map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    MemoryError::AlreadyInitialized(memory_root.clone())
                } else {
                    MemoryError::Io {
                        operation: "Hafıza klasörü oluşturma",
                        path: memory_root.clone(),
                        source,
                    }
                }
            })?;
        }

        let _lock = WriteLock::acquire(&plan.project_root)?;
        if !plan.initialize {
            ConflictDetector::verify(plan)?;
        }
        let backup_id = if plan.initialize {
            None
        } else {
            Some(create_backup(plan)?)
        };

        let ordered_outputs = plan
            .outputs
            .iter()
            .filter(|(relative, _)| relative.as_str() != ".mt-dna/manifest.json")
            .chain(
                plan.outputs
                    .iter()
                    .filter(|(relative, _)| relative.as_str() == ".mt-dna/manifest.json"),
            );
        let mut applied = Vec::new();
        for (relative, contents) in ordered_outputs {
            if plan.changes.iter().any(|change| {
                change.relative_path == *relative && change.status != ChangeStatus::Unchanged
            }) {
                let target = checked_path(&plan.project_root, relative)?;
                if let Err(error) = atomic_write(&plan.project_root, &target, contents) {
                    if let Some(backup) = backup_id.as_deref() {
                        restore_applied(plan, backup, &applied);
                    }
                    return Err(error);
                }
                applied.push(relative.clone());
            }
        }
        Ok(ApplyReport {
            changed,
            unchanged,
            backup_id,
        })
    }
}

pub fn format_memory_plan(plan: &MemoryPlan) -> String {
    let mut output = String::from("Proje hafızası değişiklik planı:\n");
    for change in &plan.changes {
        let status = match change.status {
            ChangeStatus::Create => "YENİ",
            ChangeStatus::Update => "GÜNCELLE",
            ChangeStatus::Unchanged => "AYNI",
        };
        output.push_str(&format!("\n[{status}] {}\n", change.relative_path));
        if change.status != ChangeStatus::Unchanged {
            output.push_str(&change.diff);
        }
    }
    output
}

pub(crate) fn build_plan(
    root: PathBuf,
    previous_generation: Option<u64>,
    files: BTreeMap<String, Vec<u8>>,
    source_fingerprint: String,
    initialize: bool,
) -> Result<MemoryPlan, MemoryError> {
    let timestamp = utc_now();
    let generation = previous_generation.unwrap_or(0) + 1;
    let managed = files
        .iter()
        .map(|(path, contents)| ManagedFile {
            path: path.clone(),
            schema_version: MEMORY_SCHEMA_VERSION.to_owned(),
            hash: content_hash(contents),
        })
        .collect();
    let manifest = MemoryManifest {
        schema_version: MEMORY_SCHEMA_VERSION.to_owned(),
        generation,
        updated_at: timestamp,
        source_fingerprint,
        files: managed,
    };
    SchemaValidator::manifest(&manifest)?;
    let mut full_outputs = BTreeMap::new();
    for (path, contents) in files {
        full_outputs.insert(format!(".mt-dna/{path}"), contents);
    }
    full_outputs.insert(".mt-dna/manifest.json".to_owned(), json_bytes(&manifest)?);

    let mut expected_hashes = BTreeMap::new();
    let mut changes = Vec::new();
    for (relative, new_contents) in &full_outputs {
        let target = checked_path(&root, relative)?;
        let old_contents = if target.is_file() {
            Some(read_bytes(&target)?)
        } else {
            None
        };
        let expected = old_contents.as_deref().map(content_hash);
        expected_hashes.insert(relative.clone(), expected);
        let status = match old_contents.as_deref() {
            Some(old) if old == new_contents.as_slice() => ChangeStatus::Unchanged,
            Some(_) => ChangeStatus::Update,
            None => ChangeStatus::Create,
        };
        changes.push(MemoryChange {
            relative_path: relative.clone(),
            status,
            diff: make_diff(old_contents.as_deref(), new_contents),
        });
    }
    Ok(MemoryPlan {
        project_root: root,
        changes,
        expected_generation: previous_generation,
        expected_hashes,
        outputs: full_outputs,
        initialize,
    })
}

pub(crate) fn snapshot_files(
    root: &Path,
    snapshot: &MemorySnapshot,
) -> Result<BTreeMap<String, Vec<u8>>, MemoryError> {
    let mut files = BTreeMap::new();
    insert_json(&mut files, "project.json", &snapshot.project)?;
    insert_json(&mut files, "standards.json", &snapshot.standards)?;
    insert_text(&mut files, "architecture.md", snapshot.architecture.clone());
    insert_text(&mut files, "decisions.md", snapshot.decisions.clone());
    insert_text(&mut files, "progress.md", snapshot.progress.clone());
    insert_text(&mut files, "handoff.md", snapshot.handoff.clone());
    for entry in &snapshot.manifest.files {
        if (entry.path.starts_with("reports/")
            || entry.path.starts_with("plans/")
            || entry.path.starts_with("baselines/"))
            && safe_memory_relative(&entry.path)
        {
            let path = checked_path(root, &format!(".mt-dna/{}", entry.path))?;
            if path.is_file() {
                files.insert(entry.path.clone(), read_bytes(&path)?);
            }
        }
    }
    Ok(files)
}

fn create_backup(plan: &MemoryPlan) -> Result<String, MemoryError> {
    let id = new_id();
    let backup_root = checked_path(&plan.project_root, &format!(".mt-dna/backups/{id}/files"))?;
    create_directories(&plan.project_root, &backup_root)?;
    for change in &plan.changes {
        if change.status == ChangeStatus::Update {
            let source = checked_path(&plan.project_root, &change.relative_path)?;
            let relative_inside = change
                .relative_path
                .strip_prefix(".mt-dna/")
                .unwrap_or(&change.relative_path);
            let target = backup_root.join(relative_inside);
            let contents = read_bytes(&source)?;
            write_new(&plan.project_root, &target, &contents)?;
        }
    }
    Ok(id)
}

fn restore_applied(plan: &MemoryPlan, backup_id: &str, applied: &[String]) {
    for relative in applied.iter().rev() {
        let Ok(target) = checked_path(&plan.project_root, relative) else {
            continue;
        };
        if plan
            .expected_hashes
            .get(relative)
            .is_some_and(Option::is_some)
        {
            let relative_inside = relative.strip_prefix(".mt-dna/").unwrap_or(relative);
            let backup = plan
                .project_root
                .join(format!(".mt-dna/backups/{backup_id}/files"))
                .join(relative_inside);
            if let Ok(contents) = fs::read(backup) {
                let _ = atomic_write(&plan.project_root, &target, &contents);
            }
        } else if target.is_file() {
            let _ = fs::remove_file(target);
        }
    }
}

struct WriteLock {
    path: PathBuf,
}

impl WriteLock {
    fn acquire(root: &Path) -> Result<Self, MemoryError> {
        let path = checked_path(root, ".mt-dna/.write.lock")?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    MemoryError::Locked(path.clone())
                } else {
                    MemoryError::Io {
                        operation: "Yazma kilidi oluşturma",
                        path: path.clone(),
                        source,
                    }
                }
            })?;
        file.write_all(format!("pid={}\n", std::process::id()).as_bytes())
            .map_err(|source| MemoryError::Io {
                operation: "Yazma kilidi kaydetme",
                path: path.clone(),
                source,
            })?;
        Ok(Self { path })
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn require_initialized(root: &Path) -> Result<(), MemoryError> {
    let memory_root = checked_path(root, MEMORY_DIRECTORY)?;
    if !memory_root.is_dir() {
        return Err(MemoryError::NotInitialized(memory_root));
    }
    Ok(())
}

fn audit_json<T: serde::de::DeserializeOwned>(
    root: &Path,
    relative: &str,
    issues: &mut Vec<AuditIssue>,
) -> Option<T> {
    let path = checked_path(root, &format!(".mt-dna/{relative}")).ok()?;
    if !path.is_file() {
        return None;
    }
    match read_json(&path) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(issue(
                AuditSeverity::Error,
                "invalid_json",
                relative,
                &error.to_string(),
            ));
            None
        }
    }
}

fn audit_links(
    root: &Path,
    source_relative: &str,
    text: &str,
    issues: &mut Vec<AuditIssue>,
) -> Result<(), MemoryError> {
    if !source_relative.ends_with(".md") {
        return Ok(());
    }
    for fragment in text.split("](").skip(1) {
        let Some(end) = fragment.find(')') else {
            continue;
        };
        let link = &fragment[..end];
        if link.is_empty()
            || link.starts_with('#')
            || link.contains("://")
            || link.starts_with("mailto:")
        {
            continue;
        }
        let source_dir = Path::new(source_relative)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let joined = source_dir.join(link);
        let normalized = joined.to_string_lossy().replace('\\', "/");
        if !safe_memory_relative(&normalized) {
            issues.push(issue(
                AuditSeverity::Error,
                "unsafe_link",
                source_relative,
                "Markdown bağlantısı hafıza klasörü dışına çıkıyor.",
            ));
            continue;
        }
        let target = checked_path(root, &format!(".mt-dna/{normalized}"))?;
        if !target.exists() {
            issues.push(issue(
                AuditSeverity::Warning,
                "broken_link",
                source_relative,
                &format!("Yerel bağlantı bulunamadı: {link}"),
            ));
        }
    }
    Ok(())
}

fn migrate_schema_value(value: &mut serde_json::Value) -> Result<(), MemoryError> {
    let Some(object) = value.as_object_mut() else {
        return Err(MemoryError::InvalidField {
            field: "schema_version",
            message: "JSON nesnesi bekleniyor".to_owned(),
        });
    };
    match object
        .get("schema_version")
        .and_then(|value| value.as_str())
    {
        Some(MEMORY_SCHEMA_VERSION) => Ok(()),
        Some("0.2.0") => {
            object.insert(
                "schema_version".to_owned(),
                serde_json::Value::String(MEMORY_SCHEMA_VERSION.to_owned()),
            );
            Ok(())
        }
        Some(found) => Err(MemoryError::UnsupportedSchema {
            found: found.to_owned(),
            expected: MEMORY_SCHEMA_VERSION.to_owned(),
        }),
        None => Err(MemoryError::InvalidField {
            field: "schema_version",
            message: "eksik".to_owned(),
        }),
    }
}

fn redact_values(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| SecretRedactor::redact(&value))
        .collect()
}

fn insert_json<T: serde::Serialize>(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &T,
) -> Result<(), MemoryError> {
    files.insert(path.to_owned(), json_bytes(value)?);
    Ok(())
}

fn insert_text(files: &mut BTreeMap<String, Vec<u8>>, path: &str, value: String) {
    files.insert(path.to_owned(), value.into_bytes());
}

fn json_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, MemoryError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| MemoryError::Json {
        path: PathBuf::from("<memory>"),
        source,
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, MemoryError> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|source| MemoryError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read_text(path: &Path) -> Result<String, MemoryError> {
    fs::read_to_string(path).map_err(|source| MemoryError::Io {
        operation: "Hafıza dosyasını okuma",
        path: path.to_path_buf(),
        source,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, MemoryError> {
    fs::read(path).map_err(|source| MemoryError::Io {
        operation: "Hafıza dosyasını okuma",
        path: path.to_path_buf(),
        source,
    })
}

fn create_directories(root: &Path, path: &Path) -> Result<(), MemoryError> {
    ensure_no_links(root, path)?;
    fs::create_dir_all(path).map_err(|source| MemoryError::Io {
        operation: "Hafıza klasörü oluşturma",
        path: path.to_path_buf(),
        source,
    })
}

fn write_new(root: &Path, path: &Path, contents: &[u8]) -> Result<(), MemoryError> {
    let parent = path
        .parent()
        .ok_or_else(|| MemoryError::UnsafePath(path.display().to_string()))?;
    create_directories(root, parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| MemoryError::Io {
            operation: "Yeni hafıza dosyası oluşturma",
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(contents).map_err(|source| MemoryError::Io {
        operation: "Hafıza dosyasına yazma",
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| MemoryError::Io {
        operation: "Hafıza dosyasını diske eşitleme",
        path: path.to_path_buf(),
        source,
    })
}

fn atomic_write(root: &Path, target: &Path, contents: &[u8]) -> Result<(), MemoryError> {
    ensure_no_links(root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| MemoryError::UnsafePath(target.display().to_string()))?;
    create_directories(root, parent)?;
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MemoryError::UnsafePath(target.display().to_string()))?;
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".{name}.mt-dna-{}-{counter}.tmp",
        std::process::id()
    ));
    write_new(root, &staging, contents)?;
    let result = replace_path(&staging, target).map_err(|source| MemoryError::Io {
        operation: "Atomik hafıza dosyası değiştirme",
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

fn make_diff(old: Option<&[u8]>, new: &[u8]) -> String {
    let mut output = String::new();
    match old {
        Some(old) => {
            output.push_str("--- mevcut\n+++ üretilecek\n");
            for line in String::from_utf8_lossy(old).lines().take(80) {
                output.push_str(&format!("-{line}\n"));
            }
        }
        None => output.push_str("--- /dev/null\n+++ üretilecek\n"),
    }
    for line in String::from_utf8_lossy(new).lines().take(80) {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn safe_memory_relative(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
        && !value.starts_with("backups/")
}

fn supported_language(value: &str) -> bool {
    matches!(
        value,
        "rust" | "csharp" | "python" | "cpp" | "go" | "java" | "typescript"
    )
}

fn validate_text(
    field: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), MemoryError> {
    let count = value.trim().chars().count();
    if !(minimum..=maximum).contains(&count) || SecretRedactor::contains_suspected_secret(value) {
        return Err(MemoryError::InvalidField {
            field,
            message: format!("{minimum}-{maximum} karakter olmalı ve gizli bilgi içermemelidir"),
        });
    }
    Ok(())
}

fn looks_like_utc_timestamp(value: &str) -> bool {
    value.len() == 20
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
        && value.ends_with('Z')
}

fn issue(severity: AuditSeverity, code: &str, path: &str, message: &str) -> AuditIssue {
    AuditIssue {
        severity,
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

fn latest_quality_summary(
    root: &Path,
    manifest: &MemoryManifest,
) -> Result<Option<String>, MemoryError> {
    let latest = manifest
        .files
        .iter()
        .filter(|entry| entry.path.starts_with("reports/quality/") && entry.path.ends_with(".json"))
        .max_by(|left, right| left.path.cmp(&right.path));
    let Some(entry) = latest else {
        return Ok(None);
    };
    let path = checked_path(root, &format!(".mt-dna/{}", entry.path))?;
    let value: serde_json::Value = read_json(&path)?;
    let timestamp = value
        .get("timestamp_utc")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("zaman kaydı yok");
    let gate = value
        .get("release_gate")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("UNAVAILABLE");
    Ok(Some(format!(
        "- Rapor: `{}`\n- Zaman: {}\n- Release gate: {}\n- Ayrıntılar kaynak JSON/Markdown raporundadır; PASS olmayan kontrolleri tamamlanmış sayma.",
        entry.path, timestamp, gate
    )))
}
