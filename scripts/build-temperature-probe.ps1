$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$ProbeDir = Join-Path $Root "temp-probe"
$Csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
$OutExe = Join-Path $ProbeDir "TemperatureProbe.exe"

if (!(Test-Path $Csc)) {
  $Csc = "$env:WINDIR\Microsoft.NET\Framework\v4.0.30319\csc.exe"
}

if (!(Test-Path $Csc)) {
  throw "Cannot find .NET Framework csc.exe"
}

& $Csc `
  /nologo `
  /optimize+ `
  /target:exe `
  /platform:x64 `
  /out:$OutExe `
  (Join-Path $ProbeDir "TemperatureProbe.cs")

if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "Built $OutExe"
