use crate::{
    CheckResult, CheckStatus, CommandSpec, MetricCapability, PerformanceMetrics, QualityError,
    ToolInfo,
};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct RunnerLimits {
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub sample_interval: Duration,
}

impl Default for RunnerLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(600),
            max_output_bytes: 512 * 1024,
            sample_interval: Duration::from_millis(25),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub struct CheckRunner {
    limits: RunnerLimits,
}

impl CheckRunner {
    pub const fn new(limits: RunnerLimits) -> Self {
        Self { limits }
    }

    pub fn run(
        &self,
        root: &Path,
        spec: &CommandSpec,
        cancellation: &CancellationToken,
    ) -> Result<CheckResult, QualityError> {
        spec.validate()?;
        let root = canonical_root(root)?;
        if cancellation.is_cancelled() {
            return Ok(not_run(spec, "Kullanıcı iptali çalıştırmadan önce alındı."));
        }
        let outcome =
            match execute_process(&root, &spec.program, &spec.args, self.limits, cancellation) {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(not_run(
                        spec,
                        &format!(
                            "Araç kurulu değil veya PATH üzerinde değil: {}",
                            spec.program
                        ),
                    ));
                }
                Err(source) => {
                    return Err(QualityError::Io {
                        operation: "Kalite kontrolü başlatma",
                        path: root,
                        source,
                    });
                }
            };
        let version = self.tool_version(&root, &spec.program);
        let unavailable_subtool = missing_subtool(&outcome.stdout, &outcome.stderr);
        let (status, summary) = if outcome.cancelled {
            (
                CheckStatus::NotRun,
                "Kontrol kullanıcı tarafından iptal edildi.",
            )
        } else if outcome.timed_out {
            (CheckStatus::NotRun, "Kontrol zaman aşımına uğradı.")
        } else if outcome.exit_code == Some(0) {
            (CheckStatus::Pass, "Kontrol başarıyla tamamlandı.")
        } else if unavailable_subtool {
            (
                CheckStatus::NotRun,
                "Gerekli alt araç veya bileşen kurulu değil.",
            )
        } else {
            (CheckStatus::Fail, "Kontrol başarısız oldu.")
        };
        let evidence = evidence(&outcome.stdout, &outcome.stderr, outcome.exit_code);
        Ok(CheckResult {
            id: spec.id.clone(),
            category: spec.category,
            status,
            required: spec.required,
            exit_code: outcome.exit_code,
            tool: ToolInfo {
                program: spec.program.clone(),
                available: true,
                version,
            },
            command: std::iter::once(spec.program.clone())
                .chain(spec.args.iter().cloned())
                .collect(),
            summary: summary.to_owned(),
            evidence,
            stdout_truncated: outcome.stdout_truncated,
            stderr_truncated: outcome.stderr_truncated,
            metrics: outcome.metrics,
        })
    }

    fn tool_version(&self, root: &Path, program: &str) -> Option<String> {
        let limits = RunnerLimits {
            timeout: self.limits.timeout.min(Duration::from_secs(3)),
            max_output_bytes: 8 * 1024,
            sample_interval: self.limits.sample_interval,
        };
        let token = CancellationToken::default();
        let outcome =
            execute_process(root, program, &["--version".to_owned()], limits, &token).ok()?;
        if outcome.timed_out || outcome.cancelled {
            return None;
        }
        outcome
            .stdout
            .lines()
            .chain(outcome.stderr.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(240).collect())
    }
}

fn missing_subtool(stdout: &str, stderr: &str) -> bool {
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    [
        "no such command",
        "is not installed",
        "could not execute process",
        "command not found",
        "not recognized as an internal or external command",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn canonical_root(root: &Path) -> Result<PathBuf, QualityError> {
    if !root.is_dir() {
        return Err(QualityError::InvalidProjectRoot(root.to_path_buf()));
    }
    std::fs::canonicalize(root).map_err(|source| QualityError::Io {
        operation: "Proje kökünü çözümleme",
        path: root.to_path_buf(),
        source,
    })
}

struct RawOutcome {
    exit_code: Option<i32>,
    timed_out: bool,
    cancelled: bool,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    metrics: PerformanceMetrics,
}

fn execute_process(
    root: &Path,
    program: &str,
    args: &[String],
    limits: RunnerLimits,
    cancellation: &CancellationToken,
) -> std::io::Result<RawOutcome> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| spawn_capped_reader(pipe, limits.max_output_bytes));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| spawn_capped_reader(pipe, limits.max_output_bytes));
    let started = Instant::now();
    let mut peak_rss = None;
    let mut cpu_time = None;
    let mut timed_out = false;
    let mut cancelled = false;
    let exit_code = loop {
        sample_process(&child, &mut peak_rss, &mut cpu_time);
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        if cancellation.is_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break child.wait()?.code();
        }
        if started.elapsed() >= limits.timeout {
            timed_out = true;
            let _ = child.kill();
            break child.wait()?.code();
        }
        thread::sleep(limits.sample_interval.max(Duration::from_millis(10)));
    };
    sample_process(&child, &mut peak_rss, &mut cpu_time);
    let (stdout, stdout_truncated) = join_reader(stdout_reader)?;
    let (stderr, stderr_truncated) = join_reader(stderr_reader)?;
    let windows_metrics = cfg!(windows);
    Ok(RawOutcome {
        exit_code,
        timed_out,
        cancelled,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        metrics: PerformanceMetrics {
            duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
            peak_rss_bytes: peak_rss,
            peak_rss_capability: if windows_metrics && peak_rss.is_some() {
                MetricCapability::Measured
            } else {
                MetricCapability::Unavailable
            },
            cpu_time_ms: cpu_time,
            cpu_time_capability: if windows_metrics && cpu_time.is_some() {
                MetricCapability::Measured
            } else {
                MetricCapability::Unavailable
            },
            idle_cpu_percent: None,
            idle_cpu_capability: MetricCapability::Unavailable,
            measurement_method: if windows_metrics {
                "wall clock + Windows GetProcessTimes/K32GetProcessMemoryInfo; 25 ms sınırlı örnekleme"
                    .to_owned()
            } else {
                "wall clock; OS süreç RAM/CPU ölçümü desteklenmiyor".to_owned()
            },
        },
    })
}

fn spawn_capped_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> thread::JoinHandle<std::io::Result<(String, bool)>> {
    thread::spawn(move || {
        let mut kept = Vec::with_capacity(limit.min(64 * 1024));
        let mut buffer = [0u8; 8192];
        let mut truncated = false;
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let remaining = limit.saturating_sub(kept.len());
            let take = remaining.min(count);
            kept.extend_from_slice(&buffer[..take]);
            truncated |= take < count;
        }
        Ok((String::from_utf8_lossy(&kept).into_owned(), truncated))
    })
}

fn join_reader(
    handle: Option<thread::JoinHandle<std::io::Result<(String, bool)>>>,
) -> std::io::Result<(String, bool)> {
    match handle {
        Some(handle) => handle
            .join()
            .unwrap_or_else(|_| Err(std::io::Error::other("çıktı okuyucu thread başarısız"))),
        None => Ok((String::new(), false)),
    }
}

fn evidence(stdout: &str, stderr: &str, exit_code: Option<i32>) -> String {
    let mut text = format!("exit_code={exit_code:?}");
    let output = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    if !output.trim().is_empty() {
        text.push_str(" | ");
        text.extend(output.trim().chars().take(4000));
    }
    text
}

fn not_run(spec: &CommandSpec, reason: &str) -> CheckResult {
    CheckResult {
        id: spec.id.clone(),
        category: spec.category,
        status: CheckStatus::NotRun,
        required: spec.required,
        exit_code: None,
        tool: ToolInfo {
            program: spec.program.clone(),
            available: false,
            version: None,
        },
        command: std::iter::once(spec.program.clone())
            .chain(spec.args.iter().cloned())
            .collect(),
        summary: reason.to_owned(),
        evidence: reason.to_owned(),
        stdout_truncated: false,
        stderr_truncated: false,
        metrics: PerformanceMetrics::default(),
    }
}

#[cfg(windows)]
fn sample_process(child: &Child, peak_rss: &mut Option<u64>, cpu_time: &mut Option<u64>) {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetProcessTimes;
    let handle = child.as_raw_handle();
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    let memory_ok = unsafe { K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) } != 0;
    if memory_ok {
        let sample = counters.PeakWorkingSetSize as u64;
        *peak_rss = Some(peak_rss.unwrap_or_default().max(sample));
    }
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let times_ok =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } != 0;
    if times_ok {
        let units = filetime_units(kernel).saturating_add(filetime_units(user));
        *cpu_time = Some(units / 10_000);
    }
}

#[cfg(windows)]
fn filetime_units(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

#[cfg(not(windows))]
fn sample_process(_child: &Child, _peak_rss: &mut Option<u64>, _cpu_time: &mut Option<u64>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CheckCategory;

    fn spec(args: &[&str]) -> CommandSpec {
        CommandSpec {
            id: "runner-test".to_owned(),
            category: CheckCategory::Tests,
            program: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            args: args.iter().map(|value| (*value).to_owned()).collect(),
            required: true,
            mutates_source: false,
        }
    }

    #[test]
    fn successful_and_failed_commands_keep_real_status() {
        let runner = CheckRunner::new(RunnerLimits {
            timeout: Duration::from_secs(5),
            max_output_bytes: 4096,
            sample_interval: Duration::from_millis(10),
        });
        let root = std::env::current_dir().unwrap();
        let success = runner
            .run(&root, &spec(&["--list"]), &CancellationToken::default())
            .unwrap();
        assert_eq!(success.status, CheckStatus::Pass);
        let failed = runner
            .run(
                &root,
                &spec(&["--definitely-invalid-test-argument"]),
                &CancellationToken::default(),
            )
            .unwrap();
        assert_eq!(failed.status, CheckStatus::Fail);
        assert_ne!(failed.exit_code, Some(0));
    }

    #[test]
    fn missing_tool_and_pre_cancel_are_not_run() {
        let runner = CheckRunner::new(RunnerLimits::default());
        let root = std::env::current_dir().unwrap();
        let mut missing = spec(&[]);
        missing.program = "mt-dna-tool-that-does-not-exist".to_owned();
        assert_eq!(
            runner
                .run(&root, &missing, &CancellationToken::default())
                .unwrap()
                .status,
            CheckStatus::NotRun
        );
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        assert_eq!(
            runner
                .run(&root, &spec(&["--list"]), &cancellation)
                .unwrap()
                .status,
            CheckStatus::NotRun
        );
    }

    #[test]
    fn timeout_stops_process_and_is_not_run() {
        let runner = CheckRunner::new(RunnerLimits {
            timeout: Duration::from_millis(30),
            max_output_bytes: 4096,
            sample_interval: Duration::from_millis(10),
        });
        let root = std::env::current_dir().unwrap();
        let result = runner
            .run(
                &root,
                &spec(&["--exact", "runner::tests::sleep_helper", "--nocapture"]),
                &CancellationToken::default(),
            )
            .unwrap();
        assert_eq!(result.status, CheckStatus::NotRun);
        assert!(result.summary.contains("zaman aşımı"));
    }

    #[test]
    fn sleep_helper() {
        thread::sleep(Duration::from_millis(500));
    }
}
