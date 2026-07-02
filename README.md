# ESP32 Hardware Monitor

电脑端后台程序会采集 Windows 的 CPU / GPU / RAM 使用率，通过 USB 串口发送给 ESP32-C3，由 OLED 显示硬件状态。

后台程序还会尝试通过 LibreHardwareMonitor 读取 CPU / GPU 温度，并写入日志；温度目前不发送到 OLED。

## 串口协议

电脑端每秒发送一行文本，波特率 `115200`：

```text
CPU=35;GPU=42;RAM=68
```

ESP32 固件位于 `hardwareMonitor/hardwareMonitor.ino`。

## 构建 Windows 程序

需要 Rust 工具链。运行：

```powershell
.\scripts\build-monitor-setup.ps1
```

生成文件：

```text
dist\MonitorSetup.exe
```

## 使用

双击 `MonitorSetup.exe` 后会：

- 复制自身到 `%LOCALAPPDATA%\ESP32HardwareMonitor\MonitorSetup.exe`
- 写入当前用户的开机自启动
- 启动后台监控进程
- 自动扫描 ESP32-C3 / Espressif / 常见 USB 串口并发送数据

日志文件：

```text
%LOCALAPPDATA%\ESP32HardwareMonitor\monitor.log
```

温度读取默认依赖：

```text
C:\Users\user\Downloads\LibreHardwareMonitor\LibreHardwareMonitorLib.dll
```

如果 LibreHardwareMonitor 放在其他目录，可以设置环境变量 `LIBRE_HARDWARE_MONITOR_DIR` 指向它。日志中出现 `CPU_TEMP=NA;GPU_TEMP=NA` 表示程序路径已经打通，但当前权限/硬件/驱动没有暴露温度传感器。

卸载开机自启动：

```powershell
%LOCALAPPDATA%\ESP32HardwareMonitor\MonitorSetup.exe --uninstall
```
