use dna_core::{Profile, ProfileLoadError};

const VALID_PROFILE: &str = include_str!("../../../tests/fixtures/profile-rust.json");

#[test]
fn valid_json_profile_is_loaded() {
    let profile = Profile::from_json_str(VALID_PROFILE).expect("profil geçerli olmalı");
    assert_eq!(profile.language.display_name(), "Rust");
    assert_eq!(profile.ui.display_name(), "Slint");
}

#[test]
fn global_scope_profile_drops_project_choices_and_stays_valid() {
    let profile = Profile::from_json_str(VALID_PROFILE).unwrap();
    assert!(!profile.is_global_neutral());

    let global = profile.for_global_scope();
    assert!(global.is_global_neutral());
    assert_eq!(
        global.language.display_name(),
        "Kullanıcı seçimi bekleniyor"
    );
    assert_eq!(global.ui.display_name(), "Auto");
    assert_eq!(global.name, profile.name);
    assert_eq!(global.features, profile.features);
    assert_eq!(global.standards, profile.standards);
    global
        .validate()
        .expect("nötr global profil geçerli olmalı");
}

#[test]
fn incompatible_ui_is_reported_with_alternatives() {
    let json = VALID_PROFILE.replace("\"Slint\"", "\"Wpf\"");
    let error = Profile::from_json_str(&json).expect_err("uyumsuz UI reddedilmeli");
    let ProfileLoadError::Validation(errors) = error else {
        panic!("validasyon hatası bekleniyordu");
    };

    let issue = errors
        .0
        .iter()
        .find(|issue| issue.code == "incompatible_language_ui")
        .expect("uyumsuzluk hatası bulunmalı");
    assert!(issue.alternatives.contains(&"Slint".to_owned()));
    let display = errors.to_string();
    assert!(display.contains("Rust dili ile WPF arayüzü uyumlu değil"));
    assert!(display.contains("Alternatifler: Auto, None, CLI, Slint"));
}

#[test]
fn possible_secret_in_rule_is_rejected() {
    let json = VALID_PROFILE.replace(
        "Dar kapsamlı ve geri alınabilir değişiklik yap.",
        "API_KEY=super-secret",
    );
    let error = Profile::from_json_str(&json).expect_err("gizli bilgi reddedilmeli");
    assert!(error.to_string().contains("Profil doğrulanamadı"));
}
