$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$AgentDir = Join-Path $Root "pc-agent"
$DistDir = Join-Path $Root "dist"
$Exe = Join-Path $AgentDir "target\release\MonitorSetup.exe"
$OutExe = Join-Path $DistDir "MonitorSetup.exe"

& (Join-Path $PSScriptRoot "build-temperature-probe.ps1")
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Push-Location $AgentDir
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
