param(
  [string]$Fqbn = "esp32:esp32:esp32c3"
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$SketchDir = Join-Path $Root "hardwareMonitor"

arduino-cli compile --fqbn $Fqbn $SketchDir
