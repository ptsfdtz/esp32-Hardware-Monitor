use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
};

pub struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    pub fn new(name: &str) -> Result<Self, String> {
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

pub fn show_info(title: &str, message: &str) {
    show_message(title, message, MB_ICONINFORMATION);
}

pub fn show_error(title: &str, message: &str) {
    show_message(title, message, MB_ICONERROR);
}

fn show_message(title: &str, message: &str, flags: u32) {
    let title = wide_null(title);
    let message = wide_null(message);
    unsafe {
        MessageBoxW(null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | flags);
    }
}

pub fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
