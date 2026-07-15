use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }

    let resource_compiler = find_resource_compiler()
        .unwrap_or_else(|| panic!("找不到 Windows SDK 的 rc.exe；请安装 Windows SDK 后重新构建"));
    let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing")).join("app.res");
    let status = Command::new(resource_compiler)
        .args(["/nologo", "/fo"])
        .arg(&output)
        .arg("app.rc")
        .status()
        .expect("无法启动 Windows 资源编译器 rc.exe");

    assert!(status.success(), "编译 app.manifest 失败");
    println!("cargo:rustc-link-arg={}", output.display());
}

fn find_resource_compiler() -> Option<PathBuf> {
    let executable = if env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("aarch64") {
        "arm64"
    } else if env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("x86") {
        "x86"
    } else {
        "x64"
    };

    if let Some(path) = env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join("rc.exe"))
            .find(|path| path.is_file())
    }) {
        return Some(path);
    }

    let root = env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("ProgramFiles(x86)")
                .map(|base| PathBuf::from(base).join("Windows Kits\\10"))
        })?;
    let bin_dir = root.join("bin");
    let mut versions = fs::read_dir(bin_dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    versions.sort();

    versions
        .into_iter()
        .rev()
        .map(|version| version.join(executable).join("rc.exe"))
        .find(|path| path.is_file())
}
