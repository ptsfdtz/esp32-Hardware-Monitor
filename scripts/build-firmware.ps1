param(
  [string]$Fqbn = "esp32:esp32:esp32c3:UploadSpeed=115200,CDCOnBoot=cdc,CPUFreq=160,FlashFreq=40,FlashMode=dio,FlashSize=4M,PartitionScheme=default,DebugLevel=none,EraseFlash=all,JTAGAdapter=default,ZigbeeMode=default"
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$SketchDir = Join-Path $Root "hardwareMonitor"

arduino-cli compile --fqbn $Fqbn $SketchDir
