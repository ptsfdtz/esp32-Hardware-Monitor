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
$OutZipBase64 = "$OutZip.b64"

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

[System.IO.File]::WriteAllText(
  $OutZipBase64,
  [System.Convert]::ToBase64String($zipBytes),
  [System.Text.Encoding]::ASCII
)
[System.IO.File]::WriteAllBytes($OutZip, $zipBytes)

Write-Host "Built $OutExe"
Write-Host "Built $OutZip"
Write-Host "Encoded $OutZipBase64"
