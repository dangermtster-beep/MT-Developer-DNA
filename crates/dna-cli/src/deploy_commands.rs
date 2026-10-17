use super::*;

pub(crate) fn run_profiles(command: ProfilesCommand) -> Result<String, CliError> {
    match command {
        ProfilesCommand::List => Ok(concat!(
            "Yerleşik profiller:\n",
            "- universal: profiles/universal.json (dil kullanıcıdan sorulur)\n",
            "- rust, csharp, python, cpp, go, java, typescript: mt-developer-dna/references/languages/<id>.json\n"
        ).to_owned()),
        ProfilesCommand::Validate { profile } => {
            let contents = fs::read_to_string(&profile).map_err(|source| CliError::Io {
                operation: "Profil okuma",
                path: profile.clone(),
                source,
            })?;
            if let Ok(value) = Profile::from_json_str(&contents) {
                return Ok(format!("Evrensel profil geçerli: {} | Dil: {}", value.name, value.language.display_name()));
            }
            let value = LanguageProfile::from_json_str(&contents)
                .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))?;
            Ok(format!("Dil profili geçerli: {} ({})", value.display_name, value.id))
        }
        ProfilesCommand::Show { profile } => {
            run_profiles(ProfilesCommand::Validate { profile: profile.clone() })?;
            fs::read_to_string(&profile).map_err(|source| CliError::Io {
                operation: "Profil okuma",
                path: profile,
                source,
            })
        }
    }
}

pub(crate) fn provider_list(json: bool) -> Result<String, CliError> {
    let detections = detect_all();
    if json {
        return serde_json::to_string_pretty(&detections)
            .map_err(|error| CliError::InvalidBackupManifest(error.to_string()));
    }
    let mut output = String::from("Provider keşfi (salt-okunur):\n");
    for detection in detections {
        output.push_str(&format!(
            "- {}: {} | sürüm={} | çağrı={} | slash={:?} | {}\n",
            detection.provider.as_str(),
            if detection.installed {
                "ALGILANDI"
            } else {
                "NOT_FOUND"
            },
            detection.version.as_deref().unwrap_or("UNAVAILABLE"),
            detection.invocation,
            detection.slash_command,
            detection.evidence
        ));
    }
    Ok(output)
}

pub(crate) fn run_deployment(command: DeployCommand) -> Result<String, CliError> {
    match command {
        DeployCommand::Plan(args) => {
            execute_deployment_plan(&args, false, false, DeploymentConflict::Abort)
        }
        DeployCommand::Install(args) | DeployCommand::Update(args) => {
            if !args.yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            execute_deployment_plan(&args.deployment, true, true, args.conflict)
        }
        DeployCommand::Sync(args) => {
            if args.apply && args.dry_run {
                return Err(CliError::InvalidDeploymentArguments(
                    "--apply ve --dry-run birlikte kullanılamaz".to_owned(),
                ));
            }
            if args.apply && !args.yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            execute_deployment_plan(&args.deployment, args.apply, args.yes, args.conflict)
        }
        DeployCommand::Status(args) => {
            let root = resolve_target_root(args.scope.into(), &args.path)?;
            let results = deployment_status(&root)?;
            format_diagnostic_output(&results, args.json)
        }
        DeployCommand::Rollback {
            transaction,
            scope,
            path,
            yes,
        } => {
            if !yes {
                return Err(CliError::DeploymentConsentRequired);
            }
            let root = resolve_target_root(scope.into(), &path)?;
            let report = rollback_deployment(&root, &transaction)?;
            Ok(format!(
                "Deployment rollback tamamlandı: transaction={}, değişen={}, yedek korundu.",
                transaction, report.changed
            ))
        }
    }
}

pub(crate) fn execute_deployment_plan(
    args: &DeploymentArgs,
    apply: bool,
    approved: bool,
    conflict: DeploymentConflict,
) -> Result<String, CliError> {
    if apply && !approved {
        return Err(CliError::DeploymentConsentRequired);
    }
    if args.provider == ProviderSelection::All && args.provider_path.is_some() {
        return Err(CliError::InvalidDeploymentArguments(
            "--provider-path yalnızca tek bir --provider seçimiyle kullanılabilir.".to_owned(),
        ));
    }
    let loaded = Profile::load(&args.profile)?;
    let scope: InstallScope = args.scope.into();
    let (profile, scope_note) = profile_for_scope(loaded, scope);
    let root = resolve_target_root(scope, &args.path)?;
    let detections = if args.provider == ProviderSelection::All {
        detect_all()
    } else {
        let provider = args
            .provider
            .providers()
            .into_iter()
            .next()
            .ok_or_else(|| CliError::ProviderNotDetected("all".to_owned()))?;
        vec![detect_provider_with_path(
            ProviderKey::from(provider),
            args.provider_path.as_deref(),
        )?]
    };
    let mut providers = args.provider.providers();
    if args.provider == ProviderSelection::All {
        providers.retain(|provider| {
            let key = ProviderKey::from(*provider);
            detections
                .iter()
                .any(|detection| detection.provider == key && detection.installed)
        });
    } else if let Some(provider) = providers.first() {
        let key = ProviderKey::from(*provider);
        if !detections
            .iter()
            .any(|detection| detection.provider == key && detection.installed)
        {
            return Err(CliError::ProviderNotDetected(key.as_str().to_owned()));
        }
    }
    if providers.is_empty() {
        return Err(CliError::ProviderNotDetected("all".to_owned()));
    }

    let mut output = String::new();
    if let Some(note) = scope_note {
        output.push_str(note);
    }
    for provider in providers {
        let detection = detections
            .iter()
            .find(|detection| detection.provider == ProviderKey::from(provider))
            .ok_or_else(|| {
                CliError::ProviderNotDetected(ProviderKey::from(provider).as_str().to_owned())
            })?;
        let package = render_package(&profile, provider)?;
        let plan = plan_deployment(&root, &package, scope, Some(&detection.global_skill_path))?;
        output.push_str(&format_deployment_plan(&plan));
        let report = if apply {
            Some(apply_deployment(&plan, conflict.into())?)
        } else {
            None
        };
        output.push_str(&deployment_outcome_line(report.as_ref()));
    }
    if !apply {
        output.push_str(
            "Senkronizasyon/kurulum uygulanmadı. Yazmak için ilgili komutta --yes kullanın.\n",
        );
    }
    Ok(output)
}

/// Plan satırlarının altına gerçek sonucu yazar: uygulandıysa rapor, değilse dry-run bilgisi.
pub(crate) fn deployment_outcome_line(report: Option<&DeploymentReport>) -> String {
    match report {
        Some(report) => format!(
            "Uygulandı: changed={}, unchanged={}, skipped={}, transaction={}\n",
            report.changed,
            report.unchanged,
            report.skipped,
            report.transaction_id.as_deref().unwrap_or("yok")
        ),
        None => "Dry-run: bu provider için hiçbir dosya değiştirilmedi.\n".to_owned(),
    }
}

pub(crate) const GLOBAL_SCOPE_NOTE: &str = "BİLGİ: Global kurulum projeden bağımsızdır; profildeki dil, arayüz ve veritabanı seçimi global skill'e yazılmadı. Dil her projede kullanıcıya sorulur ve `.mt-dna/` hafızasında tutulur.\n";

/// Global scope'ta profil nötrlenir; nötrleme olduysa kullanıcıya gösterilecek notu döndürür.
pub(crate) fn profile_for_scope(
    profile: Profile,
    scope: InstallScope,
) -> (Profile, Option<&'static str>) {
    if scope != InstallScope::Global || profile.is_global_neutral() {
        return (profile, None);
    }
    (profile.for_global_scope(), Some(GLOBAL_SCOPE_NOTE))
}

pub(crate) fn run_doctor(
    selection: ProviderSelection,
    scope: DeploymentScope,
    path: &Path,
    provider_path: Option<&Path>,
    json: bool,
) -> Result<String, CliError> {
    if selection == ProviderSelection::All && provider_path.is_some() {
        return Err(CliError::InvalidDeploymentArguments(
            "--provider-path yalnızca tek bir --provider seçimiyle kullanılabilir.".to_owned(),
        ));
    }
    let root = resolve_target_root(scope.into(), path)?;
    let providers = selection
        .providers()
        .into_iter()
        .map(ProviderKey::from)
        .collect::<Vec<_>>();
    let results = deploy_doctor(&root, &providers, provider_path)?;
    format_diagnostic_output(&results, json)
}

pub(crate) fn format_diagnostic_output(
    results: &[DiagnosticResult],
    json: bool,
) -> Result<String, CliError> {
    if json {
        serde_json::to_string_pretty(results)
            .map_err(|error| CliError::InvalidBackupManifest(error.to_string()))
    } else {
        Ok(format_diagnostics(results))
    }
}

pub(crate) fn render_selected(
    profile: &Profile,
    selection: ProviderSelection,
) -> Result<Vec<ProviderPackage>, RenderError> {
    selection
        .providers()
        .into_iter()
        .map(|provider| render_package(profile, provider))
        .collect()
}

pub(crate) fn format_preview(packages: &[ProviderPackage]) -> String {
    let mut output = String::new();
    for package in packages {
        output.push_str(&format!(
            "===== {} =====\n",
            package.provider.display_name()
        ));
        for warning in &package.warnings {
            output.push_str(&format!("UYARI: {warning}\n"));
        }
        for file in &package.files {
            output.push_str(&format!("--- {} ---\n", file.relative_path));
            output.push_str(&file.contents);
        }
    }
    output
}
