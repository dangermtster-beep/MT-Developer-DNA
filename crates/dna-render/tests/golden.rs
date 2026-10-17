use dna_core::{validate_skill_manifest, Profile};
use dna_render::{
    provider_descriptor, remap_skill_file, render_package, CapabilityState, Provider,
    PROVIDER_REGISTRY, SKILL_DESCRIPTION, SKILL_REFERENCES,
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

        // references/ kaynağıyla birebir aynıdır; ayrı testte doğrulanır.
        for file in first
            .files
            .into_iter()
            .filter(|file| !file.relative_path.contains("/references/"))
        {
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
            // Paylaşılan references/ dosyaları dilden bağımsızdır.
            .filter(|file| file.relative_path.ends_with(".md")
                && !file.relative_path.contains("/references/"))
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

#[test]
fn every_package_ships_references_next_to_its_skill() {
    let profile = Profile::load(fixture_root().join("../../profiles/universal.json")).unwrap();
    for provider in Provider::ALL {
        let package = render_package(&profile, provider).unwrap();
        let skill_path = provider_descriptor(provider).project_skill_path;
        let skill_dir = skill_path.strip_suffix("SKILL.md").unwrap();
        let skill = package
            .files
            .iter()
            .find(|file| file.relative_path == skill_path)
            .unwrap();

        for (name, _, contents) in SKILL_REFERENCES {
            let expected_path = format!("{skill_dir}references/{name}");
            let file = package
                .files
                .iter()
                .find(|file| file.relative_path == expected_path)
                .unwrap_or_else(|| panic!("{expected_path} pakette olmalı"));
            assert_eq!(file.contents, contents);
            assert!(!contents.trim().is_empty(), "{name} boş olmamalı");
            assert!(
                skill.contents.contains(&format!("(references/{name})")),
                "SKILL.md {name} dosyasına bağlantı vermeli"
            );
        }
        assert!(skill.contents.contains(".mt-dna/project.json"));
    }
}

#[test]
fn remap_skill_file_maps_skill_and_references_only() {
    let project = ".claude/skills/mt-developer-dna/SKILL.md";
    let target = "custom/skills/mt-developer-dna/SKILL.md";
    assert_eq!(
        remap_skill_file(project, project, target).as_deref(),
        Some(target)
    );
    assert_eq!(
        remap_skill_file(
            ".claude/skills/mt-developer-dna/references/modes.md",
            project,
            target
        )
        .as_deref(),
        Some("custom/skills/mt-developer-dna/references/modes.md")
    );
    assert_eq!(remap_skill_file("CLAUDE.md", project, target), None);
    assert_eq!(
        remap_skill_file(".claude/skills/mt-developer-dna/", project, target),
        None
    );
}

#[test]
fn skill_description_is_language_independent_for_every_profile_and_provider() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profiles = [
        Profile::load(workspace.join("tests/fixtures/profile-rust.json")).unwrap(),
        Profile::load(workspace.join("profiles/universal.json")).unwrap(),
    ];
    let expected_line = format!("description: {SKILL_DESCRIPTION}");

    for profile in &profiles {
        for provider in Provider::ALL {
            let package = render_package(profile, provider).unwrap();
            let skill = package
                .files
                .iter()
                .find(|file| file.relative_path.ends_with("SKILL.md"))
                .unwrap_or_else(|| panic!("{} için SKILL.md üretilmeli", provider.display_name()));
            let description = skill
                .contents
                .lines()
                .find(|line| line.starts_with("description:"))
                .expect("frontmatter description satırı olmalı");

            assert_eq!(description, expected_line);
            assert!(
                !description.contains(profile.language.display_name()),
                "açıklama profil dilini içermemeli: {description}"
            );
            assert!(
                description.is_ascii(),
                "açıklamaya Türkçe yer tutucu sızmamalı: {description}"
            );
        }
    }
}
