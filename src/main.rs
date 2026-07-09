#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("MonitorSetup only supports Windows.");

mod agent;
mod config;
mod install;
mod logging;
mod metrics;
mod paths;
mod serial_link;
mod temperature;
mod win_util;

use crate::agent::run_agent;
use crate::config::APP_TITLE;
use crate::install::{install_and_launch, uninstall};
use crate::logging::log_line;
use crate::win_util::{show_error, show_info};
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str);

    let result = match command {
        Some("--agent") => run_agent(),
        Some("--install") => install_and_launch(true),
        Some("--uninstall") => uninstall(),
        Some("--help") | Some("-h") => {
            show_info(
                APP_TITLE,
                "双击运行即可安装并启动。\n\n命令：\n--install 安装并启动\n--agent 后台运行\n--uninstall 移除开机自启动",
            );
            Ok(())
        }
        _ => install_and_launch(true),
    };

    if let Err(err) = result {
        log_line(&format!("error: {err}"));
        if command != Some("--agent") {
            show_error(APP_TITLE, &format!("操作失败：\n{err}"));
        }
    }
}
