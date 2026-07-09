use crate::config::APP_DIR_NAME;
use crate::logging::log_line;
use crate::paths::app_dir;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const TEMPERATURE_INTERVAL: Duration = Duration::from_secs(10);
const TEMPERATURE_PROBE_EXE: &[u8] = include_bytes!("../temp-probe/TemperatureProbe.exe");
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
        let base_dir = app_dir().unwrap_or_else(|_| env::temp_dir().join(APP_DIR_NAME));
        let exe_path = base_dir.join("TemperatureProbe.exe");
        let bundled_libre_dir = base_dir.join("LibreHardwareMonitor");
        let libre_dir = env::var_os("LIBRE_HARDWARE_MONITOR_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| bundled_libre_dir.clone());

        if let Err(err) = fs::write(&exe_path, TEMPERATURE_PROBE_EXE) {
            log_line(&format!(
                "temperature probe install failed {}: {err}",
                exe_path.display()
            ));
        }

        if let Err(err) = install_bundled_files(&bundled_libre_dir, LIBRE_HARDWARE_MONITOR_FILES) {
            log_line(&format!(
                "libre hardware monitor install failed {}: {err}",
                bundled_libre_dir.display()
            ));
        }

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

        if !self.exe_path.exists() {
            self.latest = TemperatureReading::default();
            return Some("probe_missing".to_string());
        }

        if !self.libre_dir.join("LibreHardwareMonitorLib.dll").exists() {
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
