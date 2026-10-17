use crate::CliError;
use dna_core::{
    ApprovalRecord, Confidence, FindingSource, FindingStatus, FixRisk, LineRange,
    OptimizationFinding, OptimizationFixRecord, OptimizationMode, OptimizationReport,
    OptimizationSeverity, OPTIMIZATION_SCHEMA_VERSION,
};
use dna_memory::{content_hash, utc_timestamp, MemoryStore, SecretRedactor};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_FILES: usize = 1_000;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const TRANSACTION_SCHEMA: u32 = 1;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct FixPlan {
    pub project_root: PathBuf,
    pub finding: OptimizationFinding,
    pub fix_id: String,
    pub relative_path: String,
    pub before_hash: String,
    pub after_hash: String,
    pub patch_digest: String,
    pub unified_diff: String,
    before: Vec<u8>,
    after: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ApplyOutcome {
    pub record: OptimizationFixRecord,
    pub transaction_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionManifest {
    schema_version: u32,
    fix_id: String,
    relative_path: String,
    before_hash: String,
    after_hash: String,
    patch_digest: String,
    status: TransactionStatus,
    applied_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TransactionStatus {
    Prepared,
    Applied,
    RolledBack,
}

pub struct OptimizationEngine;

impl OptimizationEngine {
    pub fn analyze(
        root: &Path,
        project: &str,
        language: &str,
        mode: OptimizationMode,
    ) -> Result<OptimizationReport, CliError> {
        let root = validate_root(root)?;
        let mut pending = vec![PathBuf::new()];
        let mut findings = Vec::new();
        let mut scanned = 0usize;
        let mut truncated = false;
        while let Some(relative_dir) = pending.pop() {
            let directory = root.join(&relative_dir);
            let mut entries = fs::read_dir(&directory)
                .map_err(|source| io_error("Optimizasyon dizinini okuma", &directory, source))?
                .flatten()
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let name = entry.file_name().to_string_lossy().into_owned();
                if ignored(&name) {
                    continue;
                }
                let relative = relative_dir.join(&name);
                let metadata = entry.metadata().map_err(|source| {
                    io_error("Optimizasyon girdisini okuma", &entry.path(), source)
                })?;
                if metadata.is_dir() {
                    pending.push(relative);
                    continue;
                }
                if scanned >= MAX_FILES {
                    truncated = true;
                    break;
                }
                if metadata.len() > MAX_FILE_BYTES || !source_candidate(&relative) {
                    continue;
                }
                scanned += 1;
                let contents = fs::read_to_string(entry.path()).unwrap_or_default();
                inspect_file(&relative, &contents, &mut findings);
            }
            if truncated {
                break;
            }
        }
        findings.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.line_range.start.cmp(&right.line_range.start))
                .then(left.finding_id.cmp(&right.finding_id))
        });
        let report = OptimizationReport {
            schema_version: OPTIMIZATION_SCHEMA_VERSION.to_owned(),
            project: project.to_owned(),
            language: language.to_owned(),
            mode,
            timestamp_utc: utc_timestamp(),
            scanned_files: scanned,
            scan_truncated: truncated,
            findings,
            fix: None,
            notes: vec![
                "Analiz sınırlı, talep üzerine ve salt-okunur çalıştı; daemon veya polling başlatılmadı."
                    .to_owned(),
                "Statik performans sezgileri ölçülmüş kazanç değildir; APPLY öncesi ve sonrası karşılaştırılabilir ölçüm gerekir."
                    .to_owned(),
            ],
        };
        report.validate()?;
        Ok(report)
    }

    pub fn plan_fix(
        root: &Path,
        report: &OptimizationReport,
        finding_id: &str,
    ) -> Result<FixPlan, CliError> {
        let root = validate_root(root)?;
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| CliError::FindingNotFound(finding_id.to_owned()))?
            .clone();
        if !finding.auto_fixable || finding.risk != FixRisk::Low {
            return Err(CliError::FindingNotAutoFixable(finding_id.to_owned()));
        }
        let target = checked_target(&root, &finding.file)?;
        let before = fs::read(&target)
            .map_err(|source| io_error("Auto-Fix hedefini okuma", &target, source))?;
        if before.len() as u64 > MAX_FILE_BYTES {
            return Err(CliError::InvalidOptimizationArtifact(
                "Auto-Fix hedefi 1 MiB sınırını aşıyor".to_owned(),
            ));
        }
        let source = std::str::from_utf8(&before)
            .map_err(|_| CliError::InvalidOptimizationArtifact("hedef UTF-8 değil".to_owned()))?;
        if SecretRedactor::contains_suspected_secret(source) {
            return Err(CliError::InvalidOptimizationArtifact(
                "şüpheli gizli bilgi içeren dosya otomatik yedeklenmez veya düzeltilmez".to_owned(),
            ));
        }
        let after = strip_trailing_whitespace(source).into_bytes();
        if before == after {
            return Err(CliError::TargetChanged(target));
        }
        let before_hash = content_hash(&before);
        let after_hash = content_hash(&after);
        let patch_digest =
            content_hash(format!("{}\0{}\0{}", finding.file, before_hash, after_hash).as_bytes());
        let fix_id = format!("fix-{}", patch_digest.trim_start_matches("fnv1a64:"));
        let unified_diff = unified_diff(&finding.file, source, &String::from_utf8_lossy(&after));
        Ok(FixPlan {
            project_root: root,
            finding,
            fix_id,
            relative_path: report
                .findings
                .iter()
                .find(|item| item.finding_id == finding_id)
                .map(|item| item.file.clone())
                .unwrap_or_default(),
            before_hash,
            after_hash,
            patch_digest,
            unified_diff,
            before,
            after,
        })
    }

    pub fn apply(plan: &FixPlan, approved: bool) -> Result<ApplyOutcome, CliError> {
        if !approved {
            return Err(CliError::OptimizationConsentRequired);
        }
        MemoryStore::load(&plan.project_root)?;
        let target = checked_target(&plan.project_root, &plan.relative_path)?;
        let current = fs::read(&target)
            .map_err(|source| io_error("Auto-Fix hedefini yeniden okuma", &target, source))?;
        if content_hash(&current) != plan.before_hash {
            return Err(CliError::TargetChanged(target));
        }

        let transaction_root = checked_target(
            &plan.project_root,
            &format!(".mt-dna/transactions/{}", plan.fix_id),
        )?;
        if transaction_root.exists() {
            return Err(CliError::OptimizationTransactionExists(plan.fix_id.clone()));
        }
        create_directories(&plan.project_root, &transaction_root.join("files"))?;
        let backup = transaction_root.join("files").join(&plan.relative_path);
        write_new_synced(&plan.project_root, &backup, &plan.before)?;
        let applied_at = utc_timestamp();
        let mut manifest = TransactionManifest {
            schema_version: TRANSACTION_SCHEMA,
            fix_id: plan.fix_id.clone(),
            relative_path: plan.relative_path.clone(),
            before_hash: plan.before_hash.clone(),
            after_hash: plan.after_hash.clone(),
            patch_digest: plan.patch_digest.clone(),
            status: TransactionStatus::Prepared,
            applied_at: applied_at.clone(),
        };
        write_new_synced(
            &plan.project_root,
            &transaction_root.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?,
        )?;
        atomic_write(&plan.project_root, &target, &plan.after)?;
        manifest.status = TransactionStatus::Applied;
        atomic_write(
            &plan.project_root,
            &transaction_root.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?,
        )?;

        let record = OptimizationFixRecord {
            schema_version: OPTIMIZATION_SCHEMA_VERSION.to_owned(),
            fix_id: plan.fix_id.clone(),
            finding_ids: vec![plan.finding.finding_id.clone()],
            target_files: vec![plan.relative_path.clone()],
            patch_digest: plan.patch_digest.clone(),
            before_hashes: vec![plan.before_hash.clone()],
            after_hashes: vec![plan.after_hash.clone()],
            approval_record: ApprovalRecord {
                explicitly_confirmed: true,
                approved_scope: vec![plan.relative_path.clone()],
                recorded_at: applied_at.clone(),
            },
            applied_at,
            tests_run: Vec::new(),
            gate_results: Vec::new(),
            rollback_artifact: format!(".mt-dna/transactions/{}", plan.fix_id),
            status: FindingStatus::Applied,
        };
        record.validate()?;
        Ok(ApplyOutcome {
            record,
            transaction_path: transaction_root,
        })
    }

    pub fn rollback(root: &Path, fix_id: &str, approved: bool) -> Result<ApplyOutcome, CliError> {
        if !approved {
            return Err(CliError::OptimizationConsentRequired);
        }
        if !safe_id(fix_id) {
            return Err(CliError::InvalidOptimizationArtifact("fix-id".to_owned()));
        }
        let root = validate_root(root)?;
        MemoryStore::load(&root)?;
        let transaction_root = checked_target(&root, &format!(".mt-dna/transactions/{fix_id}"))?;
        let manifest_path = transaction_root.join("manifest.json");
        let mut manifest: TransactionManifest =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|source| {
                io_error("Rollback manifestini okuma", &manifest_path, source)
            })?)
            .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?;
        if manifest.schema_version != TRANSACTION_SCHEMA
            || manifest.fix_id != fix_id
            || !matches!(
                manifest.status,
                TransactionStatus::Applied | TransactionStatus::Prepared
            )
        {
            return Err(CliError::InvalidOptimizationArtifact(
                "işlem durumu rollback için uygun değil".to_owned(),
            ));
        }
        let target = checked_target(&root, &manifest.relative_path)?;
        let current = fs::read(&target)
            .map_err(|source| io_error("Rollback hedefini okuma", &target, source))?;
        if content_hash(&current) != manifest.after_hash {
            return Err(CliError::OptimizationRollbackConflict(target));
        }
        let backup = transaction_root.join("files").join(&manifest.relative_path);
        let before = fs::read(&backup)
            .map_err(|source| io_error("Rollback yedeğini okuma", &backup, source))?;
        if content_hash(&before) != manifest.before_hash {
            return Err(CliError::InvalidOptimizationArtifact(
                "rollback yedeği hash doğrulamasını geçemedi".to_owned(),
            ));
        }
        atomic_write(&root, &target, &before)?;
        manifest.status = TransactionStatus::RolledBack;
        atomic_write(
            &root,
            &manifest_path,
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| CliError::InvalidOptimizationArtifact(error.to_string()))?,
        )?;
        let record = OptimizationFixRecord {
            schema_version: OPTIMIZATION_SCHEMA_VERSION.to_owned(),
            fix_id: fix_id.to_owned(),
            finding_ids: vec!["rollback".to_owned()],
            target_files: vec![manifest.relative_path.clone()],
            patch_digest: manifest.patch_digest.clone(),
            before_hashes: vec![manifest.before_hash.clone()],
            after_hashes: vec![manifest.after_hash.clone()],
            approval_record: ApprovalRecord {
                explicitly_confirmed: true,
                approved_scope: vec![manifest.relative_path.clone()],
                recorded_at: utc_timestamp(),
            },
            applied_at: manifest.applied_at.clone(),
            tests_run: Vec::new(),
            gate_results: Vec::new(),
            rollback_artifact: format!(".mt-dna/transactions/{fix_id}"),
            status: FindingStatus::RolledBack,
        };
        record.validate()?;
        Ok(ApplyOutcome {
            record,
            transaction_path: transaction_root,
        })
    }

    pub fn markdown(report: &OptimizationReport, preview: Option<&FixPlan>) -> String {
        let mut output = format!(
            "# MT Developer DNA optimizasyon raporu\n\n- Şema: {}\n- Proje: {}\n- Dil: {}\n- Mod: {:?}\n- Zaman: {}\n- Taranan dosya: {}\n- Kısmi tarama: {}\n- Bulgu: {}\n\n",
            report.schema_version,
            report.project,
            report.language,
            report.mode,
            report.timestamp_utc,
            report.scanned_files,
            if report.scan_truncated { "EVET" } else { "HAYIR" },
            report.findings.len()
        );
        for finding in &report.findings {
            output.push_str(&format!(
                "## {} — {:?}\n\n- Kaynak: {:?}\n- Dosya: {}:{}-{}\n- Güven: {:?}\n- Risk: {:?}\n- Auto-Fix: {}\n- Kanıt: {}\n- Öneri: {}\n\n",
                finding.finding_id,
                finding.severity,
                finding.source,
                finding.file,
                finding.line_range.start,
                finding.line_range.end,
                finding.confidence,
                finding.risk,
                if finding.auto_fixable { "EVET" } else { "HAYIR" },
                finding.evidence,
                finding.proposed_fix
            ));
        }
        if let Some(plan) = preview {
            output.push_str(&format!(
                "## Auto-Fix önizlemesi\n\n- Fix ID: {}\n- Hedef: {}\n- Patch digest: {}\n- Risk: {:?}\n- Rollback: `.mt-dna/transactions/{}`\n- Test planı: V0.5 quality gate uygulama öncesi ve sonrası\n\n```diff\n{}```\n\n",
                plan.fix_id,
                plan.relative_path,
                plan.patch_digest,
                plan.finding.risk,
                plan.fix_id,
                plan.unified_diff
            ));
        }
        if let Some(fix) = &report.fix {
            output.push_str(&format!(
                "## İşlem kaydı\n\n- Fix ID: {}\n- Durum: {:?}\n- Hedefler: {}\n- Patch digest: {}\n- Rollback artifact: `{}`\n",
                fix.fix_id,
                fix.status,
                fix.target_files.join(", "),
                fix.patch_digest,
                fix.rollback_artifact
            ));
            for gate in &fix.gate_results {
                output.push_str(&format!(
                    "- Gate {}: {} ({})\n",
                    gate.phase, gate.gate, gate.report_timestamp
                ));
            }
            output.push('\n');
        }
        for note in &report.notes {
            output.push_str(&format!("> {}\n\n", note));
        }
        SecretRedactor::redact(&output)
    }
}

fn inspect_file(path: &Path, contents: &str, findings: &mut Vec<OptimizationFinding>) {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let trailing = contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            (line.ends_with(' ') || line.ends_with('\t')).then_some(index + 1)
        })
        .collect::<Vec<_>>();
    if let (Some(start), Some(end)) = (trailing.first(), trailing.last()) {
        findings.push(make_finding(
            &normalized,
            *start,
            *end,
            "trailing-whitespace",
            FindingSource::Lint,
            OptimizationSeverity::Low,
            Confidence::High,
            format!("{} satırda satır sonu boşluğu doğrulandı.", trailing.len()),
            "Dosyayı aynı içerik ve satır sonlarıyla açıp trailing whitespace satırlarını kontrol et."
                .to_owned(),
            "Yalnızca satır sonundaki space/tab karakterlerini kaldır.".to_owned(),
            FixRisk::Low,
            true,
        ));
    }

    let clone_pattern = [".clone()", ".clone()"].concat();
    let polling_patterns = [
        ["from_millis", "(1)"].concat(),
        ["from_millis", "(5)"].concat(),
        ["while ", "true"].concat(),
    ];
    for (index, line) in contents.lines().enumerate() {
        if line.contains(&clone_pattern) {
            findings.push(make_finding(
                &normalized,
                index + 1,
                index + 1,
                "repeated-clone",
                FindingSource::Performance,
                OptimizationSeverity::Medium,
                Confidence::Medium,
                "Aynı ifade üzerinde ardışık clone çağrısı görüldü.".to_owned(),
                "Profiler/benchmark ile tahsis ve kopyalama maliyetini aynı iş yükünde ölç."
                    .to_owned(),
                "Sahiplik ve yaşam süresi doğrulandıktan sonra gereksiz kopyayı kaldırmayı değerlendir."
                    .to_owned(),
                FixRisk::Medium,
                false,
            ));
        }
        if polling_patterns
            .iter()
            .any(|pattern| line.contains(pattern))
        {
            findings.push(make_finding(
                &normalized,
                index + 1,
                index + 1,
                "polling-candidate",
                FindingSource::Performance,
                OptimizationSeverity::Medium,
                Confidence::Low,
                "Yüksek frekanslı bekleme veya açık uçlu döngü adayı görüldü.".to_owned(),
                "Boşta CPU ölçümü ve çağrı sıklığını profiler ile doğrula.".to_owned(),
                "Event-driven bildirim veya daha seyrek, sınırlı bekleme uygunluğunu incele."
                    .to_owned(),
                FixRisk::High,
                false,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn make_finding(
    file: &str,
    start: usize,
    end: usize,
    kind: &str,
    source: FindingSource,
    severity: OptimizationSeverity,
    confidence: Confidence,
    evidence: String,
    reproduction: String,
    proposed_fix: String,
    risk: FixRisk,
    auto_fixable: bool,
) -> OptimizationFinding {
    let digest = content_hash(format!("{file}\0{start}\0{end}\0{kind}").as_bytes());
    OptimizationFinding {
        finding_id: format!("finding-{}", digest.trim_start_matches("fnv1a64:")),
        source,
        severity,
        confidence,
        file: file.to_owned(),
        line_range: LineRange { start, end },
        evidence,
        reproduction,
        proposed_fix,
        risk,
        verification_steps: vec![
            "İlgili dil testlerini çalıştır.".to_owned(),
            "V0.5 quality gate sonucunu önce/sonra karşılaştır.".to_owned(),
        ],
        status: if auto_fixable {
            FindingStatus::Open
        } else {
            FindingStatus::NotFixable
        },
        auto_fixable,
    }
}

fn strip_trailing_whitespace(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for segment in source.split_inclusive('\n') {
        let (body, newline) = segment
            .strip_suffix('\n')
            .map(|body| (body, "\n"))
            .unwrap_or((segment, ""));
        let (body, carriage_return) = body
            .strip_suffix('\r')
            .map(|body| (body, "\r"))
            .unwrap_or((body, ""));
        output.push_str(body.trim_end_matches([' ', '\t']));
        output.push_str(carriage_return);
        output.push_str(newline);
    }
    output
}

fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let mut output = format!("--- a/{path}\n+++ b/{path}\n");
    let old = before.lines().collect::<Vec<_>>();
    let new = after.lines().collect::<Vec<_>>();
    for index in 0..old.len().max(new.len()) {
        match (old.get(index), new.get(index)) {
            (Some(left), Some(right)) if left == right => {}
            (Some(left), Some(right)) => {
                output.push_str(&format!("-{}\n+{}\n", left, right));
            }
            (Some(left), None) => output.push_str(&format!("-{}\n", left)),
            (None, Some(right)) => output.push_str(&format!("+{}\n", right)),
            (None, None) => {}
        }
    }
    output
}

fn ignored(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | ".mt-dna"
            | ".tooling"
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

fn source_candidate(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(
            "rs" | "cs"
                | "py"
                | "cpp"
                | "cc"
                | "cxx"
                | "h"
                | "hpp"
                | "go"
                | "java"
                | "ts"
                | "tsx"
                | "js"
        )
    )
}

fn validate_root(root: &Path) -> Result<PathBuf, CliError> {
    let metadata =
        fs::symlink_metadata(root).map_err(|_| CliError::InvalidProjectRoot(root.to_path_buf()))?;
    if !metadata.is_dir() || is_link(&metadata) {
        return Err(CliError::InvalidProjectRoot(root.to_path_buf()));
    }
    fs::canonicalize(root).map_err(|source| io_error("Proje kökünü çözümleme", root, source))
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
    ensure_no_links(root, &target)?;
    Ok(target)
}

fn ensure_no_links(root: &Path, target: &Path) -> Result<(), CliError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| CliError::UnsafeOutputPath(target.display().to_string()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|source| io_error("Yol bilgisini okuma", &current, source))?;
            if is_link(&metadata) {
                return Err(CliError::SymlinkTarget(current));
            }
        }
    }
    Ok(())
}

fn create_directories(root: &Path, path: &Path) -> Result<(), CliError> {
    ensure_no_links(root, path)?;
    fs::create_dir_all(path).map_err(|source| io_error("Klasör oluşturma", path, source))
}

fn write_new_synced(root: &Path, path: &Path, contents: &[u8]) -> Result<(), CliError> {
    let parent = path
        .parent()
        .ok_or_else(|| CliError::UnsafeOutputPath(path.display().to_string()))?;
    create_directories(root, parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io_error("Yeni dosya oluşturma", path, source))?;
    file.write_all(contents)
        .map_err(|source| io_error("Dosya yazma", path, source))?;
    file.sync_all()
        .map_err(|source| io_error("Dosyayı diske eşitleme", path, source))
}

fn atomic_write(root: &Path, target: &Path, contents: &[u8]) -> Result<(), CliError> {
    ensure_no_links(root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| CliError::UnsafeOutputPath(target.display().to_string()))?;
    create_directories(root, parent)?;
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CliError::UnsafeOutputPath(target.display().to_string()))?;
    let staging = parent.join(format!(
        ".{name}.mt-dna-opt-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    write_new_synced(root, &staging, contents)?;
    let result = replace_path(&staging, target)
        .map_err(|source| io_error("Atomik optimizasyon yazımı", target, source));
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

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
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

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> CliError {
    CliError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dna_memory::{InitRequest, ProviderId};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static ID: AtomicU64 = AtomicU64::new(0);

    fn project(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-optimization")
            .join(format!(
                "{name}-{}-{nonce}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {   \n}\n").unwrap();
        let plan = MemoryStore::plan_initialize(
            &root,
            InitRequest {
                project_id: "optimization-test".to_owned(),
                name: "Optimization test".to_owned(),
                language: "rust".to_owned(),
                project_type: "cli".to_owned(),
                platforms: vec!["windows".to_owned()],
                provider: ProviderId::Codex,
                architecture_rules: vec!["KISS".to_owned()],
                quality_rules: vec!["Test".to_owned()],
                security_rules: vec!["Secret yok".to_owned()],
                performance_rules: vec!["Ölç".to_owned()],
            },
        )
        .unwrap();
        MemoryStore::apply(&plan).unwrap();
        root
    }

    #[test]
    fn analyze_and_preview_are_read_only_and_unapproved_apply_is_rejected() {
        let root = project("read-only");
        let before = fs::read(root.join("src/main.rs")).unwrap();
        let report =
            OptimizationEngine::analyze(&root, "sample", "rust", OptimizationMode::Analyze)
                .unwrap();
        let finding = report
            .findings
            .iter()
            .find(|item| item.auto_fixable)
            .unwrap();
        let plan = OptimizationEngine::plan_fix(&root, &report, &finding.finding_id).unwrap();
        assert_eq!(before, fs::read(root.join("src/main.rs")).unwrap());
        assert!(matches!(
            OptimizationEngine::apply(&plan, false),
            Err(CliError::OptimizationConsentRequired)
        ));
        assert_eq!(before, fs::read(root.join("src/main.rs")).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn apply_hash_guard_and_rollback_preserve_user_changes() {
        let root = project("hash-guard");
        let report =
            OptimizationEngine::analyze(&root, "sample", "rust", OptimizationMode::Analyze)
                .unwrap();
        let finding = report
            .findings
            .iter()
            .find(|item| item.auto_fixable)
            .unwrap();
        let plan = OptimizationEngine::plan_fix(&root, &report, &finding.finding_id).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() { /* user */ }\n").unwrap();
        assert!(matches!(
            OptimizationEngine::apply(&plan, true),
            Err(CliError::TargetChanged(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn small_fix_is_recorded_and_rollback_is_conflict_safe() {
        let root = project("roundtrip");
        let original = fs::read(root.join("src/main.rs")).unwrap();
        let report =
            OptimizationEngine::analyze(&root, "sample", "rust", OptimizationMode::Analyze)
                .unwrap();
        let finding = report
            .findings
            .iter()
            .find(|item| item.auto_fixable)
            .unwrap();
        let plan = OptimizationEngine::plan_fix(&root, &report, &finding.finding_id).unwrap();
        let applied = OptimizationEngine::apply(&plan, true).unwrap();
        assert_eq!(applied.record.status, FindingStatus::Applied);
        assert_eq!(
            fs::read_to_string(root.join("src/main.rs")).unwrap(),
            "fn main() {\n}\n"
        );
        fs::write(root.join("src/main.rs"), "fn main() { /* later */ }\n").unwrap();
        assert!(matches!(
            OptimizationEngine::rollback(&root, &plan.fix_id, true),
            Err(CliError::OptimizationRollbackConflict(_))
        ));
        fs::write(root.join("src/main.rs"), &plan.after).unwrap();
        let rolled_back = OptimizationEngine::rollback(&root, &plan.fix_id, true).unwrap();
        assert_eq!(rolled_back.record.status, FindingStatus::RolledBack);
        assert_eq!(fs::read(root.join("src/main.rs")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_transaction_is_rejected() {
        let root = project("corrupt");
        let transaction = root.join(".mt-dna/transactions/fix-broken");
        fs::create_dir_all(&transaction).unwrap();
        fs::write(transaction.join("manifest.json"), "{broken").unwrap();
        assert!(matches!(
            OptimizationEngine::rollback(&root, "fix-broken", true),
            Err(CliError::InvalidOptimizationArtifact(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unfinished_existing_transaction_stops_without_partial_write() {
        let root = project("partial-stop");
        let original = fs::read(root.join("src/main.rs")).unwrap();
        let report =
            OptimizationEngine::analyze(&root, "sample", "rust", OptimizationMode::Analyze)
                .unwrap();
        let finding = report
            .findings
            .iter()
            .find(|item| item.auto_fixable)
            .unwrap();
        let plan = OptimizationEngine::plan_fix(&root, &report, &finding.finding_id).unwrap();
        fs::create_dir_all(root.join(format!(".mt-dna/transactions/{}", plan.fix_id))).unwrap();
        assert!(matches!(
            OptimizationEngine::apply(&plan, true),
            Err(CliError::OptimizationTransactionExists(_))
        ));
        assert_eq!(fs::read(root.join("src/main.rs")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn suspected_secret_file_is_never_backed_up_or_auto_fixed() {
        let root = project("secret-stop");
        let token = ["sk-proj-1234567890", "abcdefghij"].concat();
        fs::write(
            root.join("src/main.rs"),
            format!("const TOKEN: &str = \"{token}\";   \nfn main() {{}}\n"),
        )
        .unwrap();
        let report =
            OptimizationEngine::analyze(&root, "sample", "rust", OptimizationMode::Analyze)
                .unwrap();
        let finding = report
            .findings
            .iter()
            .find(|item| item.auto_fixable)
            .unwrap();
        assert!(matches!(
            OptimizationEngine::plan_fix(&root, &report, &finding.finding_id),
            Err(CliError::InvalidOptimizationArtifact(_))
        ));
        assert!(!root.join(".mt-dna/transactions").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
