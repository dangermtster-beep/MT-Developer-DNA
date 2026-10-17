use crate::{
    CheckCategory, CheckResult, CheckStatus, CommandSpec, PerformanceMetrics, QualityRunKind,
    ToolInfo,
};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct CheckSelection {
    pub commands: Vec<CommandSpec>,
    pub unavailable: Vec<CheckResult>,
}

pub struct LanguageQualityAdapter;

impl LanguageQualityAdapter {
    pub fn discover(root: &Path, language: &str, kind: QualityRunKind) -> CheckSelection {
        let all = match language {
            "rust" => rust(root),
            "csharp" => csharp(root),
            "python" => python(root),
            "cpp" => cpp(root),
            "go" => go(root),
            "java" => java(root),
            "typescript" => typescript(root),
            _ => CheckSelection {
                commands: Vec::new(),
                unavailable: vec![unavailable(
                    "tests",
                    CheckCategory::Tests,
                    true,
                    "Desteklenen bir dil seçilmedi.",
                )],
            },
        };
        filter(all, kind)
    }
}

fn filter(mut selection: CheckSelection, kind: QualityRunKind) -> CheckSelection {
    let keep = |category: CheckCategory| match kind {
        QualityRunKind::Test => category == CheckCategory::Tests,
        QualityRunKind::Quality => matches!(
            category,
            CheckCategory::Tests | CheckCategory::Format | CheckCategory::Lint
        ),
        QualityRunKind::Performance => category == CheckCategory::Tests,
        QualityRunKind::Security => category == CheckCategory::Security,
        QualityRunKind::ReleaseCheck => true,
    };
    selection.commands.retain(|item| keep(item.category));
    selection.unavailable.retain(|item| keep(item.category));
    selection
}

fn rust(root: &Path) -> CheckSelection {
    if !root.join("Cargo.toml").is_file() {
        return missing_project("Cargo.toml");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "cargo",
                &["test", "--workspace"],
                true,
            ),
            command(
                "format",
                CheckCategory::Format,
                "cargo",
                &["fmt", "--all", "--", "--check"],
                false,
            ),
            command(
                "lint",
                CheckCategory::Lint,
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
                false,
            ),
            command(
                "build",
                CheckCategory::Build,
                "cargo",
                &["build", "--workspace", "--release"],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "cargo-audit",
                &[],
                true,
            ),
        ],
        unavailable: Vec::new(),
    }
}

fn csharp(root: &Path) -> CheckSelection {
    if !has_extension(root, "sln") && !has_extension(root, "slnx") && !has_extension(root, "csproj")
    {
        return missing_project(".sln/.slnx/.csproj");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "dotnet",
                &["test", "--no-restore"],
                true,
            ),
            command(
                "format",
                CheckCategory::Format,
                "dotnet",
                &["format", "--verify-no-changes", "--no-restore"],
                false,
            ),
            command(
                "build",
                CheckCategory::Build,
                "dotnet",
                &["build", "-c", "Release", "--no-restore"],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "dotnet",
                &["list", "package", "--vulnerable", "--include-transitive"],
                true,
            ),
        ],
        unavailable: Vec::new(),
    }
}

fn python(root: &Path) -> CheckSelection {
    if !root.join("pyproject.toml").is_file() && !root.join("requirements.txt").is_file() {
        return missing_project("pyproject.toml/requirements.txt");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "python",
                &["-m", "pytest"],
                true,
            ),
            command("lint", CheckCategory::Lint, "ruff", &["check", "."], false),
            command(
                "format",
                CheckCategory::Format,
                "ruff",
                &["format", "--check", "."],
                false,
            ),
            command(
                "build",
                CheckCategory::Build,
                "python",
                &["-m", "compileall", "-q", "."],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "pip-audit",
                &[],
                true,
            ),
        ],
        unavailable: Vec::new(),
    }
}

fn cpp(root: &Path) -> CheckSelection {
    if !root.join("CMakeLists.txt").is_file() {
        return missing_project("CMakeLists.txt");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "ctest",
                &["--test-dir", "build", "--output-on-failure"],
                true,
            ),
            command(
                "build",
                CheckCategory::Build,
                "cmake",
                &["--build", "build", "--config", "Release"],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "clang-tidy",
                &["--version"],
                true,
            ),
        ],
        unavailable: vec![unavailable(
            "format",
            CheckCategory::Format,
            false,
            "Kaynak dosya listesi olmadan güvenli clang-format çağrısı üretilemedi.",
        )],
    }
}

fn go(root: &Path) -> CheckSelection {
    if !root.join("go.mod").is_file() {
        return missing_project("go.mod");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "go",
                &["test", "./..."],
                true,
            ),
            command("lint", CheckCategory::Lint, "go", &["vet", "./..."], false),
            command(
                "build",
                CheckCategory::Build,
                "go",
                &["build", "./..."],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "govulncheck",
                &["./..."],
                true,
            ),
        ],
        unavailable: vec![unavailable(
            "format",
            CheckCategory::Format,
            false,
            "gofmt dosya listesi proje taramasından güvenli olarak seçilmedi.",
        )],
    }
}

fn java(root: &Path) -> CheckSelection {
    let (program, test_args, build_args) = if root.join("pom.xml").is_file() {
        ("mvn", vec!["test"], vec!["package", "-DskipTests"])
    } else if root.join("gradlew.bat").is_file() {
        ("gradlew.bat", vec!["test"], vec!["build", "-x", "test"])
    } else if root.join("gradlew").is_file() {
        ("./gradlew", vec!["test"], vec!["build", "-x", "test"])
    } else {
        return missing_project("pom.xml/Gradle wrapper");
    };
    CheckSelection {
        commands: vec![
            command("tests", CheckCategory::Tests, program, &test_args, true),
            command("build", CheckCategory::Build, program, &build_args, true),
        ],
        unavailable: vec![
            unavailable(
                "lint",
                CheckCategory::Lint,
                false,
                "Proje tarafından seçilmiş Java lint görevi bulunamadı.",
            ),
            unavailable(
                "security_critical",
                CheckCategory::Security,
                true,
                "Yerel dependency-check aracı yapılandırması bulunamadı.",
            ),
        ],
    }
}

fn typescript(root: &Path) -> CheckSelection {
    if !root.join("package.json").is_file() {
        return missing_project("package.json");
    }
    CheckSelection {
        commands: vec![
            command(
                "tests",
                CheckCategory::Tests,
                "npm",
                &["test", "--", "--runInBand"],
                true,
            ),
            command(
                "lint",
                CheckCategory::Lint,
                "npm",
                &["run", "lint", "--if-present"],
                false,
            ),
            command(
                "format",
                CheckCategory::Format,
                "npm",
                &["run", "format:check", "--if-present"],
                false,
            ),
            command(
                "build",
                CheckCategory::Build,
                "npm",
                &["run", "build", "--if-present"],
                true,
            ),
            command(
                "security_critical",
                CheckCategory::Security,
                "npm",
                &["audit", "--audit-level=critical", "--offline"],
                true,
            ),
        ],
        unavailable: Vec::new(),
    }
}

fn command(
    id: &str,
    category: CheckCategory,
    program: &str,
    args: &[&str],
    required: bool,
) -> CommandSpec {
    CommandSpec {
        id: id.to_owned(),
        category,
        program: program.to_owned(),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        required,
        mutates_source: false,
    }
}

fn unavailable(id: &str, category: CheckCategory, required: bool, reason: &str) -> CheckResult {
    CheckResult {
        id: id.to_owned(),
        category,
        status: CheckStatus::NotRun,
        required,
        exit_code: None,
        tool: ToolInfo {
            program: String::new(),
            available: false,
            version: None,
        },
        command: Vec::new(),
        summary: reason.to_owned(),
        evidence: reason.to_owned(),
        stdout_truncated: false,
        stderr_truncated: false,
        metrics: PerformanceMetrics::default(),
    }
}

fn missing_project(expected: &str) -> CheckSelection {
    CheckSelection {
        commands: Vec::new(),
        unavailable: vec![
            unavailable(
                "tests",
                CheckCategory::Tests,
                true,
                &format!("Proje yapılandırması bulunamadı: {expected}"),
            ),
            unavailable(
                "build",
                CheckCategory::Build,
                true,
                &format!("Proje yapılandırması bulunamadı: {expected}"),
            ),
            unavailable(
                "security_critical",
                CheckCategory::Security,
                true,
                &format!("Proje yapılandırması bulunamadı: {expected}"),
            ),
        ],
    }
}

fn has_extension(root: &Path, extension: &str) -> bool {
    std::fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some(extension))
}
