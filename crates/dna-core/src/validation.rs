use crate::{Profile, ProgrammingLanguage, UiFramework, PROFILE_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
    pub alternatives: Vec<String>,
}

pub fn validate_profile(profile: &Profile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if profile.schema_version != PROFILE_SCHEMA_VERSION {
        issues.push(issue(
            "unsupported_schema_version",
            "schema_version",
            format!(
                "Desteklenmeyen profil sürümü: {}. Beklenen sürüm: {}.",
                profile.schema_version, PROFILE_SCHEMA_VERSION
            ),
            vec![PROFILE_SCHEMA_VERSION.to_string()],
        ));
    }

    let trimmed_name = profile.name.trim();
    if trimmed_name.is_empty() || trimmed_name.chars().count() > 80 {
        issues.push(issue(
            "invalid_profile_name",
            "name",
            "Profil adı 1-80 karakter arasında olmalıdır.".to_owned(),
            Vec::new(),
        ));
    }

    if !ui_is_compatible(profile.language, profile.ui) {
        issues.push(issue(
            "incompatible_language_ui",
            "ui",
            format!(
                "{} dili ile {} arayüzü uyumlu değil; seçim sessizce değiştirilmeyecek.",
                profile.language.display_name(),
                profile.ui.display_name()
            ),
            compatible_ui_names(profile.language),
        ));
    }

    if profile.features.i18n {
        if profile.features.default_locale.trim().is_empty() {
            issues.push(issue(
                "missing_default_locale",
                "features.default_locale",
                "i18n açıkken varsayılan dil boş olamaz.".to_owned(),
                vec!["tr-TR".to_owned()],
            ));
        }
        if !profile
            .features
            .supported_locales
            .iter()
            .any(|locale| locale == &profile.features.default_locale)
        {
            issues.push(issue(
                "default_locale_not_supported",
                "features.supported_locales",
                "Varsayılan dil desteklenen diller listesinde bulunmalıdır.".to_owned(),
                vec![profile.features.default_locale.clone()],
            ));
        }
    }

    validate_rule_groups(profile, &mut issues);
    issues
}

fn validate_rule_groups(profile: &Profile, issues: &mut Vec<ValidationIssue>) {
    let groups = [
        ("standards.architecture", &profile.standards.architecture),
        ("standards.quality", &profile.standards.quality),
        ("standards.security", &profile.standards.security),
        (
            "standards.performance_benchmarks",
            &profile.standards.performance_benchmarks,
        ),
    ];

    for (path, rules) in groups {
        for (index, rule) in rules.iter().enumerate() {
            if rule.trim().is_empty() {
                issues.push(issue(
                    "empty_standard_rule",
                    format!("{path}[{index}]"),
                    "Standart kuralı boş olamaz.".to_owned(),
                    Vec::new(),
                ));
            }
            if looks_like_secret(rule) {
                issues.push(issue(
                    "possible_secret",
                    format!("{path}[{index}]"),
                    "Kural olası bir gizli bilgi içeriyor; provider çıktısına gömülmedi."
                        .to_owned(),
                    Vec::new(),
                ));
            }
        }
    }
}

fn looks_like_secret(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    [
        "-----begin private key-----",
        "api_key=",
        "api-key=",
        "secret_key=",
        "access_token=",
        "bearer eyj",
        "sk-proj-",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

fn ui_is_compatible(language: ProgrammingLanguage, ui: UiFramework) -> bool {
    use ProgrammingLanguage as L;
    use UiFramework as U;

    matches!(ui, U::Auto | U::None | U::Cli)
        || match language {
            L::Auto | L::AskUser => true,
            L::Rust => matches!(ui, U::Slint | U::Gtk | U::Tauri),
            L::CSharp => matches!(ui, U::Wpf | U::WinUi3 | U::Avalonia),
            L::Python => matches!(ui, U::Qt | U::Gtk | U::Web),
            L::Cpp => matches!(ui, U::Qt),
            L::Go => matches!(ui, U::Web),
            L::Java => matches!(ui, U::JavaFx | U::Web),
            L::TypeScript => matches!(ui, U::React | U::Vue | U::Tauri | U::Web),
        }
}

fn compatible_ui_names(language: ProgrammingLanguage) -> Vec<String> {
    use ProgrammingLanguage as L;
    let language_specific: &[&str] = match language {
        L::Auto | L::AskUser => &[
            "Slint", "WPF", "WinUI 3", "Avalonia", "Qt", "GTK", "Tauri", "JavaFX", "React", "Vue",
            "Web",
        ],
        L::Rust => &["Slint", "GTK", "Tauri"],
        L::CSharp => &["WPF", "WinUI 3", "Avalonia"],
        L::Python => &["Qt", "GTK", "Web"],
        L::Cpp => &["Qt"],
        L::Go => &["Web"],
        L::Java => &["JavaFX", "Web"],
        L::TypeScript => &["React", "Vue", "Tauri", "Web"],
    };

    ["Auto", "None", "CLI"]
        .into_iter()
        .chain(language_specific.iter().copied())
        .map(str::to_owned)
        .collect()
}

fn issue(
    code: &'static str,
    path: impl Into<String>,
    message: String,
    alternatives: Vec<String>,
) -> ValidationIssue {
    ValidationIssue {
        code: code.to_owned(),
        path: path.into(),
        message,
        alternatives,
    }
}
