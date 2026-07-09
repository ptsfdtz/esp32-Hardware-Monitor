param(
  [switch]$Force
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$VendorRoot = Join-Path $Root "temp-probe\vendor"
$InstallDir = Join-Path $VendorRoot "LibreHardwareMonitor"
$ZipPath = Join-Path $VendorRoot "LibreHardwareMonitor.zip"

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

if ($hasAllRequiredFiles -and !$Force) {
  Write-Host "LibreHardwareMonitor dependencies already exist: $InstallDir"
  exit 0
}

New-Item -ItemType Directory -Force $VendorRoot | Out-Null

$headers = @{ "User-Agent" = "esp32-hardware-monitor-build" }
$release = Invoke-RestMethod `
  -Uri "https://api.github.com/repos/LibreHardwareMonitor/LibreHardwareMonitor/releases/latest" `
  -Headers $headers

$asset = $release.assets | Where-Object { $_.name -eq "LibreHardwareMonitor.zip" } | Select-Object -First 1
if (!$asset) {
  throw "Cannot find LibreHardwareMonitor.zip in latest GitHub release."
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

Write-Host "Prepared LibreHardwareMonitor dependencies: $InstallDir"
