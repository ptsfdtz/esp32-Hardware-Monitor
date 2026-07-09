use crate::config::APP_DIR_NAME;
use std::env;
use std::path::PathBuf;

pub fn app_dir() -> Result<PathBuf, String> {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "找不到 LOCALAPPDATA 环境变量".to_string())?;
    Ok(base.join(APP_DIR_NAME))
}

pub fn installed_exe_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("MonitorSetup.exe"))
}

pub fn log_path() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("monitor.log"))
}
