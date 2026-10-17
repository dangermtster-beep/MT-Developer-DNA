use super::*;

pub fn export_packages(root: &Path, packages: &[ProviderPackage]) -> Result<(), CliError> {
    ensure_root_is_safe(root)?;
    create_dir_if_missing(root)?;

    for package in packages {
        let destination = root.join(package.provider.directory_name());
        if destination.exists() {
            return Err(CliError::DestinationExists(destination));
        }
    }

    for package in packages {
        export_one_package(root, package)?;
    }
    Ok(())
}

pub(crate) fn ensure_root_is_safe(root: &Path) -> Result<(), CliError> {
    if root.exists() {
        let metadata = fs::symlink_metadata(root).map_err(|source| CliError::Io {
            operation: "Hedef bilgisi okunamadı",
            path: root.to_path_buf(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(CliError::SymlinkTarget(root.to_path_buf()));
        }
    }
    Ok(())
}

pub(crate) fn create_dir_if_missing(path: &Path) -> Result<(), CliError> {
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|source| CliError::Io {
        operation: "Klasör oluşturma",
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn export_one_package(root: &Path, package: &ProviderPackage) -> Result<(), CliError> {
    let destination = root.join(package.provider.directory_name());
    let staging = root.join(format!(
        ".mt-dna-tmp-{}-{}",
        package.provider.directory_name(),
        std::process::id()
    ));

    if staging.exists() {
        return Err(CliError::DestinationExists(staging));
    }
    create_dir_if_missing(&staging)?;

    let result = (|| {
        for file in &package.files {
            if !file.has_safe_relative_path() {
                return Err(CliError::UnsafeOutputPath(file.relative_path.clone()));
            }
            let target = staging.join(&file.relative_path);
            if let Some(parent) = target.parent() {
                create_dir_if_missing(parent)?;
            }
            write_new_file(&target, file.contents.as_bytes())?;
        }
        fs::rename(&staging, &destination).map_err(|source| CliError::Io {
            operation: "Atomik paket taşıma",
            path: destination.clone(),
            source,
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub(crate) fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), CliError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path).map_err(|source| CliError::Io {
        operation: "Yeni dosya oluşturma",
        path: path.to_path_buf(),
        source,
    })?;
    file.write_all(contents).map_err(|source| CliError::Io {
        operation: "Dosya yazma",
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| CliError::Io {
        operation: "Dosyayı diske eşitleme",
        path: path.to_path_buf(),
        source,
    })
}
