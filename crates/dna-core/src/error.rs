use crate::ValidationIssue;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileLoadError {
    #[error("Profil dosyası okunamadı ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Profil JSON biçimi geçersiz: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(pub Vec<ValidationIssue>);

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Profil doğrulanamadı ({} hata):", self.0.len())?;
        for issue in &self.0 {
            write!(formatter, "\n- [{}] {}", issue.path, issue.message)?;
            if !issue.alternatives.is_empty() {
                write!(
                    formatter,
                    " Alternatifler: {}.",
                    issue.alternatives.join(", ")
                )?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProfileErrors(pub Vec<String>);

impl std::fmt::Display for LanguageProfileErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Dil profili doğrulanamadı ({} hata):",
            self.0.len()
        )?;
        for message in &self.0 {
            write!(formatter, "\n- {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LanguageProfileErrors {}
