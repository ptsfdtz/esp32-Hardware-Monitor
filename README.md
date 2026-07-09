# ESP32 Hardware Monitor

电脑端后台程序会采集 Windows 的 CPU / GPU / RAM 使用率，通过 USB 串口发送给 ESP32-C3，由 OLED 显示硬件状态。

后台程序还会尝试通过 LibreHardwareMonitor 读取 CPU / GPU 温度，并发送到第二、第三块 OLED。GPU 温度会适配 Intel / AMD / NVIDIA 主流显卡：有独显时优先显示独显温度，没有独显时使用核显温度；如果当前驱动或硬件没有暴露温度传感器，则发送 `NA`。

## 串口协议

电脑端每秒发送一行文本，波特率 `115200`：

```text
CPU=35;GPU=42;RAM=68;CPU_TEMP=56;GPU_TEMP=61
```

温度不可用时会发送 `NA`，例如 `CPU_TEMP=NA;GPU_TEMP=NA`。

ESP32 固件位于 `hardwareMonitor/hardwareMonitor.ino`。

## OLED 接线

固件使用 PCA9848A I2C 多路复用器，默认地址 `0x70`：

- ESP32-C3 `GPIO4` -> PCA9848A `SDA`
- ESP32-C3 `GPIO5` -> PCA9848A `SCL`
- OLED1 接 PCA9848A 通道 0，显示 CPU / GPU / RAM 占用率
- OLED2 接 PCA9848A 通道 1，显示 CPU 温度
- OLED3 接 PCA9848A 通道 2，显示 GPU 温度

三块 OLED 的 I2C 地址都使用 `0x3C`。

命令行编译固件：

```powershell
.\scripts\build-firmware.ps1
```

烧录固件，按实际串口替换 `COM4`：

```powershell
.\scripts\flash-firmware.ps1 -Port COM4
```

脚本默认使用和当前 Arduino IDE 相同的 ESP32-C3 参数：

```text
UploadSpeed=115200
CDCOnBoot=cdc
FlashFreq=40
FlashMode=dio
EraseFlash=all
```

网页配置页位于：

```text
hardwareMonitor\data\index.html
```

上传网页到 SPIFFS：

```powershell
.\scripts\upload-data.ps1 -Port COM4
```

如果改了网页文件，只需要重新执行 `upload-data.ps1`，不需要重新编译固件。

## Web 配置

ESP32-C3 启动后会先连接默认 WiFi：

```text
SSID: LanaoTech
Password: lanao2025
```

连接成功后会在串口打印路由器分配的 IP，例如：

```text
station ip=192.168.1.23
```

如果连接失败，才会启动配置热点：

```text
SSID: ESP32-Monitor
Password: none
```

连接热点后打开：

```text
http://192.168.4.1/
```

网页里可以保存 WiFi，并切换显示模式：

- 当前样式：OLED1 显示 CPU / GPU / RAM 占用率，OLED2 显示 CPU 温度，OLED3 显示 GPU 温度
- 分屏温度：OLED1 显示 CPU 温度，OLED2 显示 GPU 温度，OLED3 显示 RAM 占用率

保存后的显示模式会写入 ESP32 flash，重启后仍然保留。

如果 PCA9848A 模块改过地址脚，请同步修改 `hardwareMonitor/Config.h` 里的 `PCA9848A_ADDR`。

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

温度读取依赖会在构建时下载 LibreHardwareMonitor，并打包进 `MonitorSetup.exe`。后台启动后会自动释放到：

```text
%LOCALAPPDATA%\ESP32HardwareMonitor\LibreHardwareMonitor\
```

如果想使用自己下载的 LibreHardwareMonitor，可以设置环境变量 `LIBRE_HARDWARE_MONITOR_DIR` 指向它。日志中出现 `libre_missing` 表示依赖目录不存在；日志中出现 `CPU_TEMP=NA;GPU_TEMP=NA` 表示程序路径已经打通，但当前权限/硬件/驱动没有暴露温度传感器。Intel UHD / Iris 核显常见情况是能读取频率、功耗、占用率，但没有单独的 GPU 温度传感器，此时第三块 OLED 会显示空值。

卸载开机自启动：

```powershell
%LOCALAPPDATA%\ESP32HardwareMonitor\MonitorSetup.exe --uninstall
```
