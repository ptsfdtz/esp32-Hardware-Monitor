param(
  [Parameter(Mandatory = $true)]
  [string]$Port
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$DataDir = Join-Path $Root "hardwareMonitor\data"
$OutDir = Join-Path $Root "dist"
$Image = Join-Path $OutDir "spiffs.bin"
$Arduino15 = Join-Path $env:LOCALAPPDATA "Arduino15"
$Mkspiffs = Join-Path $Arduino15 "packages\esp32\tools\mkspiffs\0.2.3\mkspiffs.exe"
$Esptool = Join-Path $Arduino15 "packages\esp32\tools\esptool_py\5.3.0\esptool.exe"

if (!(Test-Path $DataDir)) {
  throw "Cannot find data directory: $DataDir"
}

if (!(Test-Path $Mkspiffs)) {
  throw "Cannot find mkspiffs.exe: $Mkspiffs"
}

if (!(Test-Path $Esptool)) {
  throw "Cannot find esptool.exe: $Esptool"
}

New-Item -ItemType Directory -Force $OutDir | Out-Null

& $Mkspiffs -c $DataDir -b 4096 -p 256 -s 0x160000 $Image
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

& $Esptool `
  --chip esp32c3 `
  --port $Port `
  --baud 115200 `
  write_flash `
  -z `
  0x290000 `
  $Image

if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "Uploaded SPIFFS image $Image to $Port"
