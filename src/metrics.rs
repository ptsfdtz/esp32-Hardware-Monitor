use crate::win_util::wide_null;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::GetSystemTimes;

#[derive(Clone, Copy)]
pub struct Metrics {
    pub cpu: u8,
    pub gpu: u8,
    pub ram: u8,
}

pub struct MetricsCollector {
    cpu: CpuSampler,
    gpu: Option<GpuSampler>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            cpu: CpuSampler::new(),
            gpu: GpuSampler::new().ok(),
        }
    }

    pub fn sample(&mut self) -> Metrics {
        Metrics {
            cpu: self.cpu.sample().unwrap_or(0),
            gpu: self.gpu.as_mut().and_then(GpuSampler::sample).unwrap_or(0),
            ram: sample_ram_usage().unwrap_or(0),
        }
    }
}

struct CpuSampler {
    previous: Option<SystemTimes>,
}

#[derive(Clone, Copy)]
struct SystemTimes {
    idle: u64,
    kernel: u64,
    user: u64,
}

impl CpuSampler {
    fn new() -> Self {
        Self { previous: None }
    }

    fn sample(&mut self) -> Option<u8> {
        let current = read_system_times()?;
        let previous = self.previous.replace(current)?;

        let idle = current.idle.saturating_sub(previous.idle);
        let kernel = current.kernel.saturating_sub(previous.kernel);
        let user = current.user.saturating_sub(previous.user);
        let total = kernel + user;

        if total == 0 {
            return Some(0);
        }

        let busy = total.saturating_sub(idle);
        Some(percent(busy as f64 * 100.0 / total as f64))
    }
}

fn read_system_times() -> Option<SystemTimes> {
    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }

    Some(SystemTimes {
        idle: filetime_to_u64(idle),
        kernel: filetime_to_u64(kernel),
        user: filetime_to_u64(user),
    })
}

fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

fn sample_ram_usage() -> Option<u8> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return None;
    }

    Some(status.dwMemoryLoad.min(100) as u8)
}

type PdhQuery = *mut c_void;
type PdhCounter = *mut c_void;

const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
const PDH_MORE_DATA: u32 = 0x8000_07D2;
const PDH_STATUS_OK: u32 = 0;

#[repr(C)]
union PdhFmtValue {
    long_value: i32,
    double_value: f64,
    large_value: i64,
    wide_string_value: *const u16,
}

#[repr(C)]
struct PdhFmtCounterValue {
    c_status: u32,
    value: PdhFmtValue,
}

#[repr(C)]
struct PdhFmtCounterValueItemW {
    name: *mut u16,
    value: PdhFmtCounterValue,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(data_source: *const u16, user_data: usize, query: *mut PdhQuery) -> u32;
    fn PdhAddEnglishCounterW(
        query: PdhQuery,
        full_counter_path: *const u16,
        user_data: usize,
        counter: *mut PdhCounter,
    ) -> u32;
    fn PdhCollectQueryData(query: PdhQuery) -> u32;
    fn PdhGetFormattedCounterArrayW(
        counter: PdhCounter,
        format: u32,
        buffer_size: *mut u32,
        item_count: *mut u32,
        item_buffer: *mut PdhFmtCounterValueItemW,
    ) -> u32;
    fn PdhCloseQuery(query: PdhQuery) -> u32;
}

struct GpuSampler {
    query: PdhQuery,
    counter: PdhCounter,
}

impl GpuSampler {
    fn new() -> Result<Self, String> {
        let mut query = null_mut();
        let status = unsafe { PdhOpenQueryW(null(), 0, &mut query) };
        if status != PDH_STATUS_OK {
            return Err(format!("PdhOpenQueryW failed: 0x{status:08x}"));
        }

        let mut counter = null_mut();
        let path = wide_null("\\GPU Engine(*)\\Utilization Percentage");
        let status = unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) };
        if status != PDH_STATUS_OK {
            unsafe {
                PdhCloseQuery(query);
            }
            return Err(format!("PdhAddEnglishCounterW failed: 0x{status:08x}"));
        }

        unsafe {
            PdhCollectQueryData(query);
        }

        Ok(Self { query, counter })
    }

    fn sample(&mut self) -> Option<u8> {
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != PDH_STATUS_OK {
            return None;
        }

        let mut buffer_size = 0;
        let mut item_count = 0;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                null_mut(),
            )
        };

        if status != PDH_MORE_DATA || buffer_size == 0 || item_count == 0 {
            return None;
        }

        let word_count = (buffer_size as usize + std::mem::size_of::<usize>() - 1)
            / std::mem::size_of::<usize>();
        let mut buffer = vec![0_usize; word_count];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut PdhFmtCounterValueItemW,
            )
        };

        if status != PDH_STATUS_OK {
            return None;
        }

        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const PdhFmtCounterValueItemW,
                item_count as usize,
            )
        };

        let mut total_3d = 0.0;
        let mut total_all = 0.0;
        let mut has_3d = false;

        for item in items {
            if item.value.c_status != PDH_STATUS_OK {
                continue;
            }

            let value = unsafe { item.value.value.double_value };
            if !value.is_finite() || value <= 0.0 {
                continue;
            }

            total_all += value;

            let name = wide_ptr_to_string(item.name).to_ascii_lowercase();
            if name.contains("engtype_3d") {
                has_3d = true;
                total_3d += value;
            }
        }

        Some(percent(if has_3d { total_3d } else { total_all }))
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}

fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

fn percent(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }

    value.round().clamp(0.0, 100.0) as u8
}
