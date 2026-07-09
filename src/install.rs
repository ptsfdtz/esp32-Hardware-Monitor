use crate::config::{APP_TITLE, RUN_VALUE_NAME};
use crate::logging::log_line;
use crate::paths::{app_dir, installed_exe_path, log_path};
use crate::win_util::{show_info, wide_null};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn install_and_launch(show_success: bool) -> Result<(), String> {
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
        show_info(
            APP_TITLE,
            &format!(
                "已安装并启动后台监控。\n\n安装位置：{}\n日志位置：{}",
                installed_exe.display(),
                log_path()?.display()
            ),
        );
    }

    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    remove_autostart()?;
    log_line("autostart removed");
    show_info(
        APP_TITLE,
        "已移除开机自启动。\n\n如果后台程序正在运行，请在任务管理器中结束 MonitorSetup.exe。",
    );
    Ok(())
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
