mod engine;
mod error;
mod model;
mod render;
mod scanner;
mod security;
mod store;
mod time;

pub use engine::{AnalysisLimits, MemoryBridge, MemoryContext, ProjectAnalyzer};
pub use error::MemoryError;
pub use model::{
    ApplyReport, AuditIssue, AuditReport, AuditSeverity, ChangeStatus, DecisionInput, InitRequest,
    ManagedFile, MemoryChange, MemoryManifest, MemoryPlan, MemorySnapshot, MemoryStandards,
    ProjectMetadata, ProviderId, ResumeReport, UpdateRequest, MEMORY_DIRECTORY,
    MEMORY_SCHEMA_VERSION,
};
pub use render::HandoffRenderer;
pub use scanner::{ProjectScan, ProjectScanner};
pub use security::SecretRedactor;
pub use store::{format_memory_plan, ConflictDetector, MemoryStore, SchemaValidator};

pub fn utc_timestamp() -> String {
    time::utc_now()
}

pub fn content_hash(contents: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in contents {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{content_hash, ProviderId};

    #[test]
    fn content_hash_is_deterministic() {
        assert_eq!(content_hash(b"abc"), "fnv1a64:e71fa2190541574b");
    }

    #[test]
    fn every_v08_provider_has_a_handoff_identity() {
        for provider in [
            ProviderId::Codex,
            ProviderId::Claude,
            ProviderId::Gemini,
            ProviderId::Antigravity,
            ProviderId::Cursor,
            ProviderId::Cline,
            ProviderId::OpenCode,
        ] {
            assert!(!provider.as_str().is_empty());
            assert!(provider.invocation().contains("mt-developer-dna"));
        }
    }
}
