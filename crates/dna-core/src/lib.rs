mod error;
mod language;
mod optimization;
mod profile;
mod skill;
mod universal;
mod validation;
mod workflow;

pub use error::{LanguageProfileErrors, ProfileLoadError, ValidationErrors};
pub use language::{LanguageProfile, LANGUAGE_PROFILE_SCHEMA_VERSION};
pub use optimization::{
    validate_transition, ApprovalRecord, Confidence, FindingSource, FindingStatus, FixRisk,
    GateResultRecord, LineRange, OptimizationError, OptimizationFinding, OptimizationFixRecord,
    OptimizationMode, OptimizationReport, OptimizationSeverity, OPTIMIZATION_SCHEMA_VERSION,
};
pub use profile::{
    CommonStandards, Database, FeaturePreferences, LogLevel, Profile, ProgrammingLanguage, Theme,
    UiFramework, PROFILE_SCHEMA_VERSION,
};
pub use skill::{validate_skill_manifest, SkillManifest, SkillValidationError};
pub use universal::{resolve_language, LanguageSource, ResolvedLanguage};
pub use validation::{validate_profile, ValidationIssue};
pub use workflow::{
    ArchitectureAdvisor, ArchitectureRecommendation, BuildContract, BuildReport, DevelopmentPlan,
    DevelopmentPlanner, FindingSeverity, MemoryState, ModeRouter, PlanTask, Priority,
    ProjectAnalysis, Requirement, RequirementEngine, RequirementKind, RequirementSet,
    ReviewFinding, ReviewReport, RouteConfidence, RoutingDecision, TaskStatus, VerificationResult,
    VerificationStatus, WorkflowError, WorkflowMode, WORKFLOW_SCHEMA_VERSION,
};
