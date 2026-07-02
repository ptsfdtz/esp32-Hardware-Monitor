#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("MonitorSetup only supports Windows.");

use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use std::env;
use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, FILETIME, HANDLE,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{CreateMutexW, GetSystemTimes, CREATE_NO_WINDOW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const APP_TITLE: &str = "ESP32 Hardware Monitor";
const APP_DIR_NAME: &str = "ESP32HardwareMonitor";
const RUN_VALUE_NAME: &str = "ESP32 Hardware Monitor";
const BAUD_RATE: u32 = 115_200;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const TEMPERATURE_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_LIBRE_HARDWARE_MONITOR_DIR: &str = r"C:\Users\user\Downloads\LibreHardwareMonitor";
const TEMPERATURE_PROBE_EXE: &[u8] = include_bytes!("../temp-probe/TemperatureProbe.exe");

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str);

    let result = match command {
        Some("--agent") => run_agent(),
        Some("--install") => install_and_launch(true),
        Some("--uninstall") => uninstall(),
        Some("--help") | Some("-h") => {
            show_message(
                APP_TITLE,
                "双击运行即可安装并启动。\n\n命令：\n--install 安装并启动\n--agent 后台运行\n--uninstall 移除开机自启动",
                MB_ICONINFORMATION,
            );
            Ok(())
        }
        _ => install_and_launch(true),
    };

    if let Err(err) = result {
        log_line(&format!("error: {err}"));
        if command != Some("--agent") {
            show_message(APP_TITLE, &format!("操作失败：\n{err}"), MB_ICONERROR);
        }
    }
}

fn install_and_launch(show_success: bool) -> Result<(), String> {
    let app_dir = app_dir()?;
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("无法创建安装目录 {}: {e}", app_dir.display()))?;

    let installed_exe = installed_exe_path()?;
    let current_exe = env::current_exe().map_err(|e| format!("无法获取当前程序路径: {e}"))?;

    if !same_path(&current_exe, &installed_exe) {
        match fs::copy(&current_exe, &installed_exe) {
            Ok(_) => log_line(&format!("installed to {}", installed_exe.display())),
            Err(e) if installed_exe.exists() => {
                log_line(&format!(
                    "copy failed, keeping existing installed exe {}: {e}",
                    installed_exe.display()
                ));
            }
            Err(e) => {
                return Err(format!(
                    "无法复制程序到安装目录 {}: {e}",
                    installed_exe.display()
                ));
            }
        }
    }

    set_autostart(&installed_exe)?;
    launch_agent(&installed_exe)?;

    if show_success {
        show_message(
            APP_TITLE,
            &format!(
                "已安装并启动后台监控。\n\n安装位置：{}\n日志位置：{}",
                installed_exe.display(),
                log_path()?.display()
            ),
            MB_ICONINFORMATION,
        );
    }

    Ok(())
}

fn uninstall() -> Result<(), String> {
    remove_autostart()?;
    log_line("autostart removed");
    show_message(
        APP_TITLE,
        "已移除开机自启动。\n\n如果后台程序正在运行，请在任务管理器中结束 MonitorSetup.exe。",
        MB_ICONINFORMATION,
    );
    Ok(())
}

fn run_agent() -> Result<(), String> {
    fs::create_dir_all(app_dir()?).map_err(|e| format!("无法创建程序目录: {e}"))?;

    let _single_instance = SingleInstance::new("Local\\ESP32HardwareMonitorAgent")?;
    log_line("agent started");

    let mut collector = MetricsCollector::new();
    let mut serial = SerialManager::new();
    let mut temperature_probe = TemperatureProbe::new();

    loop {
        let metrics = collector.sample();
        if let Some(reading) = temperature_probe.sample() {
            log_line(&format!("temperature {reading}"));
        }

        let temperatures = temperature_probe.latest();
        let cpu_temp = serial_temperature(temperatures.cpu);
        let gpu_temp = serial_temperature(temperatures.gpu);
        let line = format!(
            "CPU={};GPU={};RAM={};CPU_TEMP={};GPU_TEMP={}\n",
            metrics.cpu, metrics.gpu, metrics.ram, cpu_temp, gpu_temp
        );

        if let Err(err) = serial.send(&line) {
            log_line(&format!("serial send failed: {err}"));
        }

        thread::sleep(SAMPLE_INTERVAL);
    }
}

struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    fn new(name: &str) -> Result<Self, String> {
        let wide_name = wide_null(name);
        let handle = unsafe { CreateMutexW(null_mut(), 0, wide_name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "无法创建单实例锁: {}",
                std::io::Error::last_os_error()
            ));
        }

        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err("后台程序已经在运行".to_string());
        }

        Ok(Self { handle })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Metrics {
    cpu: u8,
    gpu: u8,
    ram: u8,
}

struct MetricsCollector {
    cpu: CpuSampler,
    gpu: Option<GpuSampler>,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            cpu: CpuSampler::new(),
            gpu: GpuSampler::new().ok(),
        }
    }

    fn sample(&mut self) -> Metrics {
        Metrics {
            cpu: self.cpu.sample().unwrap_or(0),
            gpu: self.gpu.as_mut().and_then(GpuSampler::sample).unwrap_or(0),
            ram: sample_ram_usage().unwrap_or(0),
        }
    }
}

struct CpuSampler {
    previous: Option<SystemTimes>,
}

#[derive(Clone, Copy)]
struct SystemTimes {
    idle: u64,
    kernel: u64,
    user: u64,
}

impl CpuSampler {
    fn new() -> Self {
        Self { previous: None }
    }

    fn sample(&mut self) -> Option<u8> {
        let current = read_system_times()?;
        let previous = self.previous.replace(current)?;

        let idle = current.idle.saturating_sub(previous.idle);
        let kernel = current.kernel.saturating_sub(previous.kernel);
        let user = current.user.saturating_sub(previous.user);
        let total = kernel + user;

        if total == 0 {
            return Some(0);
        }

        let busy = total.saturating_sub(idle);
        Some(percent(busy as f64 * 100.0 / total as f64))
    }
}

fn read_system_times() -> Option<SystemTimes> {
    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }

    Some(SystemTimes {
        idle: filetime_to_u64(idle),
        kernel: filetime_to_u64(kernel),
        user: filetime_to_u64(user),
    })
}

fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

fn sample_ram_usage() -> Option<u8> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return None;
    }

    Some(status.dwMemoryLoad.min(100) as u8)
}

struct TemperatureProbe {
    exe_path: PathBuf,
    libre_dir: PathBuf,
    last_sample: Option<SystemTime>,
    latest: TemperatureReading,
}

#[derive(Clone, Copy, Default)]
struct TemperatureReading {
    cpu: Option<u8>,
    gpu: Option<u8>,
}

impl TemperatureProbe {
    fn new() -> Self {
        let exe_path = app_dir()
            .unwrap_or_else(|_| env::temp_dir())
            .join("TemperatureProbe.exe");
        let libre_dir = env::var_os("LIBRE_HARDWARE_MONITOR_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_LIBRE_HARDWARE_MONITOR_DIR));

        if let Err(err) = fs::write(&exe_path, TEMPERATURE_PROBE_EXE) {
            log_line(&format!(
                "temperature probe install failed {}: {err}",
                exe_path.display()
            ));
        }

        Self {
            exe_path,
            libre_dir,
            last_sample: None,
            latest: TemperatureReading::default(),
        }
    }

    fn latest(&self) -> TemperatureReading {
        self.latest
    }

    fn sample(&mut self) -> Option<String> {
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

fn serial_temperature(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "NA".to_string())
}

struct SerialManager {
    port: Option<Box<dyn SerialPort>>,
    port_name: Option<String>,
}

impl SerialManager {
    fn new() -> Self {
        Self {
            port: None,
            port_name: None,
        }
    }

    fn send(&mut self, line: &str) -> Result<(), String> {
        if self.port.is_none() {
            self.connect()?;
        }

        let Some(port) = self.port.as_mut() else {
            return Err("没有找到可用串口".to_string());
        };

        if let Err(err) = port.write_all(line.as_bytes()) {
            let name = self
                .port_name
                .take()
                .unwrap_or_else(|| "unknown".to_string());
            self.port = None;
            return Err(format!("{name}: {err}"));
        }

        Ok(())
    }

    fn connect(&mut self) -> Result<(), String> {
        let ports = serialport::available_ports().map_err(|e| format!("枚举串口失败: {e}"))?;
        let candidates = rank_ports(ports)
            .into_iter()
            .filter(|info| port_score(info) < 3)
            .collect::<Vec<_>>();

        for info in candidates {
            match serialport::new(&info.port_name, BAUD_RATE)
                .timeout(Duration::from_millis(250))
                .open()
            {
                Ok(mut port) => {
                    let _ = port.write_data_terminal_ready(false);
                    let _ = port.write_request_to_send(false);
                    log_line(&format!("connected serial port {}", describe_port(&info)));
                    self.port_name = Some(info.port_name);
                    self.port = Some(port);
                    return Ok(());
                }
                Err(err) => {
                    log_line(&format!("open serial {} failed: {err}", info.port_name));
                }
            }
        }

        Err("没有找到 ESP32-C3 串口".to_string())
    }
}

fn rank_ports(mut ports: Vec<SerialPortInfo>) -> Vec<SerialPortInfo> {
    ports.sort_by_key(port_score);
    ports
}

fn port_score(info: &SerialPortInfo) -> u8 {
    match &info.port_type {
        SerialPortType::UsbPort(usb) => {
            let text = format!(
                "{} {} {}",
                usb.manufacturer.as_deref().unwrap_or(""),
                usb.product.as_deref().unwrap_or(""),
                usb.serial_number.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();

            if usb.vid == 0x303a || text.contains("espressif") || text.contains("esp32") {
                0
            } else if usb.vid == 0x10c4
                || usb.vid == 0x1a86
                || text.contains("usb jtag")
                || text.contains("usb serial")
                || text.contains("cp210")
                || text.contains("ch34")
            {
                1
            } else {
                2
            }
        }
        _ => 3,
    }
}

fn describe_port(info: &SerialPortInfo) -> String {
    match &info.port_type {
        SerialPortType::UsbPort(usb) => format!(
            "{} vid={:04x} pid={:04x} {} {}",
            info.port_name,
            usb.vid,
            usb.pid,
            usb.manufacturer.as_deref().unwrap_or(""),
            usb.product.as_deref().unwrap_or("")
        ),
        _ => info.port_name.clone(),
    }
}

fn percent(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }

    value.round().clamp(0.0, 100.0) as u8
}

fn app_dir() -> Result<PathBuf, String> {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "找不到 LOCALAPPDATA 环境变量".to_string())?;
    Ok(base.join(APP_DIR_NAME))
}

fn installed_exe_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("MonitorSetup.exe"))
}

fn log_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("monitor.log"))
}

fn log_line(message: &str) {
    let Ok(path) = log_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

fn set_autostart(exe_path: &Path) -> Result<(), String> {
    let command = format!("\"{}\" --agent", exe_path.display());
    let command_wide = wide_null(&command);
    let value_name = wide_null(RUN_VALUE_NAME);
    let run_key = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

    let mut key = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            run_key.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };

    if status != 0 {
        return Err(format!("打开开机启动注册表失败: {status}"));
    }

    let set_status = unsafe {
        RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            REG_SZ,
            command_wide.as_ptr() as *const u8,
            (command_wide.len() * 2) as u32,
        )
    };

    unsafe {
        RegCloseKey(key);
    }

    if set_status != 0 {
        return Err(format!("写入开机启动注册表失败: {set_status}"));
    }

    Ok(())
}

fn remove_autostart() -> Result<(), String> {
    let value_name = wide_null(RUN_VALUE_NAME);
    let run_key = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

    let mut key = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            run_key.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };

    if status != 0 {
        return Err(format!("打开开机启动注册表失败: {status}"));
    }

    unsafe {
        RegDeleteValueW(key, value_name.as_ptr());
        RegCloseKey(key);
    }

    Ok(())
}

fn launch_agent(exe_path: &Path) -> Result<(), String> {
    Command::new(exe_path)
        .arg("--agent")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("启动后台程序失败: {e}"))?;
    Ok(())
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn show_message(title: &str, message: &str, flags: u32) {
    let title = wide_null(title);
    let message = wide_null(message);
    unsafe {
        MessageBoxW(null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | flags);
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

type PdhQuery = *mut c_void;
type PdhCounter = *mut c_void;

const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
const PDH_MORE_DATA: u32 = 0x8000_07D2;
const PDH_STATUS_OK: u32 = 0;

#[repr(C)]
union PdhFmtValue {
    long_value: i32,
    double_value: f64,
    large_value: i64,
    wide_string_value: *const u16,
}

#[repr(C)]
struct PdhFmtCounterValue {
    c_status: u32,
    value: PdhFmtValue,
}

#[repr(C)]
struct PdhFmtCounterValueItemW {
    name: *mut u16,
    value: PdhFmtCounterValue,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(data_source: *const u16, user_data: usize, query: *mut PdhQuery) -> u32;
    fn PdhAddEnglishCounterW(
        query: PdhQuery,
        full_counter_path: *const u16,
        user_data: usize,
        counter: *mut PdhCounter,
    ) -> u32;
    fn PdhCollectQueryData(query: PdhQuery) -> u32;
    fn PdhGetFormattedCounterArrayW(
        counter: PdhCounter,
        format: u32,
        buffer_size: *mut u32,
        item_count: *mut u32,
        item_buffer: *mut PdhFmtCounterValueItemW,
    ) -> u32;
    fn PdhCloseQuery(query: PdhQuery) -> u32;
}

struct GpuSampler {
    query: PdhQuery,
    counter: PdhCounter,
}

impl GpuSampler {
    fn new() -> Result<Self, String> {
        let mut query = null_mut();
        let status = unsafe { PdhOpenQueryW(null(), 0, &mut query) };
        if status != PDH_STATUS_OK {
            return Err(format!("PdhOpenQueryW failed: 0x{status:08x}"));
        }

        let mut counter = null_mut();
        let path = wide_null("\\GPU Engine(*)\\Utilization Percentage");
        let status = unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) };
        if status != PDH_STATUS_OK {
            unsafe {
                PdhCloseQuery(query);
            }
            return Err(format!("PdhAddEnglishCounterW failed: 0x{status:08x}"));
        }

        unsafe {
            PdhCollectQueryData(query);
        }

        Ok(Self { query, counter })
    }

    fn sample(&mut self) -> Option<u8> {
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != PDH_STATUS_OK {
            return None;
        }

        let mut buffer_size = 0;
        let mut item_count = 0;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                null_mut(),
            )
        };

        if status != PDH_MORE_DATA || buffer_size == 0 || item_count == 0 {
            return None;
        }

        let word_count = (buffer_size as usize + std::mem::size_of::<usize>() - 1)
            / std::mem::size_of::<usize>();
        let mut buffer = vec![0_usize; word_count];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut PdhFmtCounterValueItemW,
            )
        };

        if status != PDH_STATUS_OK {
            return None;
        }

        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const PdhFmtCounterValueItemW,
                item_count as usize,
            )
        };

        let mut total_3d = 0.0;
        let mut total_all = 0.0;
        let mut has_3d = false;

        for item in items {
            if item.value.c_status != PDH_STATUS_OK {
                continue;
            }

            let value = unsafe { item.value.value.double_value };
            if !value.is_finite() || value <= 0.0 {
                continue;
            }

            total_all += value;

            let name = wide_ptr_to_string(item.name).to_ascii_lowercase();
            if name.contains("engtype_3d") {
                has_3d = true;
                total_3d += value;
            }
        }

        Some(percent(if has_3d { total_3d } else { total_all }))
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}

fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}
