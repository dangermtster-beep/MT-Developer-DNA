use dna_cli::{export_packages, CliError};
use dna_core::Profile;
use dna_render::{render_package, Provider};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn temporary_test_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-temp")
        .join(format!("mt-dna-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn export_creates_provider_package_without_touching_other_files() {
    let root = temporary_test_directory("new-export");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.txt"), "koru").unwrap();
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    let package = render_package(&profile, Provider::Codex).unwrap();

    export_packages(&root, &[package]).unwrap();

    assert_eq!(fs::read_to_string(root.join("keep.txt")).unwrap(), "koru");
    assert!(root.join("codex/AGENTS.md").is_file());
    assert!(root
        .join("codex/.agents/skills/mt-developer-dna/SKILL.md")
        .is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn second_export_refuses_to_overwrite_existing_provider_directory() {
    let root = temporary_test_directory("no-overwrite");
    fs::create_dir_all(root.join("codex")).unwrap();
    fs::write(root.join("codex/AGENTS.md"), "kullanıcı dosyası").unwrap();
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    let package = render_package(&profile, Provider::Codex).unwrap();

    let error = export_packages(&root, &[package]).expect_err("üzerine yazma reddedilmeli");
    assert!(matches!(error, CliError::DestinationExists(_)));
    assert_eq!(
        fs::read_to_string(root.join("codex/AGENTS.md")).unwrap(),
        "kullanıcı dosyası"
    );
    fs::remove_dir_all(root).unwrap();
}
