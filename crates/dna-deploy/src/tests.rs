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
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile-rust.json"),
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
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile-rust.json"),
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

    let detection = detect_provider_with_path(ProviderKey::Antigravity, Some(&desktop)).unwrap();
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
