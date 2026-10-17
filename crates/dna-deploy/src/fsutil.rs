use super::*;

pub(crate) fn read_registry(root: &Path) -> Result<InstallRegistry, DeployError> {
    let path = state_path(root, "registry.json")?;
    if !path.exists() {
        return Ok(InstallRegistry {
            schema_version: DEPLOY_SCHEMA_VERSION,
            entries: Vec::new(),
        });
    }
    let registry: InstallRegistry = read_json(&path)?;
    if registry.schema_version != DEPLOY_SCHEMA_VERSION {
        return Err(DeployError::InvalidManifest(format!(
            "Desteklenmeyen registry şeması: {}",
            registry.schema_version
        )));
    }
    Ok(registry)
}

pub(crate) fn write_registry(root: &Path, registry: &InstallRegistry) -> Result<(), DeployError> {
    atomic_write_json(root, &state_path(root, "registry.json")?, registry)
}

pub(crate) fn verify_plan(plan: &InstallPlan) -> Result<(), DeployError> {
    for change in &plan.changes {
        let target = checked_target(&plan.target_root, &change.path)?;
        let current = if target.exists() {
            Some(sha256_hex(&read_regular_file(&target)?))
        } else {
            None
        };
        if current != change.current_sha256 {
            return Err(DeployError::Conflict(format!(
                "Plan sonrası değişti: {}",
                change.path
            )));
        }
    }
    Ok(())
}

pub(crate) fn restore_applied(
    root: &Path,
    backup_root: &Path,
    tx: &TransactionRecord,
    applied: &[String],
) {
    for path in applied.iter().rev() {
        if let Some(file) = tx.files.iter().find(|file| &file.path == path) {
            if file.had_original {
                if let Ok(contents) = read_regular_file(&backup_root.join("files").join(path)) {
                    let _ = atomic_write(root, &root.join(path), &contents);
                }
            } else {
                let _ = fs::remove_file(root.join(path));
            }
        }
    }
}

pub(crate) struct DeploymentLock {
    path: PathBuf,
}

impl DeploymentLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self, DeployError> {
        let path = state_path(root, "install.lock")?;
        if let Some(parent) = path.parent() {
            create_directory_tree(root, parent)?;
        }
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    DeployError::Locked(path.clone())
                } else {
                    io("Deployment lock oluşturma", &path, source)
                }
            })?;
        Ok(Self { path })
    }
}

impl Drop for DeploymentLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn validate_root(root: &Path) -> Result<PathBuf, DeployError> {
    let metadata =
        fs::symlink_metadata(root).map_err(|_| DeployError::InvalidRoot(root.to_path_buf()))?;
    if !metadata.is_dir() {
        return Err(DeployError::InvalidRoot(root.to_path_buf()));
    }
    if is_link(&metadata) {
        return Err(DeployError::LinkTarget(root.to_path_buf()));
    }
    fs::canonicalize(root).map_err(|source| io("Deployment kökü çözümleme", root, source))
}

pub(crate) fn state_path(root: &Path, suffix: &str) -> Result<PathBuf, DeployError> {
    checked_target(root, &format!("{STATE_DIRECTORY}/{suffix}"))
}

pub(crate) fn checked_target(root: &Path, relative: &str) -> Result<PathBuf, DeployError> {
    let relative_path = Path::new(relative);
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || !relative_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(DeployError::UnsafePath(relative.to_owned()));
    }
    let target = root.join(relative_path);
    ensure_no_links(root, &target)?;
    Ok(target)
}

pub(crate) fn ensure_no_links(root: &Path, target: &Path) -> Result<(), DeployError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| DeployError::UnsafePath(target.display().to_string()))?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|source| io("Yol bilgisi okuma", &current, source))?;
            if is_link(&metadata) {
                return Err(DeployError::LinkTarget(current));
            }
        }
    }
    Ok(())
}

pub(crate) fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(crate) fn read_regular_file(path: &Path) -> Result<Vec<u8>, DeployError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| io("Dosya bilgisi okuma", path, source))?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(DeployError::LinkTarget(path.to_path_buf()));
    }
    fs::read(path).map_err(|source| io("Dosya okuma", path, source))
}

pub(crate) fn create_directory_tree(root: &Path, path: &Path) -> Result<(), DeployError> {
    ensure_no_links(root, path)?;
    fs::create_dir_all(path).map_err(|source| io("Klasör oluşturma", path, source))
}

pub(crate) fn write_new_synced(
    root: &Path,
    path: &Path,
    contents: &[u8],
) -> Result<(), DeployError> {
    let parent = path
        .parent()
        .ok_or_else(|| DeployError::UnsafePath(path.display().to_string()))?;
    create_directory_tree(root, parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io("Yeni dosya oluşturma", path, source))?;
    file.write_all(contents)
        .map_err(|source| io("Dosya yazma", path, source))?;
    file.sync_all()
        .map_err(|source| io("Dosya eşitleme", path, source))
}

pub(crate) fn write_json_new<T: Serialize>(
    root: &Path,
    path: &Path,
    value: &T,
) -> Result<(), DeployError> {
    let data = serde_json::to_vec_pretty(value)
        .map_err(|e| DeployError::InvalidManifest(e.to_string()))?;
    write_new_synced(root, path, &data)
}

pub(crate) fn atomic_write_json<T: Serialize>(
    root: &Path,
    path: &Path,
    value: &T,
) -> Result<(), DeployError> {
    let data = serde_json::to_vec_pretty(value)
        .map_err(|e| DeployError::InvalidManifest(e.to_string()))?;
    atomic_write(root, path, &data)
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, DeployError> {
    let data = read_regular_file(path)?;
    serde_json::from_slice(&data).map_err(|e| DeployError::InvalidManifest(e.to_string()))
}

pub(crate) fn atomic_write(root: &Path, target: &Path, contents: &[u8]) -> Result<(), DeployError> {
    ensure_no_links(root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| DeployError::UnsafePath(target.display().to_string()))?;
    create_directory_tree(root, parent)?;
    let name = target
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| DeployError::UnsafePath(target.display().to_string()))?;
    let staging = parent.join(format!(
        ".{name}.mt-dna-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    write_new_synced(root, &staging, contents)?;
    let result =
        replace_path(&staging, target).map_err(|source| io("Atomik değiştirme", target, source));
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

#[cfg(windows)]
pub(crate) fn replace_path(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn replace_path(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

pub(crate) fn contains_secret(contents: &[u8]) -> bool {
    let text = String::from_utf8_lossy(contents).to_ascii_lowercase();
    [
        "api_key=",
        "apikey=",
        "secret=",
        "token=",
        "-----begin private key-----",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub(crate) fn new_transaction_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("tx-{nanos}-{}", std::process::id())
}

pub(crate) fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub(crate) fn io(operation: &'static str, path: &Path, source: std::io::Error) -> DeployError {
    DeployError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, bytes) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|value| format!("{value:08x}")).collect()
}
