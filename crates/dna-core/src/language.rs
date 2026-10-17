use crate::LanguageProfileErrors;
use serde::{Deserialize, Serialize};

pub const LANGUAGE_PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LanguageProfile {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub build_commands: Vec<String>,
    #[serde(default)]
    pub test_commands: Vec<String>,
    #[serde(default)]
    pub release_commands: Vec<String>,
    #[serde(default)]
    pub principles: Vec<String>,
}

impl LanguageProfile {
    pub fn from_json_str(json: &str) -> Result<Self, LanguageProfileErrors> {
        let profile: Self = serde_json::from_str(json).map_err(|error| {
            LanguageProfileErrors(vec![format!("JSON biçimi geçersiz: {error}")])
        })?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), LanguageProfileErrors> {
        let mut errors = Vec::new();
        if self.schema_version != LANGUAGE_PROFILE_SCHEMA_VERSION {
            errors.push(format!(
                "Desteklenmeyen şema sürümü: {}. Beklenen: {}.",
                self.schema_version, LANGUAGE_PROFILE_SCHEMA_VERSION
            ));
        }
        if !is_safe_id(&self.id) {
            errors.push(
                "id yalnızca küçük ASCII harf, rakam ve tek tire grupları içermelidir.".to_owned(),
            );
        }
        if self.display_name.trim().is_empty() || self.display_name.chars().count() > 40 {
            errors.push("display_name 1-40 karakter arasında olmalıdır.".to_owned());
        }
        if self.file_extensions.is_empty() {
            errors.push("En az bir dosya uzantısı tanımlanmalıdır.".to_owned());
        }
        if self.test_commands.is_empty() {
            errors.push("En az bir test komutu veya test yönergesi tanımlanmalıdır.".to_owned());
        }

        for (field, values) in [
            ("aliases", &self.aliases),
            ("file_extensions", &self.file_extensions),
            ("build_commands", &self.build_commands),
            ("test_commands", &self.test_commands),
            ("release_commands", &self.release_commands),
            ("principles", &self.principles),
        ] {
            if values.iter().any(|value| value.trim().is_empty()) {
                errors.push(format!("{field} boş değer içeremez."));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(LanguageProfileErrors(errors))
        }
    }
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.contains("--")
}
