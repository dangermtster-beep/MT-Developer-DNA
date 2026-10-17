use crate::{CheckCategory, CheckResult, CheckStatus, PerformanceMetrics, QualityError, ToolInfo};
use std::fs;
use std::path::{Path, PathBuf};

pub struct LocalSecurityScanner;

impl LocalSecurityScanner {
    pub fn scan(root: &Path) -> Result<CheckResult, QualityError> {
        let root = fs::canonicalize(root).map_err(|source| QualityError::Io {
            operation: "Güvenlik tarama kökünü çözümleme",
            path: root.to_path_buf(),
            source,
        })?;
        let mut pending = vec![PathBuf::new()];
        let mut scanned = 0usize;
        let mut findings = Vec::new();
        let mut truncated = false;
        while let Some(relative_dir) = pending.pop() {
            let directory = root.join(&relative_dir);
            let entries = fs::read_dir(&directory).map_err(|source| QualityError::Io {
                operation: "Yerel güvenlik taraması",
                path: directory.clone(),
                source,
            })?;
            for entry in entries.flatten() {
                if scanned >= 512 {
                    truncated = true;
                    break;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if ignored(&name) {
                    continue;
                }
                let relative = relative_dir.join(&name);
                let metadata = entry.metadata().map_err(|source| QualityError::Io {
                    operation: "Güvenlik girdisi bilgisini okuma",
                    path: entry.path(),
                    source,
                })?;
                if metadata.is_dir() {
                    pending.push(relative);
                    continue;
                }
                scanned += 1;
                if metadata.len() > 1024 * 1024 || !is_text_candidate(&relative) {
                    continue;
                }
                let contents = fs::read_to_string(entry.path()).unwrap_or_default();
                if has_critical_pattern(&contents) {
                    findings.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let status = if findings.is_empty() {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        };
        let summary = if findings.is_empty() {
            format!("{scanned} sınırlı metin dosyasında kritik sır deseni bulunmadı.")
        } else {
            format!("{} dosyada kritik sır deseni bulundu.", findings.len())
        };
        let mut evidence = if findings.is_empty() {
            "Yerel desen taraması tamamlandı; bu bir güvenlik garantisi değildir.".to_owned()
        } else {
            format!("Etkilenen göreli yollar: {}", findings.join(", "))
        };
        if truncated {
            evidence.push_str(" Tarama 512 dosya sınırında kısmi kaldı.");
        }
        Ok(CheckResult {
            id: "security_local".to_owned(),
            category: CheckCategory::Security,
            status,
            required: true,
            exit_code: None,
            tool: ToolInfo {
                program: "dna-quality-local-scan".to_owned(),
                available: true,
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            },
            command: Vec::new(),
            summary,
            evidence,
            stdout_truncated: truncated,
            stderr_truncated: false,
            metrics: PerformanceMetrics::default(),
        })
    }
}

fn ignored(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | ".mt-dna"
            | ".tooling"
            | "target"
            | "node_modules"
            | ".venv"
            | "venv"
            | "bin"
            | "obj"
            | "dist"
            | "build"
    ) || name.eq_ignore_ascii_case(".env")
        || name.starts_with(".env.")
}

fn is_text_candidate(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(
            "rs" | "cs"
                | "py"
                | "cpp"
                | "h"
                | "go"
                | "java"
                | "ts"
                | "tsx"
                | "js"
                | "json"
                | "toml"
                | "yaml"
                | "yml"
                | "md"
        )
    )
}

fn has_critical_pattern(contents: &str) -> bool {
    contents.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower
            .trim_start()
            .starts_with("-----begin private key-----")
            || has_long_prefixed_token(&lower, "sk-proj-")
            || has_long_prefixed_token(&lower, "ghp_")
            || has_long_assignment(&lower, "api_key")
            || has_long_assignment(&lower, "access_token")
    })
}

fn has_long_prefixed_token(line: &str, marker: &str) -> bool {
    line.find(marker).is_some_and(|start| {
        line[start + marker.len()..]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
            .count()
            >= 20
    })
}

fn has_long_assignment(line: &str, key: &str) -> bool {
    let Some(start) = line.find(key) else {
        return false;
    };
    let suffix = &line[start + key.len()..];
    // Rust/C/C++ type annotations may contain `:` before the actual assignment.
    // Prefer `=` and only fall back to `:` for JSON/YAML-style entries.
    let Some(delimiter) = suffix.find('=').or_else(|| suffix.find(':')) else {
        return false;
    };
    suffix[delimiter + 1..]
        .chars()
        .skip_while(|character| {
            character.is_ascii_whitespace() || matches!(character, '"' | '\'' | '\\')
        })
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .count()
        >= 20
        && !suffix.contains("[redacted]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn critical_secret_is_reported_without_copying_its_value() {
        let root = std::env::temp_dir().join(format!(
            "dna-security-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let sample_secret = ["A1b2C3d4E5f6G7h8", "I9j0K1l2M3n4"].concat();
        fs::write(
            root.join("sample.rs"),
            format!("const API_KEY: &str = \"{sample_secret}\";"),
        )
        .unwrap();
        let report = LocalSecurityScanner::scan(&root).unwrap();
        assert_eq!(report.status, CheckStatus::Fail);
        assert!(!report.evidence.contains(&sample_secret));
        fs::remove_dir_all(root).unwrap();
    }
}
