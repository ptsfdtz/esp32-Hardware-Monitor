param(
  [switch]$Force
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$VendorRoot = Join-Path $Root "temp-probe\vendor"
$InstallDir = Join-Path $VendorRoot "LibreHardwareMonitor"
$ZipPath = Join-Path $VendorRoot "LibreHardwareMonitor.zip"
$PawnIoSetupPath = Join-Path $VendorRoot "PawnIO_setup.exe"
$LibreVersion = "v0.9.6"
$PawnIoSetupUri = "https://raw.githubusercontent.com/LibreHardwareMonitor/LibreHardwareMonitor/$LibreVersion/LibreHardwareMonitor/Resources/PawnIO_setup.exe"
$PawnIoSetupSha256 = "A3A46226C5E2824F4CDD42BE0EECBABFC672C86F7889710F5AB1E6AD385B47A0"

$RequiredFiles = @(
  "LibreHardwareMonitorLib.dll",
  "System.Memory.dll",
  "System.Numerics.Vectors.dll",
  "System.Runtime.CompilerServices.Unsafe.dll",
  "System.Buffers.dll",
  "HidSharp.dll",
  "RAMSPDToolkit-NDD.dll",
  "DiskInfoToolkit.dll",
  "BlackSharp.Core.dll"
)

$hasAllRequiredFiles = $true
foreach ($file in $RequiredFiles) {
  if (!(Test-Path (Join-Path $InstallDir $file))) {
    $hasAllRequiredFiles = $false
    break
  }
}

$hasPawnIoSetup = (Test-Path $PawnIoSetupPath) -and ((Get-FileHash $PawnIoSetupPath -Algorithm SHA256).Hash -eq $PawnIoSetupSha256)

if ($hasAllRequiredFiles -and $hasPawnIoSetup -and !$Force) {
  Write-Host "LibreHardwareMonitor dependencies already exist: $InstallDir"
  exit 0
}

New-Item -ItemType Directory -Force $VendorRoot | Out-Null

$headers = @{ "User-Agent" = "esp32-hardware-monitor-build" }
$release = Invoke-RestMethod `
  -Uri "https://api.github.com/repos/LibreHardwareMonitor/LibreHardwareMonitor/releases/tags/$LibreVersion" `
  -Headers $headers

$asset = $release.assets | Where-Object { $_.name -eq "LibreHardwareMonitor.zip" } | Select-Object -First 1
if (!$asset) {
  throw "Cannot find LibreHardwareMonitor.zip in LibreHardwareMonitor $LibreVersion."
}

Write-Host "Downloading $($asset.browser_download_url)"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ZipPath -Headers $headers

if (Test-Path $InstallDir) {
  Remove-Item -Recurse -Force $InstallDir
}

Expand-Archive -Force $ZipPath $InstallDir

foreach ($file in $RequiredFiles) {
  $path = Join-Path $InstallDir $file
  if (!(Test-Path $path)) {
    throw "LibreHardwareMonitor dependency missing after extract: $file"
  }
}

Write-Host "Downloading PawnIO installer from LibreHardwareMonitor $LibreVersion"
Invoke-WebRequest -Uri $PawnIoSetupUri -OutFile $PawnIoSetupPath -Headers $headers

$pawnIoSha256 = (Get-FileHash $PawnIoSetupPath -Algorithm SHA256).Hash
if ($pawnIoSha256 -ne $PawnIoSetupSha256) {
  throw "Downloaded PawnIO installer checksum mismatch: $PawnIoSetupPath"
}

Write-Host "Prepared LibreHardwareMonitor dependencies: $InstallDir"
