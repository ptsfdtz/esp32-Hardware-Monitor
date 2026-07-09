use crate::logging::log_line;
use crate::metrics::MetricsCollector;
use crate::paths::app_dir;
use crate::serial_link::SerialManager;
use crate::temperature::{serial_temperature, TemperatureProbe};
use crate::win_util::SingleInstance;
use std::fs;
use std::thread;
use std::time::Duration;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

pub fn run_agent() -> Result<(), String> {
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
