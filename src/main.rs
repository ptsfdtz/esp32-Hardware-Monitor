#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("ESP32HardwareMonitor only supports Windows.");

mod agent;
mod config;
mod install;
mod logging;
mod metrics;
mod paths;
mod pawnio;
mod serial_link;
mod temperature;
mod win_util;

use crate::agent::run_agent;
use crate::config::APP_TITLE;
use crate::install::{
    install_and_launch, install_elevated_temperature, remove_elevated_temperature,
    request_elevated_temperature_install, request_elevated_temperature_removal, uninstall,
};
use crate::logging::log_line;
use crate::win_util::{show_error, show_info};
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str);
    let show_error_dialog = matches!(
        command,
        Some("--install-ui") | Some("--uninstall-ui") | Some("--install-pawnio")
    );

    let result = match command {
        Some("--agent") => run_agent(),
        Some("--install") | Some("--install-silent") => install_and_launch(false),
        Some("--install-ui") => install_and_launch(true),
        Some("--install-pawnio") => pawnio::install(),
        Some("--enable-elevated-temperature") => request_elevated_temperature_install(),
        Some("--install-elevated-temperature") => install_elevated_temperature(),
        Some("--disable-elevated-temperature") => request_elevated_temperature_removal(),
        Some("--remove-elevated-temperature") => remove_elevated_temperature(),
        Some("--uninstall") => uninstall(false),
        Some("--uninstall-ui") => uninstall(true),
        Some("--help") | Some("-h") => {
            show_info(
                APP_TITLE,
                "双击运行即可静默安装并启动。\n\n命令：\n--install 静默安装并启动\n--install-ui 安装并显示结果\n--install-pawnio 安装 CPU 温度驱动\n--enable-elevated-temperature 一次性启用无感 CPU 温度读取\n--disable-elevated-temperature 移除高权限温度任务\n--agent 后台运行\n--uninstall 静默移除开机自启动\n--uninstall-ui 移除开机自启动并显示结果",
            );
            Ok(())
        }
        _ => install_and_launch(false),
    };

    if let Err(err) = result {
        log_line(&format!("error: {err}"));
        if show_error_dialog {
            show_error(APP_TITLE, &format!("操作失败：\n{err}"));
        }
        std::process::exit(1);
    }
}
