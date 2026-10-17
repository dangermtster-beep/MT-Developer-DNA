use dna_cli::{apply_install, plan_install, rollback_install, CliError, InstallStatus};
use dna_core::Profile;
use dna_render::{render_package, Provider, ProviderPackage, RenderedFile};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn temporary_project(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-install")
        .join(format!("mt-dna-{name}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn codex_package() -> ProviderPackage {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    render_package(&profile, Provider::Codex).unwrap()
}

#[test]
fn planning_never_changes_the_project() {
    let root = temporary_project("preview");
    fs::write(root.join("keep.txt"), "koru").unwrap();

    let plan = plan_install(&root, &[codex_package()]).unwrap();

    assert!(plan
        .changes
        .iter()
        .all(|change| change.status == InstallStatus::Create));
    assert_eq!(fs::read_to_string(root.join("keep.txt")).unwrap(), "koru");
    assert!(!root.join("AGENTS.md").exists());
    assert!(!root.join(".mt-developer-dna").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn install_is_idempotent_and_keeps_unrelated_files() {
    let root = temporary_project("idempotent");
    fs::write(root.join("keep.txt"), "koru").unwrap();

    // AGENTS.md + SKILL.md + skill ile dağıtılan references/ dosyaları.
    let file_count = codex_package().files.len();
    assert!(file_count > 2, "references/ pakete dahil olmalı");

    let first_plan = plan_install(&root, &[codex_package()]).unwrap();
    let first = apply_install(&first_plan).unwrap();
    assert_eq!(first.changed, file_count);
    assert!(first.backup_id.is_some());
    assert!(root
        .join(".agents/skills/mt-developer-dna/references/modes.md")
        .exists());

    let second_plan = plan_install(&root, &[codex_package()]).unwrap();
    let second = apply_install(&second_plan).unwrap();
    assert_eq!(second.changed, 0);
    assert_eq!(second.unchanged, file_count);
    assert!(second.backup_id.is_none());
    assert_eq!(fs::read_to_string(root.join("keep.txt")).unwrap(), "koru");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn existing_file_is_backed_up_and_rollback_restores_it() {
    let root = temporary_project("rollback");
    fs::write(root.join("AGENTS.md"), "kullanıcının özgün dosyası\n").unwrap();

    let plan = plan_install(&root, &[codex_package()]).unwrap();
    assert_eq!(
        plan.changes
            .iter()
            .find(|change| change.relative_path == "AGENTS.md")
            .unwrap()
            .status,
        InstallStatus::Update
    );
    let report = apply_install(&plan).unwrap();
    let backup_id = report.backup_id.unwrap();
    assert_ne!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "kullanıcının özgün dosyası\n"
    );

    let rollback = rollback_install(&root, &backup_id).unwrap();
    assert_eq!(rollback.restored, 1);
    // AGENTS.md geri yüklenir; yeni oluşturulan diğer tüm dosyalar kaldırılır.
    assert_eq!(rollback.removed, codex_package().files.len() - 1);
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "kullanıcının özgün dosyası\n"
    );
    assert!(!root
        .join(".agents/skills/mt-developer-dna/SKILL.md")
        .exists());
    assert!(rollback.backup_path.join("manifest.json").is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn path_traversal_output_is_rejected() {
    let root = temporary_project("traversal");
    let package = ProviderPackage {
        provider: Provider::Codex,
        files: vec![RenderedFile {
            relative_path: "../AGENTS.md".to_owned(),
            contents: "kaçış".to_owned(),
        }],
        warnings: Vec::new(),
    };

    let error = plan_install(&root, &[package]).expect_err("path traversal reddedilmeli");
    assert!(matches!(error, CliError::UnsafeOutputPath(_)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn install_rejects_a_target_changed_after_preview() {
    let root = temporary_project("stale-plan");
    fs::write(root.join("AGENTS.md"), "ilk içerik\n").unwrap();
    let plan = plan_install(&root, &[codex_package()]).unwrap();
    fs::write(root.join("AGENTS.md"), "preview sonrası değişti\n").unwrap();

    let error = apply_install(&plan).expect_err("stale plan reddedilmeli");
    assert!(matches!(error, CliError::TargetChanged(_)));
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "preview sonrası değişti\n"
    );
    assert!(!root.join(".mt-developer-dna").exists());
    fs::remove_dir_all(root).unwrap();
}
