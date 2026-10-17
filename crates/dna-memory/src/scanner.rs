use crate::security::checked_path;
use crate::MemoryError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectScan {
    pub evidence_paths: Vec<String>,
    pub fingerprint: String,
}

pub struct ProjectScanner;

impl ProjectScanner {
    pub fn scan(root: &Path) -> Result<ProjectScan, MemoryError> {
        const MAX_ENTRIES: usize = 256;
        let mut evidence = Vec::new();
        let mut fingerprints = Vec::new();
        let mut pending = vec![PathBuf::new()];
        let mut visited = 0;
        while let Some(relative_dir) = pending.pop() {
            let directory = root.join(&relative_dir);
            let entries = fs::read_dir(&directory).map_err(|source| MemoryError::Io {
                operation: "Proje klasörünü tarama",
                path: directory.clone(),
                source,
            })?;
            for entry in entries {
                if visited >= MAX_ENTRIES {
                    break;
                }
                let entry = entry.map_err(|source| MemoryError::Io {
                    operation: "Proje girdisini okuma",
                    path: directory.clone(),
                    source,
                })?;
                let name = entry.file_name().to_string_lossy().to_string();
                if should_ignore(&name) {
                    continue;
                }
                let relative = relative_dir.join(&name);
                let relative_text = relative.to_string_lossy().replace('\\', "/");
                let path = checked_path(root, &relative_text)?;
                let metadata = fs::metadata(&path).map_err(|source| MemoryError::Io {
                    operation: "Proje girdisi bilgisini okuma",
                    path: path.clone(),
                    source,
                })?;
                visited += 1;
                if relative_dir.as_os_str().is_empty() {
                    let kind = if metadata.is_dir() { "dir" } else { "file" };
                    evidence.push(format!("{kind}:{name}:{}", metadata.len()));
                }
                if metadata.is_dir() {
                    pending.push(relative);
                } else if is_source_evidence(&path) {
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|value| value.as_nanos())
                        .unwrap_or_default();
                    fingerprints.push(format!("{relative_text}:{}:{modified}", metadata.len()));
                }
            }
        }
        evidence.sort();
        fingerprints.sort();
        let fingerprint = crate::content_hash(fingerprints.join("\n").as_bytes());
        let evidence_paths = evidence
            .iter()
            .map(|item| {
                let mut parts = item.splitn(3, ':');
                let kind = parts.next().unwrap_or("file");
                let name = parts.next().unwrap_or_default();
                format!("{name} ({kind})")
            })
            .collect();
        Ok(ProjectScan {
            evidence_paths,
            fingerprint,
        })
    }
}

fn is_source_evidence(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if matches!(
        name,
        "Cargo.toml" | "Cargo.lock" | "package.json" | "pyproject.toml" | "go.mod" | "pom.xml"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some(
            "rs" | "cs"
                | "csproj"
                | "py"
                | "cpp"
                | "cc"
                | "cxx"
                | "h"
                | "hpp"
                | "go"
                | "java"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
        )
    )
}

fn should_ignore(name: &str) -> bool {
    name.eq_ignore_ascii_case(".env")
        || name.starts_with(".env.")
        || matches!(name, ".git" | ".mt-dna" | "target" | "node_modules")
}
