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
$OutPartsDir = Join-Path $DistDir "release-package-parts"

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

$zipStream = [System.IO.MemoryStream]::new()
try {
  $archive = [System.IO.Compression.ZipArchive]::new(
    $zipStream,
    [System.IO.Compression.ZipArchiveMode]::Create,
    $true
  )
  try {
    $entry = $archive.CreateEntry(
      "ESP32HardwareMonitor.exe",
      [System.IO.Compression.CompressionLevel]::Optimal
    )
    $entryStream = $entry.Open()
    try {
      $exeStream = [System.IO.File]::OpenRead($OutExe)
      try {
        $exeStream.CopyTo($entryStream)
      }
      finally {
        $exeStream.Dispose()
      }
    }
    finally {
      $entryStream.Dispose()
    }
  }
  finally {
    $archive.Dispose()
  }
  $zipBytes = $zipStream.ToArray()
}
finally {
  $zipStream.Dispose()
}

$base64 = [System.Convert]::ToBase64String($zipBytes)
New-Item -ItemType Directory -Force $OutPartsDir | Out-Null
Get-ChildItem -LiteralPath $OutPartsDir -Filter "*.part" -File -ErrorAction SilentlyContinue |
  Remove-Item -Force
$chunkSize = 65536
$partCount = 0
for ($offset = 0; $offset -lt $base64.Length; $offset += $chunkSize) {
  $length = [System.Math]::Min($chunkSize, $base64.Length - $offset)
  $partPath = Join-Path $OutPartsDir ("{0:D4}.part" -f $partCount)
  [System.IO.File]::WriteAllText(
    $partPath,
    $base64.Substring($offset, $length),
    [System.Text.Encoding]::ASCII
  )
  $partCount++
}
[System.IO.File]::WriteAllBytes($OutZip, $zipBytes)

Write-Host "Built $OutExe"
Write-Host "Built $OutZip"
Write-Host "Encoded package into $partCount parts at $OutPartsDir"
