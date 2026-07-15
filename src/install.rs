use crate::config::{APP_TITLE, RUN_VALUE_NAME};
use crate::logging::log_line;
use crate::paths::{
    app_dir, elevated_temperature_agent_path, elevated_temperature_output_path,
    elevated_temperature_script_path, installed_exe_path, log_path, protected_app_dir,
};
use crate::temperature::install_elevated_runtime_files;
use crate::win_util::{show_info, wide_null};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, ERROR_NOT_ALL_ASSIGNED, LUID,
};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    SE_RESTORE_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcessToken, CREATE_NO_WINDOW,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const ELEVATED_TASK_SCRIPT: &str =
    include_str!("../temp-probe/RegisterElevatedTemperatureTask.ps1");
const SYSTEM_SID: &str = "*S-1-5-18";
const ADMINISTRATORS_SID: &str = "*S-1-5-32-544";
const USERS_SID: &str = "*S-1-5-32-545";

pub fn install_and_launch(show_success: bool) -> Result<(), String> {
    let app_dir = app_dir()?;
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("无法创建安装目录 {}: {e}", app_dir.display()))?;

    let installed_exe = installed_exe_path()?;
    let current_exe = env::current_exe().map_err(|e| format!("无法获取当前程序路径: {e}"))?;

    if !same_path(&current_exe, &installed_exe) {
        copy_with_retry(&current_exe, &installed_exe)?;
        log_line(&format!("installed to {}", installed_exe.display()));
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

pub fn uninstall(show_success: bool) -> Result<(), String> {
    remove_autostart()?;
    log_line("autostart removed");

    if show_success {
        show_info(
            APP_TITLE,
            "已移除开机自启动。\n\n后台监控会在本次会话结束后停止。",
        );
    }

    Ok(())
}

pub fn request_elevated_temperature_install() -> Result<(), String> {
    request_elevated_operation("--install-elevated-temperature")
}

pub fn request_elevated_temperature_removal() -> Result<(), String> {
    request_elevated_operation("--remove-elevated-temperature")
}

fn request_elevated_operation(arguments: &str) -> Result<(), String> {
    let current_exe = env::current_exe().map_err(|e| format!("无法获取当前程序路径: {e}"))?;
    let operation = wide_null("runas");
    let executable = wide_null(&current_exe.to_string_lossy());
    let arguments = wide_null(arguments);
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            executable.as_ptr(),
            arguments.as_ptr(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    let result_code = result as isize;

    if result_code <= 32 {
        return Err(format!(
            "无法请求一次性温度权限设置，ShellExecute 错误码: {result_code}"
        ));
    }

    Ok(())
}

pub fn install_elevated_temperature() -> Result<(), String> {
    let protected_dir = protected_app_dir()?;
    let output_path = elevated_temperature_output_path()?;
    let output_dir = output_path
        .parent()
        .ok_or_else(|| format!("无法获取温度输出目录: {}", output_path.display()))?;

    prepare_protected_directory(&protected_dir, false)?;
    install_elevated_runtime_files(&protected_dir)?;

    let script_path = elevated_temperature_script_path()?;
    write_if_changed(&script_path, ELEVATED_TASK_SCRIPT.as_bytes())?;
    prepare_protected_directory(&protected_dir, false)?;

    prepare_protected_directory(output_dir, true)?;
    fs::write(
        &output_path,
        "VERSION=1;TIMESTAMP=0;CPU_TEMP=NA;GPU_TEMP=NA\n",
    )
    .map_err(|e| format!("无法初始化温度输出 {}: {e}", output_path.display()))?;
    prepare_protected_directory(output_dir, true)?;

    let agent_path = elevated_temperature_agent_path()?;
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .arg("-ProgramDirectory")
        .arg(&protected_dir)
        .arg("-AgentPath")
        .arg(&agent_path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("无法启动计划任务注册器: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(format!(
            "无法注册受保护的温度任务（code={:?}）：{} {}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    log_line("elevated temperature task installed");
    Ok(())
}

pub fn remove_elevated_temperature() -> Result<(), String> {
    let script_path = elevated_temperature_script_path()?;
    if !script_path.is_file() {
        return Ok(());
    }

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .arg("-Remove")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("无法启动计划任务移除器: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(format!("无法移除受保护的温度任务：{} {}", stdout, stderr));
    }

    log_line("elevated temperature task removed");
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

fn prepare_protected_directory(dir: &Path, allow_users_read: bool) -> Result<(), String> {
    reject_reparse_points(dir)?;
    fs::create_dir_all(dir).map_err(|e| format!("无法创建受保护目录 {}: {e}", dir.display()))?;
    reject_reparse_points(dir)?;

    enable_restore_privilege()?;
    run_icacls(
        dir,
        [
            "/setowner".to_string(),
            SYSTEM_SID.to_string(),
            "/T".to_string(),
        ],
    )?;

    let mut arguments = vec![
        "/inheritance:r".to_string(),
        "/grant:r".to_string(),
        format!("{SYSTEM_SID}:F"),
        format!("{ADMINISTRATORS_SID}:F"),
        format!("{SYSTEM_SID}:(OI)(CI)F"),
        format!("{ADMINISTRATORS_SID}:(OI)(CI)F"),
    ];
    if allow_users_read {
        arguments.push(format!("{USERS_SID}:RX"));
        arguments.push(format!("{USERS_SID}:(OI)(CI)RX"));
    }
    arguments.push("/T".to_string());
    run_icacls(dir, arguments)?;

    reject_reparse_points(dir)
}

fn run_icacls<I>(dir: &Path, arguments: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let output = Command::new("icacls.exe")
        .arg(dir)
        .args(arguments)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("无法设置目录权限 {}: {e}", dir.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(format!(
        "无法保护目录 {}: {} {}",
        dir.display(),
        stdout,
        stderr
    ))
}

fn enable_restore_privilege() -> Result<(), String> {
    let mut token = null_mut();
    let opened = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    };
    if opened == 0 || token.is_null() {
        return Err(format!("无法打开提升后的访问令牌: {}", unsafe {
            GetLastError()
        }));
    }

    let result = (|| {
        let mut luid = LUID {
            LowPart: 0,
            HighPart: 0,
        };
        if unsafe { LookupPrivilegeValueW(null(), SE_RESTORE_NAME, &mut luid) } == 0 {
            return Err(format!("无法查找 SeRestorePrivilege: {}", unsafe {
                GetLastError()
            }));
        }

        let mut privileges = TOKEN_PRIVILEGES::default();
        privileges.PrivilegeCount = 1;
        privileges.Privileges[0] = LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        };

        unsafe {
            SetLastError(0);
        }
        if unsafe { AdjustTokenPrivileges(token, 0, &privileges, 0, null_mut(), null_mut()) } == 0 {
            return Err(format!("无法启用 SeRestorePrivilege: {}", unsafe {
                GetLastError()
            }));
        }
        let status = unsafe { GetLastError() };
        if status == ERROR_NOT_ALL_ASSIGNED {
            return Err(
                "当前进程没有 SeRestorePrivilege；请在 Windows 权限确认中选择“是”后重试。"
                    .to_string(),
            );
        }
        Ok(())
    })();

    unsafe {
        CloseHandle(token);
    }
    result
}

fn reject_reparse_points(path: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    use std::os::windows::fs::MetadataExt;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!("拒绝使用重解析点目录: {}", path.display()));
    }

    if metadata.is_dir() {
        for entry in
            fs::read_dir(path).map_err(|e| format!("无法检查目录 {}: {e}", path.display()))?
        {
            let entry = entry.map_err(|e| format!("无法检查目录项 {}: {e}", path.display()))?;
            reject_reparse_points(&entry.path())?;
        }
    }

    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if fs::read(path)
        .map(|current| current == bytes)
        .unwrap_or(false)
    {
        return Ok(());
    }
    fs::write(path, bytes).map_err(|e| format!("无法写入 {}: {e}", path.display()))
}

fn copy_with_retry(source: &Path, destination: &Path) -> Result<(), String> {
    const ATTEMPTS: u32 = 20;
    let mut last_error = None;

    for attempt in 0..ATTEMPTS {
        match fs::copy(source, destination) {
            Ok(_) => return Ok(()),
            Err(error) if is_retryable_file_lock(&error) && attempt + 1 < ATTEMPTS => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(250));
            }
            Err(error) if destination.exists() => {
                return Err(format!(
                    "无法更新已安装的程序 {}: {error}。请退出后台监控后重试。",
                    destination.display()
                ));
            }
            Err(error) => {
                return Err(format!(
                    "无法复制程序到安装目录 {}: {error}",
                    destination.display()
                ));
            }
        }
    }

    let error = last_error.expect("copy retry must retain the final error");
    Err(format!(
        "无法更新已安装的程序 {}: {error}。请退出后台监控后重试。",
        destination.display()
    ))
}

fn is_retryable_file_lock(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}
