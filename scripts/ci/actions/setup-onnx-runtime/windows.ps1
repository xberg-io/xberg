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
  function Get-OrtPin([string]$Name) {
    $Pins = @(foreach ($Line in Get-Content -LiteralPath $Checksums -ErrorAction Stop) {
      $Fields = $Line.Trim() -split '\s+'
      if ($Fields.Count -ge 2 -and $Fields[1] -ceq $Name) {
        $Fields[0]
      }
    })
    if ($Pins.Count -ne 1 -or $Pins[0] -cnotmatch '\A[a-f0-9]{64}\z') {
      throw "Expected exactly one pinned SHA-256 for $Name"
    }
    $Pins[0]
  }
  function Assert-OrtDigest([string]$Path, [string]$Name, [string]$Pin) {
    $ActualDigest = (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
    if ($ActualDigest -cne $Pin) {
      throw "SHA-256 mismatch for ${Name}: expected $Pin, observed $ActualDigest"
    }
    Write-Host "${Name}: SHA-256 verified"
  }
  $Pin = Get-OrtPin $Archive
  $DownloadPath = Join-Path $env:TEMP $Archive
  $Downloaded = $true
  try {
    Invoke-WebRequest -Uri "https://github.com/microsoft/onnxruntime/releases/download/v$OrtVersion/$Archive" -OutFile $DownloadPath -UseBasicParsing -MaximumRetryCount 5 -RetryIntervalSec 5 -ErrorAction Stop
  } catch {
    Write-Host "::warning::GitHub release download of $Archive failed; using the NuGet package ($($_.Exception.Message))"
    Remove-Item -LiteralPath $DownloadPath -Force -ErrorAction SilentlyContinue
    $Downloaded = $false
  }
  if ($Downloaded) {
    Assert-OrtDigest $DownloadPath $Archive $Pin
    New-Item -ItemType Directory -Path $ExtractRoot -Force | Out-Null
    Expand-Archive -LiteralPath $DownloadPath -DestinationPath $ExtractRoot -Force -ErrorAction Stop
  } else {
    # The NuGet package carries the same binaries; rebuild the release layout from it.
    $Package = "microsoft.ml.onnxruntime.$OrtVersion.nupkg"
    $PackagePin = Get-OrtPin $Package
    $Work = Join-Path $env:TEMP "onnxruntime-nuget-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $Work -Force | Out-Null
    $PackageZip = Join-Path $Work "$Package.zip"
    Invoke-WebRequest -Uri "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/$OrtVersion/$Package" -OutFile $PackageZip -UseBasicParsing -MaximumRetryCount 5 -RetryIntervalSec 5 -ErrorAction Stop
    Assert-OrtDigest $PackageZip $Package $PackagePin
    $Unpacked = Join-Path $Work 'package'
    Expand-Archive -LiteralPath $PackageZip -DestinationPath $Unpacked -Force -ErrorAction Stop
    $Tree = Join-Path $Work 'tree'
    New-Item -ItemType Directory -Path (Join-Path $Tree 'lib'), (Join-Path $Tree 'include') -Force | Out-Null
    Copy-Item -Path (Join-Path $Unpacked "runtimes/win-$ArchId/native/*") -Destination (Join-Path $Tree 'lib') -Force -ErrorAction Stop
    Copy-Item -Path (Join-Path $Unpacked 'build/native/include/*') -Destination (Join-Path $Tree 'include') -Force -ErrorAction Stop
    New-Item -ItemType Directory -Path $ExtractRoot -Force | Out-Null
    Move-Item -LiteralPath $Tree -Destination $OrtRoot -Force -ErrorAction Stop
    Remove-Item -LiteralPath $Work -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Staged ONNX Runtime $OrtVersion (win-$ArchId) from NuGet"
  }
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
