$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$ProbeDir = Join-Path $Root "temp-probe"
$Csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
$OutExe = Join-Path $ProbeDir "TemperatureProbe.exe"
$ElevatedAgentExe = Join-Path $ProbeDir "ElevatedTemperatureAgent.exe"
$AssemblyInfo = Join-Path $ProbeDir "AssemblyInfo.cs"

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
  $AssemblyInfo `
  (Join-Path $ProbeDir "TemperatureProbe.cs")

if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

& $Csc `
  /nologo `
  /optimize+ `
  /target:winexe `
  /platform:x64 `
  /out:$ElevatedAgentExe `
  $AssemblyInfo `
  (Join-Path $ProbeDir "ElevatedTemperatureAgent.cs")

if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

& $OutExe --self-test
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host "Built $OutExe"
Write-Host "Built $ElevatedAgentExe"
