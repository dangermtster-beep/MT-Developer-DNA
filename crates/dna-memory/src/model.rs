use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const MEMORY_SCHEMA_VERSION: &str = "0.3.0";
pub const MEMORY_DIRECTORY: &str = ".mt-dna";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Codex,
    Claude,
    Gemini,
    Antigravity,
    Cursor,
    Cline,
    OpenCode,
}

impl ProviderId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
            Self::Cursor => "cursor",
            Self::Cline => "cline",
            Self::OpenCode => "opencode",
        }
    }

    pub const fn invocation(self) -> &'static str {
        match self {
            Self::Codex => "$mt-developer-dna",
            Self::Claude
            | Self::Gemini
            | Self::Antigravity
            | Self::Cursor
            | Self::Cline
            | Self::OpenCode => "/mt-developer-dna",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectMetadata {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub language: String,
    pub project_type: String,
    pub platforms: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_provider: ProviderId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoryStandards {
    pub schema_version: String,
    pub language: String,
    pub architecture: Vec<String>,
    pub quality: Vec<String>,
    pub security: Vec<String>,
    pub performance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoryManifest {
    pub schema_version: String,
    pub generation: u64,
    pub updated_at: String,
    pub source_fingerprint: String,
    pub files: Vec<ManagedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManagedFile {
    pub path: String,
    pub schema_version: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub project: ProjectMetadata,
    pub standards: MemoryStandards,
    pub manifest: MemoryManifest,
    pub architecture: String,
    pub decisions: String,
    pub progress: String,
    pub handoff: String,
}

#[derive(Debug, Clone)]
pub struct InitRequest {
    pub project_id: String,
    pub name: String,
    pub language: String,
    pub project_type: String,
    pub platforms: Vec<String>,
    pub provider: ProviderId,
    pub architecture_rules: Vec<String>,
    pub quality_rules: Vec<String>,
    pub security_rules: Vec<String>,
    pub performance_rules: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DecisionInput {
    pub context: String,
    pub options: Vec<String>,
    pub decision: String,
    pub rationale: String,
    pub affected_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateRequest {
    pub provider: ProviderId,
    pub completed: Vec<String>,
    pub in_progress: Vec<String>,
    pub next: Vec<String>,
    pub blockers: Vec<String>,
    pub test_results: Vec<String>,
    pub decision: Option<DecisionInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Create,
    Update,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryChange {
    pub relative_path: String,
    pub status: ChangeStatus,
    pub diff: String,
}

#[derive(Debug, Clone)]
pub struct MemoryPlan {
    pub project_root: PathBuf,
    pub changes: Vec<MemoryChange>,
    pub(crate) expected_generation: Option<u64>,
    pub(crate) expected_hashes: BTreeMap<String, Option<String>>,
    pub(crate) outputs: BTreeMap<String, Vec<u8>>,
    pub(crate) initialize: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub changed: usize,
    pub unchanged: usize,
    pub backup_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub issues: Vec<AuditIssue>,
}

impl AuditReport {
    pub fn is_healthy(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeReport {
    pub summary: String,
    pub issues: Vec<AuditIssue>,
}
