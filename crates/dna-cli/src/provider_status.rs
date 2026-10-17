use dna_deploy::detect_all;

pub fn provider_status_report() -> String {
    let mut output = String::from("Provider durumu (salt-okunur):\n");
    for probe in detect_all() {
        output.push_str(&format!(
            "- {}: {} | sürüm: {} | çağrı: {} | slash: {:?} | {}\n",
            probe.provider.as_str(),
            if probe.installed {
                "ALGILANDI"
            } else {
                "NOT_FOUND/NOT_APPLICABLE"
            },
            probe.version.as_deref().unwrap_or("UNAVAILABLE"),
            probe.invocation,
            probe.slash_command,
            probe.evidence,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::provider_status_report;

    #[test]
    fn report_lists_all_v08_providers() {
        let report = provider_status_report();
        for provider in [
            "codex",
            "claude",
            "gemini",
            "antigravity",
            "cursor",
            "cline",
            "opencode",
        ] {
            assert!(report.contains(provider));
        }
    }
}
