mod adapters;

use dna_core::{
    BuildContract, DevelopmentPlan, FindingSeverity, Profile, ReviewReport, TaskStatus,
    ValidationErrors, WorkflowError,
};
use std::fmt::Write as _;
use std::path::{Component, Path};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Codex,
    Claude,
    Gemini,
    Antigravity,
    Cursor,
    Cline,
    OpenCode,
}

impl Provider {
    pub const ALL: [Self; 7] = [
        Self::Codex,
        Self::Claude,
        Self::Gemini,
        Self::Antigravity,
        Self::Cursor,
        Self::Cline,
        Self::OpenCode,
    ];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex CLI",
            Self::Claude => "Claude Code",
            Self::Gemini => "Gemini CLI",
            Self::Antigravity => "Google Antigravity",
            Self::Cursor => "Cursor",
            Self::Cline => "Cline",
            Self::OpenCode => "OpenCode",
        }
    }

    pub const fn directory_name(self) -> &'static str {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityState {
    Supported,
    Unsupported,
    Unverified,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub provider: Provider,
    pub id: &'static str,
    pub display_name: &'static str,
    pub executable: Option<&'static str>,
    pub primary_path: &'static str,
    pub project_skill_path: &'static str,
    pub global_skill_path: &'static str,
    pub invocation: &'static str,
    pub skill: CapabilityState,
    pub rules: CapabilityState,
    pub slash_command: CapabilityState,
    pub project_install: CapabilityState,
    pub global_install: CapabilityState,
    pub note: &'static str,
}

pub const PROVIDER_REGISTRY: [ProviderDescriptor; 7] = [
    ProviderDescriptor { provider: Provider::Codex, id: "codex", display_name: "Codex CLI", executable: Some("codex"), primary_path: "AGENTS.md", project_skill_path: ".agents/skills/mt-developer-dna/SKILL.md", global_skill_path: ".agents/skills/mt-developer-dna/SKILL.md", invocation: "$mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Unsupported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Codex skill'i `.agents/skills/` altında keşfeder. Açık çağrı `$mt-developer-dna` biçimindedir; desteklenmeyen `/mt-developer-dna` çağrısını varmış gibi gösterme." },
    ProviderDescriptor { provider: Provider::Claude, id: "claude", display_name: "Claude Code", executable: Some("claude"), primary_path: "CLAUDE.md", project_skill_path: ".claude/skills/mt-developer-dna/SKILL.md", global_skill_path: ".claude/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Claude Code skill'i `.claude/skills/` altında keşfeder ve `/mt-developer-dna` ile doğrudan çağırır." },
    ProviderDescriptor { provider: Provider::Gemini, id: "gemini", display_name: "Gemini CLI", executable: Some("gemini"), primary_path: "GEMINI.md", project_skill_path: ".gemini/skills/mt-developer-dna/SKILL.md", global_skill_path: ".gemini/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Gemini CLI skill'i `.gemini/skills/` altında, slash komutunu `.gemini/commands/mt-developer-dna.toml` yolunda kullanır. Kurulu CLI sürümünde `/skills list` ve `/commands list` ile doğrula." },
    ProviderDescriptor { provider: Provider::Antigravity, id: "antigravity", display_name: "Google Antigravity", executable: Some("agy"), primary_path: ".agents/mt-developer-dna.md", project_skill_path: ".agents/skills/mt-developer-dna/SKILL.md", global_skill_path: ".gemini/config/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Antigravity IDE `.gemini/config/skills/`, Antigravity CLI `.gemini/antigravity-cli/skills/` kullanır; ikisi de workspace `.agents/skills/` ve `/mt-developer-dna` çağrısını destekler." },
    ProviderDescriptor { provider: Provider::Cursor, id: "cursor", display_name: "Cursor", executable: Some("cursor"), primary_path: ".cursor/rules/mt-developer-dna.mdc", project_skill_path: ".cursor/skills/mt-developer-dna/SKILL.md", global_skill_path: ".cursor/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Cursor skill'leri slash ile çağırır; canlı IDE kabul testi ayrıca gerekir." },
    ProviderDescriptor { provider: Provider::Cline, id: "cline", display_name: "Cline", executable: None, primary_path: ".clinerules/mt-developer-dna.md", project_skill_path: ".cline/skills/mt-developer-dna/SKILL.md", global_skill_path: ".cline/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Supported, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Cline VS Code extension kimliği `saoudrizwan.claude-dev` olarak doğrulanır. Project skill `.cline/skills/`, global skill `~/.cline/skills/` altındadır; etkin profilde enable durumu canlı IDE doğrulaması gerektirir." },
    ProviderDescriptor { provider: Provider::OpenCode, id: "opencode", display_name: "OpenCode", executable: Some("opencode"), primary_path: ".opencode/mt-developer-dna.md", project_skill_path: ".opencode/skills/mt-developer-dna/SKILL.md", global_skill_path: ".config/opencode/skills/mt-developer-dna/SKILL.md", invocation: "/mt-developer-dna", skill: CapabilityState::Supported, rules: CapabilityState::Unverified, slash_command: CapabilityState::Supported, project_install: CapabilityState::Supported, global_install: CapabilityState::Supported, note: "Windows discovery `opencode.exe`, npm `opencode.cmd` ve `opencode.ps1` çalıştırıcılarını destekler. Project skill `.opencode/skills/`, global skill `~/.config/opencode/skills/` altındadır; `metadata.opencode/slash` ile `/mt-developer-dna` görünür olur." },
];

pub fn provider_descriptor(provider: Provider) -> &'static ProviderDescriptor {
    PROVIDER_REGISTRY
        .iter()
        .find(|entry| entry.provider == provider)
        .expect("provider registry eksiksiz olmalı")
}

/// Proje skill klasöründeki bir dosyayı (SKILL.md veya references/...) hedef skill
/// klasörüne taşır. Dosya skill klasörüne ait değilse `None` döner.
pub fn remap_skill_file(
    project_path: &str,
    project_skill_path: &str,
    target_skill_path: &str,
) -> Option<String> {
    if project_path == project_skill_path {
        return Some(target_skill_path.to_owned());
    }
    let project_dir = project_skill_path.strip_suffix("SKILL.md")?;
    let target_dir = target_skill_path.strip_suffix("SKILL.md")?;
    project_path
        .strip_prefix(project_dir)
        .filter(|rest| !rest.is_empty())
        .map(|rest| format!("{target_dir}{rest}"))
}

pub fn deployment_relative_path(
    provider: Provider,
    project_path: &str,
    global: bool,
) -> Option<String> {
    if !global {
        return Some(project_path.to_owned());
    }
    let descriptor = provider_descriptor(provider);
    if let Some(path) = remap_skill_file(
        project_path,
        descriptor.project_skill_path,
        descriptor.global_skill_path,
    ) {
        return Some(path);
    }
    if provider == Provider::Gemini && project_path == ".gemini/commands/mt-developer-dna.toml" {
        return Some(project_path.to_owned());
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFile {
    pub relative_path: String,
    pub contents: String,
}

impl RenderedFile {
    pub fn has_safe_relative_path(&self) -> bool {
        let path = Path::new(&self.relative_path);
        !path.as_os_str().is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPackage {
    pub provider: Provider,
    pub files: Vec<RenderedFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
    #[error(transparent)]
    Workflow(#[from] WorkflowError),
    #[error("Renderer güvenli olmayan bir göreli yol üretti: {0}")]
    UnsafeOutputPath(String),
}

pub struct ReportRenderer;

impl ReportRenderer {
    pub fn development_plan(plan: &DevelopmentPlan) -> Result<String, RenderError> {
        plan.validate()?;
        let mut output = format!(
            "# Geliştirme Planı — {}\n\n- Mod: PLAN\n- Plan: `{}`\n- Dil: {}\n- Proje türü: {}\n- Kaynak parmak izi: `{}`\n\n",
            plan.project_id,
            plan.plan_id,
            plan.language.as_deref().unwrap_or("kullanıcı seçimi bekleniyor"),
            plan.project_type,
            plan.source_fingerprint
        );
        output.push_str("## Gereksinimler\n\n");
        for requirement in &plan.requirements {
            writeln!(
                output,
                "- **{}** — {} (Kabul: {})",
                requirement.id, requirement.text, requirement.acceptance
            )
            .unwrap();
        }
        output.push_str("\n## Mimari\n\n");
        writeln!(output, "- Stil: {}", plan.architecture.style).unwrap();
        for component in &plan.architecture.components {
            writeln!(output, "- Bileşen: {component}").unwrap();
        }
        output.push_str("\n## Görevler\n\n");
        for task in &plan.tasks {
            let status = match task.status {
                TaskStatus::Pending => "bekliyor",
                TaskStatus::InProgress => "sürüyor",
                TaskStatus::Completed => "tamamlandı",
                TaskStatus::Blocked => "engelli",
            };
            let dependencies = if task.depends_on.is_empty() {
                "yok".to_owned()
            } else {
                task.depends_on.join(", ")
            };
            writeln!(
                output,
                "- **{}** [{}] {} — bağımlılık: {}",
                task.id, status, task.title, dependencies
            )
            .unwrap();
        }
        output.push_str("\n## Doğrulama\n\n");
        if plan.verification_commands.is_empty() {
            output.push_str("- Dil seçilmeden doğrulama komutları kesinleştirilemez.\n");
        } else {
            for command in &plan.verification_commands {
                writeln!(output, "- `{command}`").unwrap();
            }
        }
        if !plan.open_questions.is_empty() {
            output.push_str("\n## Açık sorular\n\n");
            for question in &plan.open_questions {
                writeln!(output, "- {question}").unwrap();
            }
        }
        Ok(output)
    }

    pub fn build_contract(contract: &BuildContract) -> Result<String, RenderError> {
        contract.plan.validate()?;
        let state = if contract.approved {
            "ONAYLI — provider ajanı plan kapsamını uygulayabilir"
        } else {
            "ONAY BEKLİYOR — kaynak dosyalar değiştirilmez"
        };
        Ok(format!(
            "# BUILD Sözleşmesi\n\n- Plan: `{}`\n- Durum: {}\n- Ajan uygulaması gerekli: {}\n- Rust çekirdeği kaynak değiştirdi: {}\n- İzin sınırı: {}\n\n> Bu çıktı bir başarı veya test PASS iddiası değildir. Gerçek uygulama ve komut kanıtı provider ajanı tarafından kaydedilmelidir.\n",
            contract.plan.plan_id,
            state,
            contract.agent_execution_required,
            contract.source_changes_performed,
            contract.permission_boundary
        ))
    }

    pub fn review_report(report: &ReviewReport) -> Result<String, RenderError> {
        report.validate()?;
        let mut output = format!(
            "# REVIEW Raporu — {}\n\n- Mod: REVIEW (salt-okunur)\n- Dil: {}\n- Taranan dosya: {}\n- Kısmi tarama: {}\n- Kapsam: {}\n\n",
            report.project_id,
            report.language.as_deref().unwrap_or("belirlenmedi"),
            report.files_scanned,
            if report.truncated { "evet" } else { "hayır" },
            report.scope
        );
        output.push_str("## Bulgular\n\n");
        if report.findings.is_empty() {
            output.push_str("- Yapısal taramada bulgu yok. Bu, semantik kodun kusursuz olduğu anlamına gelmez.\n");
        }
        for finding in &report.findings {
            let severity = match finding.severity {
                FindingSeverity::Critical => "KRİTİK",
                FindingSeverity::High => "YÜKSEK",
                FindingSeverity::Medium => "ORTA",
                FindingSeverity::Low => "DÜŞÜK",
                FindingSeverity::Info => "BİLGİ",
            };
            let location = match (&finding.file, finding.line) {
                (Some(file), Some(line)) => format!("{file}:{line}"),
                (Some(file), None) => file.clone(),
                _ => "proje geneli".to_owned(),
            };
            writeln!(
                output,
                "### {} [{}] {}\n\n- Konum: `{}`\n- Kanıt: {}\n- Etki: {}\n- Öneri: {}\n",
                finding.id,
                severity,
                finding.title,
                location,
                finding.evidence,
                finding.impact,
                finding.recommendation
            )
            .unwrap();
        }
        if !report.assumptions.is_empty() {
            output.push_str("## Varsayımlar ve sınırlar\n\n");
            for assumption in &report.assumptions {
                writeln!(output, "- {assumption}").unwrap();
            }
        }
        Ok(output)
    }
}

/// Skill keşfinde kullanılan açıklama. Profil dilinden bağımsızdır; dil seçimi
/// skill gövdesinde ve proje hafızasında taşınır.
pub const SKILL_DESCRIPTION: &str = "Plan, implement, review, test, secure, optimize, and prepare software projects for release in any programming language, preserving the user's chosen language, Turkish user interface, and existing architecture.";

/// Skill ile birlikte dağıtılan referans dosyaları: (dosya, ne zaman okunur, içerik).
/// Tek kaynak `mt-developer-dna/references/` klasörüdür; içerik derleme anında gömülür.
pub const SKILL_REFERENCES: [(&str, &str, &str); 13] = [
    (
        "modes.md",
        "PLAN/BUILD/REVIEW yönlendirmesi, plan, uygulama ve inceleme öncesinde",
        include_str!("../../../mt-developer-dna/references/modes.md"),
    ),
    (
        "standards.md",
        "mimari, kalite, güvenlik ve performans standartları",
        include_str!("../../../mt-developer-dna/references/standards.md"),
    ),
    (
        "quality.md",
        "`test`, `quality`, `performance`, `security`, `release-check` komutlarından önce",
        include_str!("../../../mt-developer-dna/references/quality.md"),
    ),
    (
        "optimization.md",
        "`optimize`, `fix`, `optimization-report` ve rollback öncesinde",
        include_str!("../../../mt-developer-dna/references/optimization.md"),
    ),
    (
        "project-memory.md",
        "`.mt-dna/` hafıza işlemleri: `init`, `resume`, `update`, `handoff`, `memory check/repair`",
        include_str!("../../../mt-developer-dna/references/project-memory.md"),
    ),
    (
        "deployment.md",
        "provider kurulum, senkronizasyon ve Doctor istekleri",
        include_str!("../../../mt-developer-dna/references/deployment.md"),
    ),
    (
        "languages/cpp.json",
        "C++ derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/cpp.json"),
    ),
    (
        "languages/csharp.json",
        "C# derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/csharp.json"),
    ),
    (
        "languages/go.json",
        "Go derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/go.json"),
    ),
    (
        "languages/java.json",
        "Java derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/java.json"),
    ),
    (
        "languages/python.json",
        "Python derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/python.json"),
    ),
    (
        "languages/rust.json",
        "Rust derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/rust.json"),
    ),
    (
        "languages/typescript.json",
        "TypeScript derleme/test/release komutları ve ilkeleri",
        include_str!("../../../mt-developer-dna/references/languages/typescript.json"),
    ),
];

pub fn render_package(
    profile: &Profile,
    provider: Provider,
) -> Result<ProviderPackage, RenderError> {
    profile.validate()?;
    let adapter = adapters::adapter_for(provider);
    let skill_path = adapter.skill_path();
    let skill_dir = skill_path
        .strip_suffix("/SKILL.md")
        .expect("skill yolu SKILL.md ile bitmelidir");
    let mut files = vec![
        RenderedFile {
            relative_path: adapter.primary_path().to_owned(),
            contents: render_primary_document(profile, adapter),
        },
        RenderedFile {
            relative_path: skill_path.to_owned(),
            contents: render_skill(profile, adapter),
        },
    ];
    files.extend(
        SKILL_REFERENCES
            .iter()
            .map(|(name, _, contents)| RenderedFile {
                relative_path: format!("{skill_dir}/references/{name}"),
                contents: (*contents).to_owned(),
            }),
    );
    files.extend(adapter.additional_files());

    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if let Some(file) = files.iter().find(|file| !file.has_safe_relative_path()) {
        return Err(RenderError::UnsafeOutputPath(file.relative_path.clone()));
    }

    Ok(ProviderPackage {
        provider,
        files,
        warnings: adapter.warnings(),
    })
}

fn render_primary_document(profile: &Profile, adapter: &dyn adapters::ProviderAdapter) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "# MT Developer DNA — {}",
        adapter.provider().display_name()
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "> Bu dosya MT Developer DNA profilinden deterministik olarak üretildi."
    )
    .unwrap();
    writeln!(output).unwrap();
    render_profile_summary(&mut output, profile);
    render_modes(&mut output, adapter);
    render_quality_system(&mut output, adapter);
    render_optimization_system(&mut output, adapter);
    render_deployment_system(&mut output, adapter);
    render_required_workflow(&mut output, profile);
    render_project_memory(&mut output, adapter);
    render_standards(&mut output, profile);
    render_provider_notes(&mut output, adapter);
    output
}

fn render_skill(profile: &Profile, adapter: &dyn adapters::ProviderAdapter) -> String {
    let mut output = String::new();
    writeln!(output, "---").unwrap();
    writeln!(output, "name: mt-developer-dna").unwrap();
    writeln!(output, "description: {SKILL_DESCRIPTION}").unwrap();
    if adapter.provider() == Provider::OpenCode {
        writeln!(output, "metadata:").unwrap();
        writeln!(output, "  opencode/slash: \"true\"").unwrap();
    }
    writeln!(output, "---").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "# MT Developer DNA").unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "Bu skill {} için üretilmiştir. Çağırma biçimi: `{}`.",
        adapter.provider().display_name(),
        adapter.invocation()
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(output, "## İş akışı").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "1. Yeni projede gereksinimleri; mevcut projede önce kodu, bağımlılıkları ve testleri incele.").unwrap();
    writeln!(output, "2. Kritik eksikleri belirle. Dil belirtilmemişse uygun alternatifler sun; seçimi kullanıcıya bırak.").unwrap();
    writeln!(output, "3. Orantılı mimari ve geliştirme planı oluştur; gerekli onayları almadan kapsamı genişletme.").unwrap();
    writeln!(
        output,
        "4. Çalışan kodu gereksiz yere yeniden yazmadan en küçük bütünlüklü değişikliği uygula."
    )
    .unwrap();
    writeln!(
        output,
        "5. İlgili testleri, güvenlik kontrollerini ve release derlemesini çalıştır."
    )
    .unwrap();
    writeln!(
        output,
        "6. Sonucu; değişiklikleri, gerçek doğrulama kanıtını ve sınırlamalarıyla raporla."
    )
    .unwrap();
    writeln!(output).unwrap();
    render_reference_index(&mut output);
    render_modes(&mut output, adapter);
    render_quality_system(&mut output, adapter);
    render_optimization_system(&mut output, adapter);
    render_deployment_system(&mut output, adapter);
    render_required_workflow(&mut output, profile);
    render_project_memory(&mut output, adapter);
    render_standards(&mut output, profile);
    output
}

fn render_reference_index(output: &mut String) {
    writeln!(output, "## Referanslar").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "Ayrıntılı sözleşmeler bu skill klasöründeki `references/` altındadır. Yalnız etkin işlem için gereken dosyayı oku:").unwrap();
    writeln!(output).unwrap();
    for (name, purpose, _) in SKILL_REFERENCES {
        writeln!(
            output,
            "- [`references/{name}`](references/{name}): {purpose}"
        )
        .unwrap();
    }
    writeln!(output).unwrap();
}

fn render_modes(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(output, "## PLAN, BUILD ve REVIEW modları").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- `PLAN`: projeyi sınırlı ve ignore-aware tara; gereksinim, mimari, görev bağımlılıkları ve doğrulama komutları üret. Varsayılan olarak kaynak veya hafıza yazma.").unwrap();
    writeln!(output, "- `BUILD`: yalnızca kullanıcı uygulamayı istediğinde veya onayladığında devreye gir. Onaylı plan kapsamındaki en küçük değişikliği provider izin modeliyle uygula; Rust çekirdeğinin kendi başına kaynak yazdığını ya da komut çalıştırdığını iddia etme.").unwrap();
    writeln!(output, "- `REVIEW`: varsayılan olarak salt-okunurdur. Her bulguda önem, dosya/konum, kanıt, etki ve düzeltme önerisi ver; kısmi taramayı ve çalıştırılmayan testleri açıkça belirt.").unwrap();
    writeln!(
        output,
        "- Niyet belirsizse güvenli REVIEW modunda kal ve uygulama öncesinde doğrulama iste."
    )
    .unwrap();
    writeln!(output, "- Proje hafızasındaki dil ve onaylı plan provider değişse de korunur. Plan/review kaydı için diff göster ve açık onay al.").unwrap();
    writeln!(
        output,
        "- Bu provider için çağrılar: `{0} plan`, `{0} build`, `{0} review`.",
        adapter.invocation()
    )
    .unwrap();
    writeln!(output).unwrap();
}

fn render_quality_system(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(output, "## V0.5 kalite ve ölçüm sistemi").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- `test`, `quality`, `performance`, `security` ve `release-check` kontrollerini talep üzerine çalıştır; daemon, watcher veya sürekli polling başlatma.").unwrap();
    writeln!(output, "- Komutları shell metni olarak birleştirme. Dil adapter'ının program/argüman listesini, proje çalışma dizinini, timeout'u ve sınırlı çıktıyı kullan.").unwrap();
    writeln!(output, "- Eksik araç veya çalıştırılamayan zorunlu kontrol `NOT_RUN`; desteklenmeyen metrik `UNAVAILABLE`; zorunlu `NOT_RUN/UNAVAILABLE` release sonucunu `BLOCKED` yapar.").unwrap();
    writeln!(output, "- Format/lint sırasında auto-fix çalıştırma. Kaynak değişikliği yalnızca BUILD izniyle ayrı bir adımdır.").unwrap();
    writeln!(output, "- Performansı warm-up/tekrar, ortam ve ölçüm yöntemiyle raporla; karşılaştırılamayan baseline için regresyon yüzdesi üretme. Temiz güvenlik taramasını garanti sayma.").unwrap();
    writeln!(output, "- Rapor veya baseline hafızaya yazılacaksa `.mt-dna` diff'ini göster ve açık onay al; eski raporları varsayılan olarak silme.").unwrap();
    writeln!(output, "- Bu provider için hedef çağrılar: `{0} test`, `{0} quality`, `{0} performance`, `{0} security`, `{0} release-check`.", adapter.invocation()).unwrap();
    writeln!(output).unwrap();
}

fn render_optimization_system(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(output, "## V0.6 kontrollü optimizasyon ve Auto-Fix").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- `optimize --mode analyze` varsayılan ve salt-okunurdur. `preview` yalnız dosya listesi, unified diff, risk, test ve rollback planı üretir.").unwrap();
    writeln!(output, "- `apply` yalnız açık kullanıcı onayı, düşük riskli doğrulanabilir yama, uygulama öncesi başarılı zorunlu kalite kapısı ve değişmemiş hedef hash'i ile çalışır. Güvenlik bulgusu kendiliğinden yetki vermez.").unwrap();
    writeln!(output, "- `rollback --fix-id` yalnız ilgili V0.6 işlem kaydındaki dosyaları geri alır. Sonraki kullanıcı değişikliği veya hash uyuşmazlığında dur; toplu restore, `reset --hard` veya `clean` kullanma.").unwrap();
    writeln!(output, "- Statik performans sezgisini ölçülmüş kazanç gibi sunma. Önce/sonra karşılaştırması aynı ortam ve iş yükünde warm-up ve tekrarlarla yapılmalıdır.").unwrap();
    writeln!(output, "- İşlem raporunu `.mt-dna/reports/optimization/`, geri alma artifact'ini `.mt-dna/transactions/` altında tut; token veya hassas içerik rapora kopyalama.").unwrap();
    writeln!(output, "- Bu provider için hedef çağrılar: `{0} optimize --mode analyze`, `{0} optimize --mode preview`, `{0} optimize --mode apply`, `{0} optimize --mode rollback --fix-id <id>`, `{0} fix --finding-id <id>` ve `{0} optimization-report`.", adapter.invocation()).unwrap();
    writeln!(output).unwrap();
}

fn render_deployment_system(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(
        output,
        "## V0.7 Universal Plugin Deployment ve Synchronization"
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- `deploy plan` ve `deploy sync` varsayılan olarak salt-okunur dry-run üretir. Install, update, sync apply ve rollback yalnız kullanıcının CLI'de verdiği açık `--yes` onayıyla dosya değiştirir.").unwrap();
    writeln!(output, "- Registry dışı veya kayıtlı SHA-256 değerinden farklı dosyayı `user-modified/conflict` say. Sessiz üzerine yazma veya otomatik merge yapma; abort, skip ya da açık backup-and-replace politikası kullan.").unwrap();
    writeln!(output, "- `provider all` yalnız executable keşfinde gerçekten bulunan provider'lara dağıtım yapar. Global ve project scope'u karıştırma; dil profilini kurulum sırasında değiştirme.").unwrap();
    writeln!(output, "- Doctor sonuçlarında PASS, WARN, FAIL, NOT_RUN ve UNSUPPORTED ayrımını koru. Yapısal dosya doğrulamasını canlı interaktif çağrı olarak sunma.").unwrap();
    writeln!(output, "- Provider çağrısı: `{}`. `doctor`, `install` ve `sync` isteklerinde skill çağrısı tek başına mutasyon yetkisi değildir; Rust CLI planı ve açık onay gerekir.", adapter.invocation()).unwrap();
    writeln!(output, "- Codex doğrudan `/mt-developer-dna` sunmaz; `$mt-developer-dna` eşdeğeridir. Claude Code skill'i slash çağrısı sağlar. Gemini slash çağrısı `.gemini/commands/mt-developer-dna.toml` dosyasına dayanır ve `/commands list` ile doğrulanır.").unwrap();
    writeln!(output).unwrap();
}

fn render_profile_summary(output: &mut String, profile: &Profile) {
    writeln!(output, "## Profil").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Ad: {}", profile.name.trim()).unwrap();
    writeln!(
        output,
        "- Programlama dili: {}",
        profile.language.display_name()
    )
    .unwrap();
    writeln!(output, "- Arayüz: {}", profile.ui.display_name()).unwrap();
    writeln!(output, "- Veritabanı: {}", profile.database.display_name()).unwrap();
    writeln!(output, "- Arayüz dili: Türkçe").unwrap();
    writeln!(output).unwrap();
}

fn render_required_workflow(output: &mut String, profile: &Profile) {
    writeln!(output, "## Zorunlu çalışma biçimi").unwrap();
    writeln!(output).unwrap();
    if matches!(
        profile.language,
        dna_core::ProgrammingLanguage::Auto | dna_core::ProgrammingLanguage::AskUser
    ) {
        writeln!(output, "- Dil seçimi önce proje hafızasından gelir: proje kökünde `.mt-dna/project.json` varsa `language` alanını oku, kayıtlı dili aynen koru ve tekrar sorma.").unwrap();
        writeln!(output, "- Programlama dili seçilmedi ve hafızada kayıt yoksa: uygun seçenekleri sun, kullanıcıya sor ve açık seçim yapılmadan kod üretme. Seçimden sonra dili `.mt-dna/` hafızasına kaydetmeyi öner; yazmadan önce önizleme göster ve açık onay al.").unwrap();
    } else {
        writeln!(
            output,
            "- Kullanıcının seçtiği programlama dilini aynen koru: {}.",
            profile.language.display_name()
        )
        .unwrap();
    }
    writeln!(
        output,
        "- Kullanıcı arayüzü ve kullanıcıya dönük mesajlar Türkçe olsun."
    )
    .unwrap();
    writeln!(
        output,
        "- RAM verimliliğini ve düşük boşta CPU kullanımını önceliklendir."
    )
    .unwrap();
    writeln!(
        output,
        "- Performans değerlerini garanti değil, ölçülebilir benchmark hedefi olarak belirt."
    )
    .unwrap();
    if profile.features.tests_required {
        writeln!(
            output,
            "- Her aşamada ilgili testleri çalıştır ve gerçek sonucu bildir."
        )
        .unwrap();
    }
    if profile.features.gpu_acceleration {
        writeln!(
            output,
            "- GPU hızlandırmayı yalnızca ölçülmüş fayda varsa kullan."
        )
        .unwrap();
    } else {
        writeln!(output, "- GPU bağımlılığı ekleme.").unwrap();
    }
    writeln!(output).unwrap();
}

fn render_project_memory(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(output, "## Proje hafızası").unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "- Ortak ve denetlenebilir bağlam yalnızca proje kökündeki `.mt-dna/` dosyalarından gelir; provider sohbet geçmişlerinin ortak olduğunu varsayma."
    )
    .unwrap();
    writeln!(
        output,
        "- `init`, `update`, `handoff` ve `memory repair` yazmalarından önce diff/önizleme göster ve açık onay al."
    )
    .unwrap();
    writeln!(
        output,
        "- `resume` ve `memory check` sırasında şema, hash, seçili dil, gizli bilgi ve kaynak güncelliği sorunlarını kesin gerçeklerden ayır."
    )
    .unwrap();
    writeln!(
        output,
        "- `.env`, anahtar, token, kişisel veri veya provider'a özgü geçici bağlamı hafızaya aktarma."
    )
    .unwrap();
    writeln!(
        output,
        "- Bu provider için hedef çağrılar: `{0} init`, `{0} resume`, `{0} update`, `{0} handoff`, `{0} memory check`, `{0} memory repair`.",
        adapter.invocation()
    )
    .unwrap();
    writeln!(output).unwrap();
}

fn render_standards(output: &mut String, profile: &Profile) {
    writeln!(output, "## Ortak yazılım standartları").unwrap();
    writeln!(output).unwrap();
    render_rule_group(output, "Mimari", &profile.standards.architecture);
    render_rule_group(output, "Kalite", &profile.standards.quality);
    render_rule_group(output, "Güvenlik", &profile.standards.security);
    render_rule_group(
        output,
        "Performans benchmark hedefleri",
        &profile.standards.performance_benchmarks,
    );
}

fn render_rule_group(output: &mut String, heading: &str, rules: &[String]) {
    writeln!(output, "### {heading}").unwrap();
    writeln!(output).unwrap();
    for rule in rules {
        writeln!(output, "- {}", rule.trim()).unwrap();
    }
    writeln!(output).unwrap();
}

fn render_provider_notes(output: &mut String, adapter: &dyn adapters::ProviderAdapter) {
    writeln!(output, "## Provider notu").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Çağırma: `{}`", adapter.invocation()).unwrap();
    writeln!(output, "- {}", adapter.provider_note()).unwrap();
    writeln!(output).unwrap();
}

#[cfg(test)]
mod report_tests {
    use super::ReportRenderer;
    use dna_core::{
        FindingSeverity, ReviewFinding, ReviewReport, WorkflowMode, WORKFLOW_SCHEMA_VERSION,
    };

    #[test]
    fn review_renderer_includes_required_evidence_fields() {
        let report = ReviewReport {
            schema_version: WORKFLOW_SCHEMA_VERSION.to_owned(),
            project_id: "sample".to_owned(),
            mode: WorkflowMode::Review,
            language: Some("rust".to_owned()),
            source_fingerprint: "hash".to_owned(),
            scope: "bounded".to_owned(),
            findings: vec![ReviewFinding {
                id: "REVIEW-001".to_owned(),
                severity: FindingSeverity::High,
                file: Some("src/main.rs".to_owned()),
                line: Some(12),
                title: "Örnek bulgu".to_owned(),
                evidence: "Somut kanıt".to_owned(),
                impact: "Somut etki".to_owned(),
                recommendation: "Düzeltme önerisi".to_owned(),
            }],
            assumptions: Vec::new(),
            files_scanned: 3,
            truncated: false,
        };
        let rendered = ReportRenderer::review_report(&report).unwrap();
        assert!(rendered.contains("src/main.rs:12"));
        assert!(rendered.contains("Somut kanıt"));
        assert!(rendered.contains("Somut etki"));
        assert!(rendered.contains("Düzeltme önerisi"));
    }
}
