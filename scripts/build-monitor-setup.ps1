$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $Root "dist"
$Exe = Join-Path $Root "target\release\MonitorSetup.exe"
$OutExe = Join-Path $DistDir "MonitorSetup.exe"

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

Write-Host "Built $OutExe"
