use dna_core::{validate_skill_manifest, LanguageProfile};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mt-developer-dna")
}

#[test]
fn canonical_skill_manifest_is_valid() {
    let source = fs::read_to_string(skill_root().join("SKILL.md")).unwrap();
    let manifest = validate_skill_manifest(&source, "mt-developer-dna").unwrap();
    assert_eq!(manifest.name, "mt-developer-dna");
    assert!(!manifest.description.trim().is_empty());
    assert!(manifest.description.len() <= 1024);
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
