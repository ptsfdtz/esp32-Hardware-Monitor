use crate::logging::log_line;
use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use std::io::Write;
use std::time::Duration;

const BAUD_RATE: u32 = 115_200;

pub struct SerialManager {
    port: Option<Box<dyn SerialPort>>,
    port_name: Option<String>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            port_name: None,
        }
    }

    pub fn send(&mut self, line: &str) -> Result<(), String> {
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
