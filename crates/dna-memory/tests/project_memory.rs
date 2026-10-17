use dna_core::DevelopmentPlanner;
use dna_memory::{
    DecisionInput, InitRequest, MemoryBridge, MemoryError, MemoryStore, ProjectAnalyzer,
    ProviderId, UpdateRequest, MEMORY_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_project(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-memory")
        .join(format!("{name}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname='sample'\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    root
}

fn init_request(provider: ProviderId) -> InitRequest {
    InitRequest {
        project_id: "sample-project".to_owned(),
        name: "Örnek Proje".to_owned(),
        language: "rust".to_owned(),
        project_type: "cli".to_owned(),
        platforms: vec!["windows".to_owned()],
        provider,
        architecture_rules: vec!["KISS".to_owned()],
        quality_rules: vec!["Test çalıştır".to_owned()],
        security_rules: vec!["Sır kaydetme".to_owned()],
        performance_rules: vec!["Ölçmeden iddia etme".to_owned()],
    }
}

fn initialize(root: &Path) {
    let plan = MemoryStore::plan_initialize(root, init_request(ProviderId::Codex)).unwrap();
    MemoryStore::apply(&plan).unwrap();
}

#[test]
fn init_creates_schema_and_second_init_preserves_data() {
    let root = temporary_project("init");
    let preview = MemoryStore::plan_initialize(&root, init_request(ProviderId::Codex)).unwrap();
    assert!(!root.join(".mt-dna").exists());
    MemoryStore::apply(&preview).unwrap();
    let original = fs::read(root.join(".mt-dna/project.json")).unwrap();

    let error = MemoryStore::plan_initialize(&root, init_request(ProviderId::Claude))
        .expect_err("ikinci init reddedilmeli");
    assert!(matches!(error, MemoryError::AlreadyInitialized(_)));
    assert_eq!(
        original,
        fs::read(root.join(".mt-dna/project.json")).unwrap()
    );
    assert!(fs::read_to_string(root.join(".mt-dna/progress.md"))
        .unwrap()
        .contains("Test edilmedi."));
    assert!(MemoryStore::check(&root).unwrap().is_healthy());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_resume_and_handoff_preserve_language_and_verified_state() {
    let root = temporary_project("flow");
    initialize(&root);
    let update = MemoryStore::plan_update(
        &root,
        UpdateRequest {
            provider: ProviderId::Claude,
            completed: vec!["Parser tamamlandı".to_owned()],
            in_progress: Vec::new(),
            next: vec!["Entegrasyon testini çalıştır".to_owned()],
            blockers: vec!["Gemini CLI kurulu değil".to_owned()],
            test_results: vec!["cargo test: geçti".to_owned()],
            decision: Some(DecisionInput {
                context: "Hafıza depolama".to_owned(),
                options: vec!["dna-core modülü".to_owned(), "ayrı crate".to_owned()],
                decision: "ayrı crate".to_owned(),
                rationale: "I/O sınırını ayırır".to_owned(),
                affected_files: vec!["crates/dna-memory".to_owned()],
            }),
        },
    )
    .unwrap();
    MemoryStore::apply(&update).unwrap();

    let resume = MemoryStore::resume(&root).unwrap();
    assert!(resume.summary.contains("Dil: rust"));
    assert!(resume.summary.contains("Karar: ayrı crate"));
    assert!(resume.summary.contains("Entegrasyon testini çalıştır"));

    let handoff = MemoryStore::plan_handoff(&root, ProviderId::Gemini).unwrap();
    MemoryStore::apply(&handoff).unwrap();
    let snapshot = MemoryStore::load(&root).unwrap();
    assert_eq!(snapshot.project.language, "rust");
    assert!(snapshot.handoff.contains("Hedef provider: gemini"));
    assert!(snapshot.handoff.contains("cargo test: geçti"));
    assert!(snapshot.handoff.contains("/mt-developer-dna resume"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v04_plan_is_previewed_persisted_and_included_in_handoff() {
    let root = temporary_project("v04-plan");
    initialize(&root);
    let analysis = ProjectAnalyzer::default()
        .analyze(&root, Some("csharp"))
        .unwrap();
    assert_eq!(analysis.selected_language.as_deref(), Some("rust"));
    assert!(analysis
        .warnings
        .iter()
        .any(|warning| warning.contains("çelişiyor")));
    let plan =
        DevelopmentPlanner::create("sample-project", "PLAN: CLI geliştir", &analysis).unwrap();
    let memory_plan = MemoryBridge::plan_development_plan(&root, &plan, "# Plan\n").unwrap();
    assert!(!root.join(".mt-dna/plans/current.json").exists());
    MemoryStore::apply(&memory_plan).unwrap();

    let context = MemoryBridge::load(&root).unwrap();
    assert_eq!(context.selected_language, "rust");
    assert_eq!(context.current_plan.unwrap().plan_id, plan.plan_id);
    assert_eq!(
        MemoryStore::load(&root).unwrap().project.schema_version,
        MEMORY_SCHEMA_VERSION
    );

    let handoff = MemoryStore::plan_handoff(&root, ProviderId::Claude).unwrap();
    MemoryStore::apply(&handoff).unwrap();
    let handoff_text = fs::read_to_string(root.join(".mt-dna/handoff.md")).unwrap();
    assert!(handoff_text.contains("V0.4 geliştirme planı"));
    assert!(handoff_text.contains(&plan.plan_id));
    assert!(handoff_text.contains("çalıştırılmadıysa PASS sayma"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_v04_plan_is_rejected_without_fallback_claims() {
    let root = temporary_project("v04-invalid-plan");
    initialize(&root);
    fs::create_dir_all(root.join(".mt-dna/plans")).unwrap();
    fs::write(
        root.join(".mt-dna/plans/current.json"),
        r#"{"schema_version":"0.3.0","plan_id":"stale"}"#,
    )
    .unwrap();
    let error = MemoryBridge::load(&root).expect_err("bozuk plan reddedilmeli");
    assert!(matches!(error, MemoryError::Json { .. }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v05_quality_report_and_baseline_use_atomic_memory_plan() {
    let root = temporary_project("v05-quality");
    initialize(&root);
    let mut project_before = MemoryStore::load(&root).unwrap().project;
    let plan = MemoryBridge::plan_quality_report_with_baseline(
        &root,
        "quality",
        "2026-10-16T18-00-00Z-abcd1234",
        br#"{"schema_version":"0.5","timestamp_utc":"2026-10-16T18:00:00Z","release_gate":"BLOCKED"}"#.to_vec(),
        b"# Performance\n\nNOT_RUN\n".to_vec(),
        br#"{"schema_version":"0.5","repetitions":3}"#.to_vec(),
    )
    .unwrap();
    assert!(!root.join(".mt-dna/reports/quality").exists());
    MemoryStore::apply(&plan).unwrap();
    let project_after = MemoryStore::load(&root).unwrap().project;
    project_before.updated_at = project_after.updated_at.clone();
    assert_eq!(project_before, project_after);
    assert!(root
        .join(".mt-dna/reports/quality/2026-10-16T18-00-00Z-abcd1234.json")
        .is_file());
    assert!(root.join(".mt-dna/baselines/performance.json").is_file());
    assert!(MemoryStore::check(&root).unwrap().is_healthy());
    let handoff = MemoryStore::plan_handoff(&root, ProviderId::Gemini).unwrap();
    MemoryStore::apply(&handoff).unwrap();
    let handoff = fs::read_to_string(root.join(".mt-dna/handoff.md")).unwrap();
    assert!(handoff.contains("Son V0.5 kalite kanıtı"));
    assert!(handoff.contains("Release gate: BLOCKED"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v06_optimization_report_uses_versioned_memory_plan() {
    let root = temporary_project("v06-optimization");
    initialize(&root);
    let plan = MemoryBridge::plan_optimization_report(
        &root,
        "fix-abcd1234",
        br#"{"schema_version":"0.6","mode":"APPLY","status":"APPLIED"}"#.to_vec(),
        b"# Optimization\n\nAPPLIED\n".to_vec(),
    )
    .unwrap();
    assert!(!root.join(".mt-dna/reports/optimization").exists());
    MemoryStore::apply(&plan).unwrap();
    assert!(root
        .join(".mt-dna/reports/optimization/fix-abcd1234.json")
        .is_file());
    assert!(root
        .join(".mt-dna/reports/optimization/fix-abcd1234.md")
        .is_file());
    assert!(MemoryStore::check(&root).unwrap().is_healthy());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn env_content_and_user_secret_are_not_written() {
    let root = temporary_project("secrets");
    fs::write(root.join(".env"), "API_KEY=never-copy-this\n").unwrap();
    initialize(&root);
    let update = MemoryStore::plan_update(
        &root,
        UpdateRequest {
            provider: ProviderId::Codex,
            completed: vec!["API_KEY=fake-test-key".to_owned()],
            in_progress: Vec::new(),
            next: Vec::new(),
            blockers: Vec::new(),
            test_results: Vec::new(),
            decision: None,
        },
    )
    .unwrap();
    MemoryStore::apply(&update).unwrap();

    let all_memory = read_tree(&root.join(".mt-dna"));
    assert!(!all_memory.contains("never-copy-this"));
    assert!(!all_memory.contains("fake-test-key"));
    assert!(all_memory.contains("[REDACTED]"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_concurrent_update_is_rejected_without_data_loss() {
    let root = temporary_project("conflict");
    initialize(&root);
    let request = || UpdateRequest {
        provider: ProviderId::Codex,
        completed: vec!["Birinci yazım".to_owned()],
        in_progress: Vec::new(),
        next: Vec::new(),
        blockers: Vec::new(),
        test_results: Vec::new(),
        decision: None,
    };
    let first = MemoryStore::plan_update(&root, request()).unwrap();
    let stale = MemoryStore::plan_update(&root, request()).unwrap();
    MemoryStore::apply(&first).unwrap();
    let before = fs::read(root.join(".mt-dna/progress.md")).unwrap();
    let error = MemoryStore::apply(&stale).expect_err("stale plan reddedilmeli");
    assert!(matches!(error, MemoryError::Conflict(_)));
    assert_eq!(before, fs::read(root.join(".mt-dna/progress.md")).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn check_reports_traversal_and_old_schema_repair_migrates_safely() {
    let root = temporary_project("repair");
    initialize(&root);

    let project_path = root.join(".mt-dna/project.json");
    let mut project: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project_path).unwrap()).unwrap();
    project["schema_version"] = serde_json::Value::String("0.2.0".to_owned());
    fs::write(&project_path, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
    let audit = MemoryStore::check(&root).unwrap();
    assert!(audit
        .issues
        .iter()
        .any(|issue| issue.code == "project_schema"));

    let repair = MemoryStore::plan_repair(&root).unwrap();
    let report = MemoryStore::apply(&repair).unwrap();
    assert!(report.backup_id.is_some());
    assert_eq!(
        MemoryStore::load(&root).unwrap().project.schema_version,
        MEMORY_SCHEMA_VERSION
    );

    let manifest_path = root.join(".mt-dna/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["files"][0]["path"] = serde_json::Value::String("../outside".to_owned());
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let error = MemoryStore::check(&root).expect_err("path traversal reddedilmeli");
    assert!(matches!(error, MemoryError::UnsafePath(_)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_change_is_reported_as_stale_and_missing_manifest_can_be_repaired() {
    let root = temporary_project("stale-source");
    initialize(&root);
    fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();
    let audit = MemoryStore::check(&root).unwrap();
    assert!(audit
        .issues
        .iter()
        .any(|issue| issue.code == "stale_source"));

    fs::remove_file(root.join(".mt-dna/manifest.json")).unwrap();
    let repair = MemoryStore::plan_repair(&root).unwrap();
    MemoryStore::apply(&repair).unwrap();
    assert!(MemoryStore::check(&root).unwrap().is_healthy());
    fs::remove_dir_all(root).unwrap();
}

fn read_tree(root: &Path) -> String {
    let mut output = String::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            output.push_str(&read_tree(&path));
        } else {
            output.push_str(&String::from_utf8_lossy(&fs::read(path).unwrap()));
        }
    }
    output
}
