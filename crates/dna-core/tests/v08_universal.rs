use dna_core::{resolve_language, LanguageSource, Profile, ProgrammingLanguage};
use std::path::PathBuf;

#[test]
fn universal_profile_is_valid_and_requires_user_language_choice() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles/universal.json");
    let profile = Profile::load(path).unwrap();
    assert_eq!(profile.language, ProgrammingLanguage::AskUser);
    assert!(profile.features.low_memory);
    assert!(profile.features.low_idle_cpu);
    assert!(profile.features.tests_required);
}

#[test]
fn project_memory_precedes_universal_default() {
    let result = resolve_language(
        None,
        Some(ProgrammingLanguage::Python),
        Some(ProgrammingLanguage::Go),
        Some(ProgrammingLanguage::Rust),
    );
    assert_eq!(result.language, ProgrammingLanguage::Python);
    assert_eq!(result.source, LanguageSource::ProjectMemory);
}

#[test]
fn every_explicit_starting_language_remains_selectable() {
    for language in ["Rust", "C#", "Python", "C++", "Go", "Java", "TypeScript"] {
        let json = format!(
            r#"{{"schema_version":1,"name":"test","language":"{language}","ui":"Auto","database":"Auto"}}"#
        );
        let profile = Profile::from_json_str(&json).unwrap();
        assert_eq!(profile.language.display_name(), language);
    }
}
