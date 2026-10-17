use crate::ProgrammingLanguage;

/// Dil seçiminin hangi katmandan geldiğini kaybetmeden V0.8 önceliğini uygular.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSource {
    ExplicitRequest,
    ProjectMemory,
    UniversalDefault,
    ProviderFallback,
    AskUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedLanguage {
    pub language: ProgrammingLanguage,
    pub source: LanguageSource,
}

pub fn resolve_language(
    explicit: Option<ProgrammingLanguage>,
    project_memory: Option<ProgrammingLanguage>,
    universal_default: Option<ProgrammingLanguage>,
    provider_fallback: Option<ProgrammingLanguage>,
) -> ResolvedLanguage {
    for (candidate, source) in [
        (explicit, LanguageSource::ExplicitRequest),
        (project_memory, LanguageSource::ProjectMemory),
        (universal_default, LanguageSource::UniversalDefault),
        (provider_fallback, LanguageSource::ProviderFallback),
    ] {
        if let Some(language) = candidate {
            if !matches!(
                language,
                ProgrammingLanguage::Auto | ProgrammingLanguage::AskUser
            ) {
                return ResolvedLanguage { language, source };
            }
        }
    }
    ResolvedLanguage {
        language: ProgrammingLanguage::AskUser,
        source: LanguageSource::AskUser,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_language_has_highest_precedence() {
        let result = resolve_language(
            Some(ProgrammingLanguage::CSharp),
            Some(ProgrammingLanguage::Python),
            Some(ProgrammingLanguage::Rust),
            Some(ProgrammingLanguage::Go),
        );
        assert_eq!(result.language, ProgrammingLanguage::CSharp);
        assert_eq!(result.source, LanguageSource::ExplicitRequest);
    }

    #[test]
    fn missing_language_never_defaults_to_rust() {
        let result = resolve_language(None, None, None, None);
        assert_eq!(result.language, ProgrammingLanguage::AskUser);
        assert_eq!(result.source, LanguageSource::AskUser);
    }
}
