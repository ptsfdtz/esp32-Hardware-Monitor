use crate::config::APP_DIR_NAME;
use crate::logging::log_line;
use crate::paths::{agent_runtime_dir, elevated_temperature_output_path};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const TEMPERATURE_INTERVAL: Duration = Duration::from_secs(10);
const ELEVATED_TEMPERATURE_MAX_AGE: Duration = Duration::from_secs(35);
const TEMPERATURE_PROBE_EXE: &[u8] = include_bytes!("../temp-probe/TemperatureProbe.exe");
const ELEVATED_TEMPERATURE_AGENT_EXE: &[u8] =
    include_bytes!("../temp-probe/ElevatedTemperatureAgent.exe");
const LIBRE_HARDWARE_MONITOR_FILES: &[BundledFile] = &[
    BundledFile {
        name: "LibreHardwareMonitorLib.dll",
        bytes: include_bytes!(
            "../temp-probe/vendor/LibreHardwareMonitor/LibreHardwareMonitorLib.dll"
        ),
    },
    BundledFile {
        name: "System.Memory.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/System.Memory.dll"),
    },
    BundledFile {
        name: "System.Numerics.Vectors.dll",
        bytes: include_bytes!(
            "../temp-probe/vendor/LibreHardwareMonitor/System.Numerics.Vectors.dll"
        ),
    },
    BundledFile {
        name: "System.Runtime.CompilerServices.Unsafe.dll",
        bytes: include_bytes!(
            "../temp-probe/vendor/LibreHardwareMonitor/System.Runtime.CompilerServices.Unsafe.dll"
        ),
    },
    BundledFile {
        name: "System.Buffers.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/System.Buffers.dll"),
    },
    BundledFile {
        name: "HidSharp.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/HidSharp.dll"),
    },
    BundledFile {
        name: "RAMSPDToolkit-NDD.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/RAMSPDToolkit-NDD.dll"),
    },
    BundledFile {
        name: "DiskInfoToolkit.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/DiskInfoToolkit.dll"),
    },
    BundledFile {
        name: "BlackSharp.Core.dll",
        bytes: include_bytes!("../temp-probe/vendor/LibreHardwareMonitor/BlackSharp.Core.dll"),
    },
];

struct BundledFile {
    name: &'static str,
    bytes: &'static [u8],
}

pub struct TemperatureProbe {
    exe_path: PathBuf,
    libre_dir: PathBuf,
    last_sample: Option<SystemTime>,
    latest: TemperatureReading,
}

#[derive(Clone, Copy, Default)]
pub struct TemperatureReading {
    pub cpu: Option<u8>,
    pub gpu: Option<u8>,
}

impl TemperatureProbe {
    pub fn new() -> Self {
        let base_dir = agent_runtime_dir().unwrap_or_else(|_| env::temp_dir().join(APP_DIR_NAME));
        let exe_path = base_dir.join("TemperatureProbe.exe");
        let bundled_libre_dir = base_dir.join("LibreHardwareMonitor");

        if let Err(err) = install_probe_runtime_files(&base_dir) {
            log_line(&format!(
                "temperature runtime install failed {}: {err}",
                base_dir.display()
            ));
        }

        let libre_dir = resolve_libre_dir(&bundled_libre_dir);

        Self {
            exe_path,
            libre_dir,
            last_sample: None,
            latest: TemperatureReading::default(),
        }
    }

    pub fn latest(&self) -> TemperatureReading {
        self.latest
    }

    pub fn sample(&mut self) -> Option<String> {
        let now = SystemTime::now();
        if let Some(last_sample) = self.last_sample {
            if now.duration_since(last_sample).unwrap_or(Duration::ZERO) < TEMPERATURE_INTERVAL {
                return None;
            }
        }

        self.last_sample = Some(now);

        if let Some(reading) = read_elevated_temperature() {
            self.latest = reading;
            return Some(format!(
                "elevated CPU_TEMP={};GPU_TEMP={}",
                serial_temperature(reading.cpu),
                serial_temperature(reading.gpu)
            ));
        }

        if !self.exe_path.exists() {
            self.latest = TemperatureReading::default();
            return Some("probe_missing".to_string());
        }

        if !has_required_libre_files(&self.libre_dir) {
            self.latest = TemperatureReading::default();
            return Some(format!("libre_missing path={}", self.libre_dir.display()));
        }

        let output = Command::new(&self.exe_path)
            .arg(&self.libre_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.is_empty() {
                    self.latest = TemperatureReading::default();
                    Some("empty_probe_output".to_string())
                } else {
                    self.latest = parse_temperature_reading(&stdout);
                    Some(stdout)
                }
            }
            Ok(output) => {
                self.latest = TemperatureReading::default();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Some(format!(
                    "probe_failed code={:?} {}",
                    output.status.code(),
                    stderr
                ))
            }
            Err(err) => {
                self.latest = TemperatureReading::default();
                Some(format!("probe_launch_failed {err}"))
            }
        }
    }
}

pub fn install_elevated_runtime_files(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("无法创建目录 {}: {e}", dir.display()))?;
    let probe_path = dir.join("TemperatureProbe.exe");
    if !file_matches(&probe_path, TEMPERATURE_PROBE_EXE) {
        fs::write(&probe_path, TEMPERATURE_PROBE_EXE)
            .map_err(|e| format!("无法写入 {}: {e}", probe_path.display()))?;
    }

    let agent_path = dir.join("ESP32HardwareMonitorTemperatureAgent.exe");
    if !file_matches(&agent_path, ELEVATED_TEMPERATURE_AGENT_EXE) {
        fs::write(&agent_path, ELEVATED_TEMPERATURE_AGENT_EXE)
            .map_err(|e| format!("无法写入 {}: {e}", agent_path.display()))?;
    }

    install_bundled_files(
        &dir.join("LibreHardwareMonitor"),
        LIBRE_HARDWARE_MONITOR_FILES,
    )
}

fn install_probe_runtime_files(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("无法创建目录 {}: {e}", dir.display()))?;
    let probe_path = dir.join("TemperatureProbe.exe");
    if !file_matches(&probe_path, TEMPERATURE_PROBE_EXE) {
        fs::write(&probe_path, TEMPERATURE_PROBE_EXE)
            .map_err(|e| format!("无法写入 {}: {e}", probe_path.display()))?;
    }

    install_bundled_files(
        &dir.join("LibreHardwareMonitor"),
        LIBRE_HARDWARE_MONITOR_FILES,
    )
}

fn resolve_libre_dir(bundled_libre_dir: &Path) -> PathBuf {
    let Some(override_dir) = env::var_os("LIBRE_HARDWARE_MONITOR_DIR").map(PathBuf::from) else {
        return bundled_libre_dir.to_path_buf();
    };

    if has_required_libre_files(&override_dir) {
        return override_dir;
    }

    log_line(&format!(
        "invalid LIBRE_HARDWARE_MONITOR_DIR {}, using bundled dependencies",
        override_dir.display()
    ));
    bundled_libre_dir.to_path_buf()
}

fn has_required_libre_files(dir: &Path) -> bool {
    LIBRE_HARDWARE_MONITOR_FILES
        .iter()
        .all(|file| dir.join(file.name).is_file())
}

pub fn serial_temperature(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NA".to_string())
}

fn install_bundled_files(dir: &Path, files: &[BundledFile]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("无法创建目录 {}: {e}", dir.display()))?;

    for file in files {
        let path = dir.join(file.name);
        if file_matches(&path, file.bytes) {
            continue;
        }

        fs::write(&path, file.bytes).map_err(|e| format!("无法写入 {}: {e}", path.display()))?;
    }

    Ok(())
}

fn file_matches(path: &Path, expected: &[u8]) -> bool {
    fs::read(path)
        .map(|actual| actual == expected)
        .unwrap_or(false)
}

fn parse_temperature_reading(line: &str) -> TemperatureReading {
    TemperatureReading {
        cpu: parse_temperature_field(line, "CPU_TEMP"),
        gpu: parse_temperature_field(line, "GPU_TEMP"),
    }
}

fn parse_temperature_field(line: &str, key: &str) -> Option<u8> {
    let prefix = format!("{key}=");
    for part in line.split(';') {
        if let Some(value) = part.trim().strip_prefix(&prefix) {
            let value = value.trim();
            if value.eq_ignore_ascii_case("NA") || value.is_empty() {
                return None;
            }

            let Ok(parsed) = value.parse::<i32>() else {
                return None;
            };

            return Some(parsed.clamp(0, 199) as u8);
        }
    }

    None
}

fn read_elevated_temperature() -> Option<TemperatureReading> {
    let path = elevated_temperature_output_path().ok()?;
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    if SystemTime::now().duration_since(modified).ok()? > ELEVATED_TEMPERATURE_MAX_AGE {
        return None;
    }

    let line = fs::read_to_string(path).ok()?;
    if !line
        .split(';')
        .any(|field| field.trim().eq_ignore_ascii_case("VERSION=1"))
    {
        return None;
    }
    let timestamp = parse_timestamp(&line)?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.abs_diff(timestamp) > ELEVATED_TEMPERATURE_MAX_AGE.as_secs() {
        return None;
    }

    Some(parse_temperature_reading(&line))
}

fn parse_timestamp(line: &str) -> Option<u64> {
    line.split(';')
        .find_map(|field| field.trim().strip_prefix("TIMESTAMP="))?
        .trim()
        .parse()
        .ok()
}
