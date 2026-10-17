use super::{
    deployment_outcome_line, profile_for_scope, Cli, DeploymentReport, InstallScope, Profile,
};
use clap::{CommandFactory, Parser};
use dna_core::ProgrammingLanguage;
use dna_render::{render_package, Provider};

#[test]
fn cli_argument_definition_has_no_conflicts() {
    Cli::command().debug_assert();
}

#[test]
fn v07_documented_command_shapes_parse() {
    for args in [
        vec!["mt-dna", "providers", "list"],
        vec![
            "mt-dna",
            "deploy",
            "sync",
            "--profile",
            "profile.json",
            "--provider",
            "codex",
            "--dry-run",
        ],
        vec![
            "mt-dna",
            "deploy",
            "rollback",
            "--transaction",
            "tx-1",
            "--yes",
        ],
        vec!["mt-dna", "doctor", "--provider", "all", "--json"],
    ] {
        Cli::try_parse_from(args).expect("V0.7 belgelenen komut biçimi parse edilmeli");
    }
}

#[test]
fn v08_profile_and_provider_commands_parse() {
    for args in [
        vec!["mt-dna", "profiles", "list"],
        vec![
            "mt-dna",
            "profiles",
            "validate",
            "--profile",
            "profiles/universal.json",
        ],
        vec!["mt-dna", "doctor", "--provider", "antigravity"],
        vec![
            "mt-dna",
            "doctor",
            "--provider",
            "antigravity",
            "--provider-path",
            "C:/Tools/Antigravity IDE.exe",
        ],
        vec![
            "mt-dna",
            "deploy",
            "plan",
            "--profile",
            "profiles/universal.json",
            "--provider",
            "antigravity",
            "--scope",
            "global",
            "--provider-path",
            "C:/Tools/agy.exe",
        ],
        vec![
            "mt-dna",
            "preview",
            "--profile",
            "profiles/universal.json",
            "--provider",
            "opencode",
        ],
        vec![
            "mt-dna",
            "handoff",
            "--project",
            ".",
            "--provider",
            "cursor",
        ],
    ] {
        Cli::try_parse_from(args).expect("V0.8 komut biçimi parse edilmeli");
    }
}

fn workspace_profile(relative: &str) -> Profile {
    Profile::load(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative),
    )
    .unwrap()
}

#[test]
fn global_scope_neutralizes_language_specific_profile() {
    let rust = workspace_profile("tests/fixtures/profile-rust.json");
    let (profile, note) = profile_for_scope(rust.clone(), InstallScope::Global);

    assert_eq!(profile.language, ProgrammingLanguage::AskUser);
    assert!(profile.is_global_neutral());
    assert_eq!(profile.name, rust.name);
    assert_eq!(profile.standards, rust.standards);
    assert!(note.is_some(), "nötrleme kullanıcıya bildirilmeli");

    for provider in Provider::ALL {
        let package = render_package(&profile, provider).unwrap();
        let combined = package
            .files
            .iter()
            .map(|file| file.contents.as_str())
            .collect::<String>();
        assert!(!combined.contains("aynen koru: Rust"));
        assert!(combined.contains("Programlama dili seçilmedi"));
    }
}

#[test]
fn outcome_line_reports_apply_and_dry_run_truthfully() {
    let report = DeploymentReport {
        transaction_id: Some("tx-1".to_owned()),
        changed: 1,
        unchanged: 0,
        skipped: 0,
    };
    let applied = deployment_outcome_line(Some(&report));
    assert!(applied.starts_with("Uygulandı: changed=1"));
    assert!(applied.contains("transaction=tx-1"));
    assert!(!applied.contains("değiştirilmedi"));

    let dry_run = deployment_outcome_line(None);
    assert!(dry_run.starts_with("Dry-run"));
    assert!(dry_run.contains("hiçbir dosya değiştirilmedi"));
}

#[test]
fn project_scope_keeps_selected_language() {
    let rust = workspace_profile("tests/fixtures/profile-rust.json");
    let (profile, note) = profile_for_scope(rust.clone(), InstallScope::Project);

    assert_eq!(profile, rust);
    assert!(note.is_none());
}

#[test]
fn already_neutral_global_profile_is_left_untouched() {
    let universal = workspace_profile("profiles/universal.json");
    let (profile, note) = profile_for_scope(universal.clone(), InstallScope::Global);

    assert!(universal.is_global_neutral());
    assert_eq!(profile, universal);
    assert!(note.is_none());
}
