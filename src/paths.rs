use crate::config::{APP_DIR_NAME, ELEVATED_TEMPERATURE_AGENT_EXE_NAME, INSTALLED_EXE_NAME};
use std::env;
use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{
    FOLDERID_ProgramData, FOLDERID_ProgramFiles, SHGetKnownFolderPath,
};

pub fn app_dir() -> Result<PathBuf, String> {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "找不到 LOCALAPPDATA 环境变量".to_string())?;
    Ok(base.join(APP_DIR_NAME))
}

pub fn installed_exe_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join(INSTALLED_EXE_NAME))
}

pub fn agent_runtime_dir() -> Result<PathBuf, String> {
    let current_exe = env::current_exe().map_err(|e| format!("无法获取当前程序路径: {e}"))?;
    let is_installed_agent = current_exe
        .file_name()
        .map(|name| {
            name.to_string_lossy()
                .eq_ignore_ascii_case(INSTALLED_EXE_NAME)
        })
        .unwrap_or(false);

    if is_installed_agent {
        return current_exe
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| format!("无法获取后台程序目录: {}", current_exe.display()));
    }

    app_dir()
}

pub fn protected_app_dir() -> Result<PathBuf, String> {
    Ok(known_folder(&FOLDERID_ProgramFiles)?.join(APP_DIR_NAME))
}

pub fn elevated_temperature_agent_path() -> Result<PathBuf, String> {
    Ok(protected_app_dir()?.join(ELEVATED_TEMPERATURE_AGENT_EXE_NAME))
}

pub fn elevated_temperature_script_path() -> Result<PathBuf, String> {
    Ok(protected_app_dir()?.join("RegisterElevatedTemperatureTask.ps1"))
}

pub fn elevated_temperature_output_path() -> Result<PathBuf, String> {
    Ok(known_folder(&FOLDERID_ProgramData)?
        .join(APP_DIR_NAME)
        .join("temperature.txt"))
}

pub fn log_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("monitor.log"))
}

fn known_folder(folder: &windows_sys::core::GUID) -> Result<PathBuf, String> {
    let mut raw = null_mut();
    let status = unsafe { SHGetKnownFolderPath(folder, 0, null_mut(), &mut raw) };
    if status < 0 || raw.is_null() {
        return Err(format!("无法获取 Windows 已知目录: 0x{status:08x}"));
    }

    let mut len = 0;
    unsafe {
        while *raw.add(len) != 0 {
            len += 1;
        }
    }
    let path = unsafe {
        PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
            raw, len,
        )))
    };
    unsafe {
        CoTaskMemFree(raw as *const c_void);
    }
    Ok(path)
}
