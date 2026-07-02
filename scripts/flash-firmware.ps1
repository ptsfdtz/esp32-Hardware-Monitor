param(
  [Parameter(Mandatory = $true)]
  [string]$Port,

  [string]$Fqbn = "esp32:esp32:esp32c3"
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$SketchDir = Join-Path $Root "hardwareMonitor"

arduino-cli upload -p $Port --fqbn $Fqbn $SketchDir
