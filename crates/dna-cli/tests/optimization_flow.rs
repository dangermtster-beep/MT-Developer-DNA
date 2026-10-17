use dna_cli::{run, Cli, Command, OptimizationEngine, OptimizeArgs, OptimizeMode};
use dna_core::OptimizationMode;
use dna_memory::{InitRequest, MemoryStore, ProviderId};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID: AtomicU64 = AtomicU64::new(0);

fn project() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-optimization-flow")
        .join(format!(
            "{}-{nonce}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='optimization_fixture'\nversion='0.1.0'\nedition='2021'\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {   \n}\n").unwrap();
    let init = MemoryStore::plan_initialize(
        &root,
        InitRequest {
            project_id: "optimization-fixture".to_owned(),
            name: "Optimization fixture".to_owned(),
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
    MemoryStore::apply(&init).unwrap();
    root
}

#[test]
fn approved_fix_runs_gates_records_report_and_rolls_back() {
    let root = project();
    let report = OptimizationEngine::analyze(
        &root,
        "optimization-fixture",
        "rust",
        OptimizationMode::Analyze,
    )
    .unwrap();
    let finding = report
        .findings
        .iter()
        .find(|item| item.auto_fixable)
        .unwrap();
    let plan = OptimizationEngine::plan_fix(&root, &report, &finding.finding_id).unwrap();

    let applied = run(Cli {
        command: Command::Optimize(OptimizeArgs {
            project: root.clone(),
            language: Some("rust".to_owned()),
            mode: OptimizeMode::Apply,
            finding_id: Some(finding.finding_id.clone()),
            fix_id: None,
            confirm: true,
        }),
    })
    .unwrap();
    assert!(applied.contains("Durum: Applied"));
    assert_eq!(
        fs::read_to_string(root.join("src/main.rs")).unwrap(),
        "fn main() {\n}\n"
    );
    assert!(root
        .join(format!(".mt-dna/reports/optimization/{}.json", plan.fix_id))
        .is_file());
    let stored = run(Cli {
        command: Command::OptimizationReport {
            project: root.clone(),
            fix_id: Some(plan.fix_id.clone()),
        },
    })
    .unwrap();
    assert!(stored.contains("İşlem kaydı"));

    let rolled_back = run(Cli {
        command: Command::Optimize(OptimizeArgs {
            project: root.clone(),
            language: Some("rust".to_owned()),
            mode: OptimizeMode::Rollback,
            finding_id: None,
            fix_id: Some(plan.fix_id.clone()),
            confirm: true,
        }),
    })
    .unwrap();
    assert!(rolled_back.contains("Durum: RolledBack"));
    assert_eq!(
        fs::read_to_string(root.join("src/main.rs")).unwrap(),
        "fn main() {   \n}\n"
    );
    fs::write(
        root.join(format!(".mt-dna/reports/optimization/{}.json", plan.fix_id)),
        "{broken",
    )
    .unwrap();
    assert!(run(Cli {
        command: Command::OptimizationReport {
            project: root.clone(),
            fix_id: Some(plan.fix_id),
        },
    })
    .is_err());
    fs::remove_dir_all(root).unwrap();
}
