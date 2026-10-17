mod adapter;
mod baseline;
mod engine;
mod gate;
mod model;
mod report;
mod runner;
mod security;

pub use adapter::{CheckSelection, LanguageQualityAdapter};
pub use baseline::{BaselineComparison, BaselineStore, PerformanceBaseline};
pub use engine::QualityEngine;
pub use gate::GateEvaluator;
pub use model::{
    CheckCategory, CheckResult, CheckStatus, CommandSpec, EnvironmentInfo, GateStatus,
    MetricCapability, PerformanceMetrics, PerformancePolicy, QualityError, QualityPolicy,
    QualityReport, QualityRunKind, ToolInfo, QUALITY_SCHEMA_VERSION,
};
pub use report::QualityReporter;
pub use runner::{CancellationToken, CheckRunner, RunnerLimits};
pub use security::LocalSecurityScanner;
