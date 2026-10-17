use crate::{
    DecisionInput, MemorySnapshot, ProjectMetadata, ProjectScan, ProviderId, SecretRedactor,
    UpdateRequest,
};
use dna_core::{DevelopmentPlan, TaskStatus};
use std::fmt::Write as _;

pub(crate) fn architecture_document(project: &ProjectMetadata, scan: &ProjectScan) -> String {
    let mut output = format!(
        "# Mimari\n\n- Proje: {}\n- Dil: {}\n- Tür: {}\n\n## Kaynak kanıtları\n\n",
        project.name, project.language, project.project_type
    );
    if scan.evidence_paths.is_empty() {
        output.push_str("- Erişilebilir üst düzey kaynak bulunamadı.\n");
    } else {
        for path in &scan.evidence_paths {
            writeln!(output, "- `{path}`").unwrap();
        }
    }
    output.push_str(
        "\n## Sınırlar\n\n- Mimari yalnızca erişilebilen dosya sistemi kanıtlarından çıkarılır.\n- Varsayım ve öneriler karar kaydı olmadan kanıt sayılmaz.\n",
    );
    output
}

pub(crate) fn initial_decisions() -> String {
    "# Kararlar\n\nHenüz karar kaydı yok.\n".to_owned()
}

pub(crate) fn append_decision(existing: &str, decision: &DecisionInput, timestamp: &str) -> String {
    let mut output = existing.trim_end().to_owned();
    if output.ends_with("Henüz karar kaydı yok.") {
        output = output.replace("Henüz karar kaydı yok.", "");
    }
    writeln!(output, "\n## {timestamp}").unwrap();
    writeln!(
        output,
        "\n- Bağlam: {}",
        SecretRedactor::redact(&decision.context)
    )
    .unwrap();
    output.push_str("- Seçenekler:\n");
    if decision.options.is_empty() {
        output.push_str("  - Kaydedilmedi.\n");
    } else {
        for option in &decision.options {
            writeln!(output, "  - {}", SecretRedactor::redact(option)).unwrap();
        }
    }
    writeln!(
        output,
        "- Karar: {}",
        SecretRedactor::redact(&decision.decision)
    )
    .unwrap();
    writeln!(
        output,
        "- Gerekçe: {}",
        SecretRedactor::redact(&decision.rationale)
    )
    .unwrap();
    output.push_str("- Etkilenen dosyalar:\n");
    if decision.affected_files.is_empty() {
        output.push_str("  - Kaydedilmedi.\n");
    } else {
        for path in &decision.affected_files {
            writeln!(output, "  - `{}`", SecretRedactor::redact(path)).unwrap();
        }
    }
    output
}

pub(crate) fn initial_progress() -> String {
    concat!(
        "# İlerleme\n\n",
        "## Yapıldı\n\n- Proje hafızası başlatıldı.\n\n",
        "## Sürüyor\n\n- Yok.\n\n",
        "## Sıradaki\n\n- Proje gereksinimlerini doğrula.\n\n",
        "## Engeller\n\n- Yok.\n\n",
        "## Testler\n\n- Test edilmedi.\n"
    )
    .to_owned()
}

pub(crate) fn update_progress(request: &UpdateRequest) -> String {
    let mut output = String::from("# İlerleme\n");
    write_section(&mut output, "Yapıldı", &request.completed, "Yok.");
    write_section(&mut output, "Sürüyor", &request.in_progress, "Yok.");
    write_section(&mut output, "Sıradaki", &request.next, "Belirlenmedi.");
    write_section(&mut output, "Engeller", &request.blockers, "Yok.");
    write_section(
        &mut output,
        "Testler",
        &request.test_results,
        "Test edilmedi.",
    );
    output
}

fn write_section(output: &mut String, heading: &str, values: &[String], empty: &str) {
    writeln!(output, "\n## {heading}\n").unwrap();
    if values.is_empty() {
        writeln!(output, "- {empty}").unwrap();
    } else {
        for value in values {
            writeln!(output, "- {}", SecretRedactor::redact(value)).unwrap();
        }
    }
}

pub struct HandoffRenderer;

impl HandoffRenderer {
    pub fn render(snapshot: &MemorySnapshot, target: ProviderId, timestamp: &str) -> String {
        Self::render_with_plan(snapshot, target, timestamp, None)
    }

    pub fn render_with_plan(
        snapshot: &MemorySnapshot,
        target: ProviderId,
        timestamp: &str,
        plan: Option<&DevelopmentPlan>,
    ) -> String {
        Self::render_with_context(snapshot, target, timestamp, plan, None)
    }

    pub fn render_with_context(
        snapshot: &MemorySnapshot,
        target: ProviderId,
        timestamp: &str,
        plan: Option<&DevelopmentPlan>,
        quality_summary: Option<&str>,
    ) -> String {
        let last_decision = last_decision(&snapshot.decisions);
        let open_work = section(&snapshot.progress, "Sıradaki");
        let blockers = section(&snapshot.progress, "Engeller");
        let tests = section(&snapshot.progress, "Testler");
        let mut output = format!(
            "# Provider aktarımı\n\n- Oluşturma: {timestamp}\n- Hedef provider: {}\n- Proje: {}\n- Dil/stack: {} / {}\n- Hafıza şeması: {}\n\n## Son karar\n\n{}\n\n## Açık işler\n\n{}\n\n## Doğrulanan testler\n\n{}\n\n## Riskler ve engeller\n\n{}\n\n## İlk önerilen komut\n\n`{} resume`\n\n> Bu dosya sohbet geçmişini taşımaz; yalnızca denetlenebilir `.mt-dna` proje hafızasını özetler.\n",
            target.as_str(),
            snapshot.project.name,
            snapshot.project.language,
            snapshot.project.project_type,
            snapshot.project.schema_version,
            last_decision,
            open_work,
            tests,
            blockers,
            target.invocation()
        );
        if let Some(plan) = plan {
            writeln!(output, "\n## V0.4 geliştirme planı\n").unwrap();
            writeln!(output, "- Plan: `{}`", plan.plan_id).unwrap();
            writeln!(output, "- Kaynak parmak izi: `{}`", plan.source_fingerprint).unwrap();
            output.push_str("- Bekleyen görevler:\n");
            for task in plan
                .tasks
                .iter()
                .filter(|task| task.status != TaskStatus::Completed)
            {
                writeln!(
                    output,
                    "  - {} — {} ({:?})",
                    task.id, task.title, task.status
                )
                .unwrap();
            }
            output.push_str("- Doğrulama komutları:\n");
            for command in &plan.verification_commands {
                writeln!(output, "  - `{command}` — çalıştırılmadıysa PASS sayma").unwrap();
            }
        }
        if let Some(summary) = quality_summary {
            writeln!(output, "\n## Son V0.5 kalite kanıtı\n").unwrap();
            writeln!(output, "{summary}").unwrap();
        }
        output
    }
}

pub(crate) fn resume_summary(snapshot: &MemorySnapshot) -> String {
    format!(
        "Proje: {}\nDil: {}\nMimari: {}\nSon karar: {}\nBiten işler: {}\nAçık riskler: {}\nSonraki adım: {}",
        snapshot.project.name,
        snapshot.project.language,
        first_evidence(&snapshot.architecture),
        last_decision(&snapshot.decisions),
        section(&snapshot.progress, "Yapıldı"),
        section(&snapshot.progress, "Engeller"),
        section(&snapshot.progress, "Sıradaki")
    )
}

fn first_evidence(value: &str) -> String {
    value
        .lines()
        .find(|line| line.starts_with("- `"))
        .unwrap_or("- Kaynak kanıtı yok.")
        .trim_start_matches("- ")
        .to_owned()
}

fn last_decision(value: &str) -> String {
    let sections = value.match_indices("\n## ").collect::<Vec<_>>();
    sections
        .last()
        .map(|(index, _)| value[index + 1..].trim().to_owned())
        .unwrap_or_else(|| "Henüz karar kaydı yok.".to_owned())
}

fn section(value: &str, heading: &str) -> String {
    let marker = format!("## {heading}");
    let Some(start) = value.find(&marker) else {
        return "Kayıt yok.".to_owned();
    };
    let body = &value[start + marker.len()..];
    let end = body.find("\n## ").unwrap_or(body.len());
    body[..end].trim().to_owned()
}
