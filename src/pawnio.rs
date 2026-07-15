use crate::paths::app_dir;
use crate::win_util::wide_null;
use std::fs;
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const PAWN_IO_SETUP_EXE: &[u8] = include_bytes!("../temp-probe/vendor/PawnIO_setup.exe");

pub fn install() -> Result<(), String> {
    let app_dir = app_dir()?;
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("无法创建程序目录 {}: {e}", app_dir.display()))?;

    let setup_path = app_dir.join("PawnIO_setup.exe");
    if !file_matches(&setup_path, PAWN_IO_SETUP_EXE) {
        fs::write(&setup_path, PAWN_IO_SETUP_EXE)
            .map_err(|e| format!("无法准备 PawnIO 安装器 {}: {e}", setup_path.display()))?;
    }

    let operation = wide_null("runas");
    let setup_path_wide = wide_null(&setup_path.to_string_lossy());
    let arguments = wide_null("-install");
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            setup_path_wide.as_ptr(),
            arguments.as_ptr(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    let result_code = result as isize;

    if result_code <= 32 {
        return Err(format!(
            "无法启动 PawnIO 安装器，ShellExecute 错误码: {result_code}"
        ));
    }

    Ok(())
}

pub fn is_installed() -> bool {
    const UNINSTALL_KEYS: &[&str] = &[
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PawnIO",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PawnIO",
    ];

    UNINSTALL_KEYS.iter().any(|key| registry_key_exists(key))
}

fn file_matches(path: &Path, expected: &[u8]) -> bool {
    fs::read(path)
        .map(|actual| actual == expected)
        .unwrap_or(false)
}

fn registry_key_exists(key: &str) -> bool {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    let key_wide = wide_null(key);
    let mut handle = null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            key_wide.as_ptr(),
            0,
            KEY_READ,
            &mut handle,
        )
    };

    if status != 0 {
        return false;
    }

    unsafe {
        RegCloseKey(handle);
    }
    true
}
