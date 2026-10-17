use crate::MemoryError;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub struct SecretRedactor;

impl SecretRedactor {
    pub fn redact(value: &str) -> String {
        let mut inside_private_key = false;
        let mut output = Vec::new();
        for line in value.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("-----begin") && lower.contains("private key-----") {
                inside_private_key = true;
                output.push("[REDACTED PRIVATE KEY]".to_owned());
                continue;
            }
            if inside_private_key {
                if lower.contains("-----end") && lower.contains("private key-----") {
                    inside_private_key = false;
                }
                continue;
            }
            output.push(redact_line(line));
        }
        output.join("\n")
    }

    pub fn contains_suspected_secret(value: &str) -> bool {
        value.lines().any(|line| {
            if line.contains("[REDACTED]") {
                return false;
            }
            let lower = line.to_ascii_lowercase();
            [
                "-----begin private key-----",
                "api_key",
                "api-key",
                "secret_key",
                "access_token",
                "password=",
                "password:",
                "token=",
                "token:",
                "authorization: bearer ",
                "bearer eyj",
                "sk-proj-",
                "ghp_",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
        })
    }
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if let Some(index) = lower.find("authorization: bearer ") {
        let end = index + "authorization: bearer ".len();
        return format!("{}{}[REDACTED]", &line[..index], &line[index..end]);
    }
    for key in [
        "api_key",
        "api-key",
        "secret_key",
        "access_token",
        "password",
        "token",
    ] {
        if let Some(key_index) = lower.find(key) {
            let suffix = &line[key_index + key.len()..];
            if let Some(delimiter_offset) = suffix.find(['=', ':']) {
                let end = key_index + key.len() + delimiter_offset + 1;
                return format!("{}[REDACTED]", &line[..end]);
            }
        }
    }

    let mut redacted = line.to_owned();
    for marker in ["sk-proj-", "ghp_", "bearer eyj"] {
        loop {
            let lowercase = redacted.to_ascii_lowercase();
            let Some(start) = lowercase.find(marker) else {
                break;
            };
            let end = redacted[start..]
                .find(char::is_whitespace)
                .map(|offset| start + offset)
                .unwrap_or(redacted.len());
            redacted.replace_range(start..end, "[REDACTED]");
        }
    }
    redacted
}

pub(crate) fn validate_project_root(root: &Path) -> Result<PathBuf, MemoryError> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| MemoryError::InvalidProjectRoot(root.to_path_buf()))?;
    if !metadata.is_dir() {
        return Err(MemoryError::InvalidProjectRoot(root.to_path_buf()));
    }
    if is_link_or_reparse(&metadata) {
        return Err(MemoryError::LinkPath(root.to_path_buf()));
    }
    fs::canonicalize(root).map_err(|source| MemoryError::Io {
        operation: "Proje kökünü çözümleme",
        path: root.to_path_buf(),
        source,
    })
}

pub(crate) fn checked_path(root: &Path, relative: &str) -> Result<PathBuf, MemoryError> {
    let relative_path = Path::new(relative);
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || !relative_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(MemoryError::UnsafePath(relative.to_owned()));
    }
    let target = root.join(relative_path);
    ensure_no_links(root, &target)?;
    Ok(target)
}

pub(crate) fn ensure_no_links(root: &Path, target: &Path) -> Result<(), MemoryError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| MemoryError::UnsafePath(target.display().to_string()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current).map_err(|source| MemoryError::Io {
                operation: "Yol bilgisini okuma",
                path: current.clone(),
                source,
            })?;
            if is_link_or_reparse(&metadata) {
                return Err(MemoryError::LinkPath(current));
            }
        }
    }
    Ok(())
}

pub(crate) fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::SecretRedactor;

    #[test]
    fn secrets_are_redacted_without_removing_safe_context() {
        let value = "Dağıtım başarısız\nAPI_KEY = fake-secret\nanahtar sk-proj-example\n-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----";
        let redacted = SecretRedactor::redact(value);
        assert!(redacted.contains("Dağıtım başarısız"));
        assert!(!redacted.contains("fake-secret"));
        assert!(!redacted.contains("sk-proj-example"));
        assert!(!redacted.contains("abc123"));
        assert!(!SecretRedactor::contains_suspected_secret(&redacted));
    }
}
