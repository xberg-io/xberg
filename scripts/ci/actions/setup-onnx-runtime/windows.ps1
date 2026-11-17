$OrtVersion = $args[0]
if ([string]::IsNullOrWhiteSpace($OrtVersion)) { throw "Usage: windows.ps1 <ortVersion> [destDir] [archId] [strategy]" }

if ($OrtVersion -cnotmatch '\A[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?\z') {
  throw "Invalid ONNX Runtime version: $OrtVersion"
}

$DestDir = if ($args.Count -ge 2 -and -not [string]::IsNullOrWhiteSpace($args[1])) { $args[1] } else { "crates/xberg-node" }
$ArchId = if ($args.Count -ge 3) { $args[2] } else { "" }
$Strategy = if ($args.Count -ge 4 -and -not [string]::IsNullOrWhiteSpace($args[3])) { $args[3] } else { "system" }

$ExtractRoot = Join-Path $env:TEMP "onnxruntime"
if ([string]::IsNullOrWhiteSpace($ArchId)) {
  $ArchId = $env:RUNNER_ARCH
}
$ArchId = $ArchId.ToLowerInvariant()
if ($ArchId -eq "arm64") { $ArchId = "arm64" } else { $ArchId = "x64" }

$OrtRoot = Join-Path $ExtractRoot "onnxruntime-win-$ArchId-$OrtVersion"
$OrtBin = Join-Path $OrtRoot 'bin'
$OrtLib = Join-Path $OrtRoot 'lib'

if (-Not (Test-Path $OrtRoot)) {
  Write-Host "Cache miss: Downloading ONNX Runtime $OrtVersion"
  $Archive = "onnxruntime-win-$ArchId-$OrtVersion.zip"
  # A custom version needs a trusted manifest with its archive's digest.
  $Checksums = if ($env:ORT_CHECKSUMS_FILE) { $env:ORT_CHECKSUMS_FILE } else { Join-Path $PSScriptRoot 'checksums.sha256' }
  $Pins = @(foreach ($Line in Get-Content -LiteralPath $Checksums -ErrorAction Stop) {
    $Fields = $Line.Trim() -split '\s+'
    if ($Fields.Count -ge 2 -and $Fields[1] -ceq $Archive) {
      $Fields[0]
    }
  })
  if ($Pins.Count -ne 1 -or $Pins[0] -cnotmatch '\A[a-f0-9]{64}\z') {
    throw "Expected exactly one pinned SHA-256 for $Archive"
  }
  $DownloadPath = Join-Path $env:TEMP $Archive
  Invoke-WebRequest -Uri "https://github.com/microsoft/onnxruntime/releases/download/v$OrtVersion/$Archive" -OutFile $DownloadPath -UseBasicParsing -MaximumRetryCount 5 -RetryIntervalSec 5 -ErrorAction Stop
  $ActualDigest = (Get-FileHash -LiteralPath $DownloadPath -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
  if ($ActualDigest -cne $Pins[0]) {
    throw "SHA-256 mismatch for ${Archive}: expected $($Pins[0]), observed $ActualDigest"
  }
  Write-Host "${Archive}: SHA-256 verified"
  New-Item -ItemType Directory -Path $ExtractRoot -Force | Out-Null
  Expand-Archive -LiteralPath $DownloadPath -DestinationPath $ExtractRoot -Force -ErrorAction Stop
} else {
  # The extracted-tree cache does not retain the archive for re-verification.
  Write-Host "Cache hit: Using cached ONNX Runtime $OrtVersion"
}

if (!(Test-Path $OrtLib)) {
  Write-Error "ERROR: ONNX Runtime lib directory missing at $OrtLib"
  Get-ChildItem -Path $ExtractRoot -Recurse | Write-Host
  exit 1
}

$LibFiles = @(Get-ChildItem -Path $OrtLib -Filter "*.lib" -ErrorAction SilentlyContinue)
if ($LibFiles.Count -eq 0) {
  Write-Error "ERROR: No ONNX Runtime library files found in $OrtLib"
  Get-ChildItem -Path $OrtLib | Write-Host
  exit 1
}

$DllDirs = @()
foreach ($Candidate in @($OrtLib, $OrtBin)) {
  if (Test-Path $Candidate) {
    $CandidateDlls = @(Get-ChildItem -Path $Candidate -Filter "*.dll" -File -ErrorAction SilentlyContinue)
    if ($CandidateDlls.Count -gt 0) {
      $DllDirs += $Candidate
    }
  }
}
if ($DllDirs.Count -eq 0) {
  $OrtDll = Get-ChildItem -Path $OrtRoot -Recurse -Filter "onnxruntime.dll" -File -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($OrtDll) { $DllDirs += $OrtDll.DirectoryName }
}
if ($DllDirs.Count -eq 0) {
  $AnyDll = Get-ChildItem -Path $OrtRoot -Recurse -Filter "*.dll" -File -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($AnyDll) { $DllDirs += $AnyDll.DirectoryName }
}
$DllDirs = @($DllDirs | Select-Object -Unique)
if ($DllDirs.Count -eq 0) {
  Write-Error "ERROR: No ONNX Runtime runtime DLLs found under $OrtRoot"
  Get-ChildItem -Path $OrtRoot -Recurse | Write-Host
  exit 1
}

$Dest = Join-Path $env:GITHUB_WORKSPACE $DestDir
New-Item -ItemType Directory -Path $Dest -Force | Out-Null
Copy-Item -Path (Join-Path $OrtLib '*') -Destination $Dest -Force
foreach ($Dir in $DllDirs) {
  Copy-Item -Path (Join-Path $Dir '*.dll') -Destination $Dest -Force
}

$RustFlags = if ($env:RUSTFLAGS) { "$env:RUSTFLAGS -L $OrtLib" } else { "-L $OrtLib" }

if ($Strategy -eq "bundled") {
  Write-Host "Using bundled ORT strategy (Windows) - dynamic linking against pre-downloaded ORT (no static binaries for windows-gnu)"
  @(
    "ORT_LIB_LOCATION=$OrtLib"
    "ORT_PREFER_DYNAMIC_LINK=1"
    "RUSTFLAGS=$RustFlags"
    "LIB=$OrtLib;$env:LIB"
    "LIBRARY_PATH=$OrtLib;$env:LIBRARY_PATH"
    "PATH=$Dest;$env:PATH"
  ) | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
} else {
  @(
    "ORT_LIB_LOCATION=$OrtLib"
    "ORT_PREFER_DYNAMIC_LINK=1"
    "ORT_SKIP_DOWNLOAD=1"
    "ORT_STRATEGY=system"
    "ORT_DYLIB_PATH=$Dest\onnxruntime.dll"
    "RUSTFLAGS=$RustFlags"
    "LIB=$OrtLib;$env:LIB"
    "LIBRARY_PATH=$OrtLib;$env:LIBRARY_PATH"
    "PATH=$Dest;$env:PATH"
  ) | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
}
