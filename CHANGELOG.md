# 更新日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 记录发行版本。

## [1.0.0] - 2026-07-15

### 正式发布

- 发布 Windows x64 正式版，双击后静默安装并以当前用户身份开机自启动。
- 采集 CPU、GPU、内存使用率，并通过串口发送给 ESP32-C3 OLED 显示器。
- 使用 LibreHardwareMonitor 读取 CPU / GPU 温度；有独立显卡时优先使用独显温度。
- 为需要 PawnIO 的 AMD Ryzen 设备提供一次性高权限温度助手设置，后续登录保持无感后台运行。

### 发布保障

- Windows 可执行文件及温度助手写入 `1.0.0.0` 文件版本信息。
- GitHub Release 同时提供 SHA-256 校验文件。
