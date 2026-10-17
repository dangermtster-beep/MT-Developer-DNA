use dna_core::LanguageProfile;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// SKILL.md artık yalnızca dna-render tarafından üretilir (bkz. dna-render golden
// testleri); bu klasör skill ile dağıtılan references/ kaynağıdır.
fn skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mt-developer-dna")
}

#[test]
fn skill_source_folder_holds_only_references() {
    assert!(!skill_root().join("SKILL.md").exists());
    assert!(skill_root().join("references/modes.md").is_file());
}

#[test]
fn all_starting_language_profiles_are_valid_and_unique() {
    let directory = skill_root().join("references/languages");
    let mut paths = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for path in &paths {
        let source = fs::read_to_string(path).unwrap();
        let profile = LanguageProfile::from_json_str(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(ids.insert(profile.id));
        assert!(names.insert(profile.display_name));
    }

    assert_eq!(paths.len(), 7);
    assert_eq!(
        names,
        BTreeSet::from([
            "C#".to_owned(),
            "C++".to_owned(),
            "Go".to_owned(),
            "Java".to_owned(),
            "Python".to_owned(),
            "Rust".to_owned(),
            "TypeScript".to_owned(),
        ])
    );
}
