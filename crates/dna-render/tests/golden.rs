use dna_core::{validate_skill_manifest, Profile};
use dna_render::{
    provider_descriptor, render_package, CapabilityState, Provider, PROVIDER_REGISTRY,
};
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

#[test]
fn all_provider_outputs_match_golden_files() {
    let fixture_root = fixture_root();
    let profile = Profile::load(fixture_root.join("profile-rust.json")).unwrap();

    for provider in [Provider::Codex, Provider::Claude, Provider::Gemini] {
        let first = render_package(&profile, provider).unwrap();
        let second = render_package(&profile, provider).unwrap();
        assert_eq!(first, second, "çıktı deterministik olmalı");

        for file in first.files {
            let expected_path = fixture_root
                .join("golden")
                .join(provider.directory_name())
                .join(&file.relative_path);
            let expected = fs::read_to_string(&expected_path).unwrap_or_else(|error| {
                panic!(
                    "golden dosya okunamadı {}: {error}",
                    expected_path.display()
                )
            });
            assert_eq!(file.contents, expected, "{} eşleşmedi", file.relative_path);
        }
    }
}

#[test]
fn selected_language_is_preserved_for_every_provider() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    for provider in Provider::ALL {
        let package = render_package(&profile, provider).unwrap();
        assert!(package
            .files
            .iter()
            .filter(|file| file.relative_path.ends_with(".md"))
            .all(|file| file.contents.contains("Rust")));
    }
}

#[test]
fn every_provider_contains_a_valid_skill_and_expected_invocation() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    for (provider, expected_invocation) in [
        (Provider::Codex, "$mt-developer-dna"),
        (Provider::Claude, "/mt-developer-dna"),
        (Provider::Gemini, "/mt-developer-dna"),
        (Provider::Antigravity, "/mt-developer-dna"),
        (Provider::Cursor, "/mt-developer-dna"),
        (Provider::Cline, "/mt-developer-dna"),
        (Provider::OpenCode, "/mt-developer-dna"),
    ] {
        let package = render_package(&profile, provider).unwrap();
        let skill = package
            .files
            .iter()
            .find(|file| file.relative_path.ends_with("/SKILL.md"))
            .expect("provider skill dosyası üretmeli");
        validate_skill_manifest(&skill.contents, "mt-developer-dna").unwrap();
        assert!(skill.contents.contains(expected_invocation));
    }
}

#[test]
fn gemini_adapter_emits_skill_and_slash_command() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    let package = render_package(&profile, Provider::Gemini).unwrap();
    let paths = package
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&".gemini/skills/mt-developer-dna/SKILL.md"));
    assert!(paths.contains(&".gemini/commands/mt-developer-dna.toml"));
}

#[test]
fn v08_adapters_emit_official_skill_paths() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    for (provider, expected_path) in [
        (
            Provider::Antigravity,
            ".agents/skills/mt-developer-dna/SKILL.md",
        ),
        (Provider::Cursor, ".cursor/skills/mt-developer-dna/SKILL.md"),
        (Provider::Cline, ".cline/skills/mt-developer-dna/SKILL.md"),
        (
            Provider::OpenCode,
            ".opencode/skills/mt-developer-dna/SKILL.md",
        ),
    ] {
        let package = render_package(&profile, provider).unwrap();
        assert!(package
            .files
            .iter()
            .any(|file| file.relative_path == expected_path));
    }
}

#[test]
fn antigravity_does_not_emit_deprecated_workflow() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    let package = render_package(&profile, Provider::Antigravity).unwrap();
    assert!(package
        .files
        .iter()
        .all(|file| !file.relative_path.contains("workflows")));
    assert!(package
        .warnings
        .iter()
        .any(|warning| warning.contains("19 Ekim 2026")));
}

#[test]
fn opencode_skill_enables_slash_metadata() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    let package = render_package(&profile, Provider::OpenCode).unwrap();
    let skill = package
        .files
        .iter()
        .find(|file| file.relative_path.ends_with("SKILL.md"))
        .unwrap();
    assert!(skill.contents.contains("opencode/slash: \"true\""));
}

#[test]
fn provider_registry_has_stable_unique_ids_and_explicit_capabilities() {
    let mut ids = PROVIDER_REGISTRY
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), Provider::ALL.len());
    assert_eq!(
        provider_descriptor(Provider::Codex).slash_command,
        CapabilityState::Unsupported
    );
    assert_eq!(
        provider_descriptor(Provider::OpenCode).rules,
        CapabilityState::Unverified
    );
}

#[test]
fn renderer_only_emits_safe_relative_paths() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    for provider in Provider::ALL {
        let package = render_package(&profile, provider).unwrap();
        assert!(package
            .files
            .iter()
            .all(|file| file.has_safe_relative_path()));
    }
}

#[test]
fn every_provider_reads_the_same_project_memory_contract() {
    let profile = Profile::load(fixture_root().join("profile-rust.json")).unwrap();
    for provider in Provider::ALL {
        let package = render_package(&profile, provider).unwrap();
        let combined = package
            .files
            .iter()
            .map(|file| file.contents.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("`.mt-dna/`"));
        assert!(combined.contains("memory check"));
        assert!(combined.contains("handoff"));
        assert!(combined.contains("sohbet geçmişlerinin ortak olduğunu varsayma"));
        assert!(combined.contains("PLAN, BUILD ve REVIEW modları"));
        assert!(combined.contains("Plan/review kaydı için diff göster"));
        assert!(combined.contains("V0.5 kalite ve ölçüm sistemi"));
        assert!(combined.contains("release-check"));
        assert!(combined.contains("`NOT_RUN`"));
        assert!(combined.contains("V0.6 kontrollü optimizasyon ve Auto-Fix"));
        assert!(combined.contains("optimize --mode rollback --fix-id <id>"));
        assert!(combined.contains("hash uyuşmazlığında dur"));
        assert!(combined.contains("V0.7 Universal Plugin Deployment"));
        assert!(combined.contains("user-modified/conflict"));
        assert!(combined.contains("PASS, WARN, FAIL, NOT_RUN ve UNSUPPORTED"));
    }
}
