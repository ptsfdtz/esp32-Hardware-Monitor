$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $Root "dist"
$CargoTargetDir = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $Root "target"
} else {
  $env:CARGO_TARGET_DIR
}
$Exe = Join-Path $CargoTargetDir "release\ESP32HardwareMonitor.exe"
$OutExe = Join-Path $DistDir "ESP32HardwareMonitor.exe"
$OutZip = Join-Path $DistDir "ESP32HardwareMonitor-windows-x64.zip"

& (Join-Path $PSScriptRoot "prepare-libre-hardware-monitor.ps1")
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

& (Join-Path $PSScriptRoot "build-temperature-probe.ps1")
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Push-Location $Root
try {
  cargo build --release
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}
finally {
  Pop-Location
}

New-Item -ItemType Directory -Force $DistDir | Out-Null
Copy-Item -Force $Exe $OutExe
Compress-Archive -LiteralPath $OutExe -DestinationPath $OutZip -Force

Write-Host "Built $OutExe"
Write-Host "Built $OutZip"
