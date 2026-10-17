#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SkillValidationError {
    #[error("SKILL.md YAML frontmatter ile başlamalıdır")]
    MissingFrontmatter,
    #[error("SKILL.md frontmatter kapanışı bulunamadı")]
    UnclosedFrontmatter,
    #[error("SKILL.md frontmatter alanı eksik: {0}")]
    MissingField(&'static str),
    #[error("Skill adı beklenen değerle eşleşmiyor: beklenen '{expected}', bulunan '{actual}'")]
    UnexpectedName { expected: String, actual: String },
    #[error("Skill adı yalnızca küçük ASCII harf, rakam ve tire içerebilir")]
    InvalidName,
    #[error("Skill açıklaması 20-1024 karakter arasında olmalıdır")]
    InvalidDescription,
    #[error("SKILL.md yönerge gövdesi boş olamaz")]
    EmptyBody,
    #[error("SKILL.md tamamlanmamış şablon işareti içeriyor: {0}")]
    Placeholder(String),
}

pub fn validate_skill_manifest(
    source: &str,
    expected_name: &str,
) -> Result<SkillManifest, SkillValidationError> {
    let normalized = source.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return Err(SkillValidationError::MissingFrontmatter);
    }

    let mut name = None;
    let mut description = None;
    let mut closed = false;
    let mut body = Vec::new();
    for line in &mut lines {
        if line == "---" {
            closed = true;
            body.extend(lines);
            break;
        }
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').to_owned());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    if !closed {
        return Err(SkillValidationError::UnclosedFrontmatter);
    }

    let name = name.ok_or(SkillValidationError::MissingField("name"))?;
    let description = description.ok_or(SkillValidationError::MissingField("description"))?;
    if name != expected_name {
        return Err(SkillValidationError::UnexpectedName {
            expected: expected_name.to_owned(),
            actual: name,
        });
    }
    if !is_valid_skill_name(&name) {
        return Err(SkillValidationError::InvalidName);
    }
    if !(20..=1024).contains(&description.chars().count()) {
        return Err(SkillValidationError::InvalidDescription);
    }
    let body = body.join("\n");
    if body.trim().is_empty() {
        return Err(SkillValidationError::EmptyBody);
    }
    for marker in ["TODO", "TBD", "REPLACE_ME", "[placeholder]"] {
        if body.contains(marker) {
            return Err(SkillValidationError::Placeholder(marker.to_owned()));
        }
    }

    Ok(SkillManifest { name, description })
}

fn is_valid_skill_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
