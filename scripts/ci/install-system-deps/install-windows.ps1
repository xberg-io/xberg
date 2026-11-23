Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "::group::Installing Windows dependencies"

function Retry-Command {
  param(
    [scriptblock]$Command,
    [int]$MaxAttempts = 3,
    [int]$DelaySeconds = 5
  )

  $attempt = 1
  while ($attempt -le $MaxAttempts) {
    try {
      Write-Host "Attempt $attempt of $MaxAttempts..."
      $global:LASTEXITCODE = 0
      & $Command | Out-Host
      if ($LASTEXITCODE -ne 0) { throw "Native dependency command failed with exit $LASTEXITCODE" }
      return $true
    }
    catch {
      $attempt++
      if ($attempt -le $MaxAttempts) {
        $backoffDelay = $DelaySeconds * [Math]::Pow(2, $attempt - 1)
        Write-Host "⚠ Attempt failed, retrying in ${backoffDelay}s..." -ForegroundColor Yellow
        Start-Sleep -Seconds $backoffDelay
      }
      else {
        return $false
      }
    }
  }
}

function Assert-LibheifVersion {
  param([string[]]$Packages, [string[]]$X265Packages = @())
  # `vcpkg list` normally prints the core row with its version and each enabled
  # feature as a separate version-less row, e.g. `libheif[aom]:x64-windows`, but
  # some vcpkg versions collapse the features onto the versioned row
  # (`libheif[aom,libde265]:x64-windows 1.23.7#1`). Accept either shape; the
  # feature set is parsed from every feature row below. ~keep
  $coreRows = @($Packages | Where-Object { $_ -match '^libheif(?:\[[^\]]*\])?:x64-windows\s+1\.23\.7(?:#\d+)?\s' })
  if ($coreRows.Count -ne 1) {
    throw 'Expected exactly one installed libheif 1.23.7 x64-windows port'
  }
  Write-Host "Verified $($coreRows[0])"

  $features = @()
  foreach ($row in @($Packages | Where-Object { $_ -match '^libheif\[([^\]]*)\]:x64-windows(?:\s|$)' })) {
    if ($row -match '^libheif\[([^\]]*)\]:') {
      $features += @($Matches[1] -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    }
  }
  $features = @($features | Where-Object { $_ -ne 'core' } | Select-Object -Unique)
  if ($features -contains 'hevc' -or $features -contains 'x265') {
    throw "libheif resolved the GPL-2.0 x265 encoder via the 'hevc' feature (features: $($features -join ','))"
  }
  if ($X265Packages.Count -ne 0) {
    throw "vcpkg installed the GPL-2.0 x265 port alongside libheif: $($X265Packages[0])"
  }
  $expected = @('aom', 'libde265')
  $unexpected = @($features | Where-Object { $expected -notcontains $_ })
  $missing = @($expected | Where-Object { $features -notcontains $_ })
  if ($unexpected.Count -ne 0 -or $missing.Count -ne 0) {
    throw "libheif resolved features '$($features -join ',')'; expected exactly 'aom,libde265'"
  }
  Write-Host "Verified decode-only libheif features: $($features -join ',')"
}

function Assert-LibheifArtifacts {
  param([string]$VcpkgRoot)
  # The metadata assertions above cannot see a cached tree built before the triplet
  # change. Check the files the linker, the loader and the release packaging read.
  # The static-md tree may also be present (boost/zlib) and is deliberately ignored. ~keep
  $bin = Join-Path $VcpkgRoot 'installed\x64-windows\bin'
  $lib = Join-Path $VcpkgRoot 'installed\x64-windows\lib'
  $share = Join-Path $VcpkgRoot 'installed\x64-windows\share'
  foreach ($dll in @('heif.dll', 'libde265.dll', 'aom.dll')) {
    $path = Join-Path $bin $dll
    if (-not (Test-Path -LiteralPath $path)) {
      throw "$path is missing -- the dynamic libheif triplet did not install its runtime DLLs"
    }
  }
  $importLib = Join-Path $lib 'heif.lib'
  if (-not (Test-Path -LiteralPath $importLib)) {
    throw "$importLib is missing -- libheif-sys cannot link the import library"
  }
  foreach ($port in @('libheif', 'libde265', 'aom')) {
    $copyright = Join-Path $share "$port\copyright"
    if (-not (Test-Path -LiteralPath $copyright)) {
      throw "$copyright is missing -- the release archive cannot ship the $port license text"
    }
  }
  $x265 = @(
    foreach ($dir in @($bin, $lib)) {
      Get-ChildItem -LiteralPath $dir -Filter '*x265*' -ErrorAction SilentlyContinue
    }
  )
  if ($x265.Count -ne 0) {
    throw "The dynamic vcpkg tree carries a GPL-2.0 x265 file: $($x265[0].FullName)"
  }
  Write-Host "✓ dynamic libheif artifacts verified: $bin"
}

$tesseractCacheHit = $env:TESSERACT_CACHE_HIT -eq "true"
$llvmCacheHit = $env:LLVM_CACHE_HIT -eq "true"
. "$PSScriptRoot/ensure-tesseract-windows.ps1"
. "$PSScriptRoot/ensure-build-tools-windows.ps1"
$cmakeCacheHit = $env:CMAKE_CACHE_HIT -eq "true"
$cmakeInstalled = $false

Write-Host "Cache status:"
Write-Host "  TESSERACT_CACHE_HIT: $env:TESSERACT_CACHE_HIT (evaluated: $tesseractCacheHit)"
Write-Host "  LLVM_CACHE_HIT: $env:LLVM_CACHE_HIT (evaluated: $llvmCacheHit)"
Write-Host "  CMAKE_CACHE_HIT: $env:CMAKE_CACHE_HIT (evaluated: $cmakeCacheHit)"
Write-Host ""
try {
  & cmake --version 2>$null
  Write-Host "✓ CMake already installed"
  $cmakeInstalled = $true
}
catch {
  Write-Host "CMake not found, will attempt to install"
}

Ensure-BuildTool -Package 'llvm' -Executable 'C:\Program Files\LLVM\bin\clang.exe' -Version '23.1.3'

$libheifCacheHit = $env:LIBHEIF_CACHE_HIT -eq "true"
if (-not $libheifCacheHit) {
  Write-Host "libheif/boost/zlib cache miss, installing via vcpkg..."
  $vcpkgRoot = 'C:\xberg-vcpkg-5363acb7'
  $vcpkgExe = Join-Path $vcpkgRoot "vcpkg.exe"
  if (-not (Test-Path (Join-Path $vcpkgRoot '.git'))) {
    & git clone --filter=blob:none --no-checkout https://github.com/microsoft/vcpkg.git $vcpkgRoot
    if ($LASTEXITCODE -ne 0) { throw 'Failed to clone the dedicated vcpkg root' }
  }
  $baseline = '5363acb796e576328288fae60c25304e227e9839'
  & git -C $vcpkgRoot fetch --depth=1 origin $baseline
  if ($LASTEXITCODE -ne 0) { throw 'Failed to fetch the verified vcpkg baseline' }
  & git -C $vcpkgRoot checkout --detach $baseline
  if ($LASTEXITCODE -ne 0) { throw 'Failed to select the verified vcpkg baseline' }
  & (Join-Path $vcpkgRoot 'bootstrap-vcpkg.bat') -disableMetrics
  if ($LASTEXITCODE -ne 0) { throw 'Failed to bootstrap the verified vcpkg baseline' }
  # The port's default features are `hevc` (HEVC encoding via x265, GPL-2.0-or-later)
  # and `libde265`. `core` suppresses both defaults so only the AV1/HEIC decoders are
  # linked; HEIF output reports EncoderUnavailable instead. Re-adding a plain
  # `libheif` here silently restores the GPL encoder.
  # The dynamic triplet is on purpose: libheif and libde265 are LGPL-3.0, so the CLI
  # links replaceable DLLs and ships their texts instead of embedding the code in
  # xberg.exe (#3339). zlib is installed in the same triplet only so libz-sys's vcpkg
  # probe succeeds under VCPKGRS_TRIPLET=x64-windows and it does not fall back to a
  # vendored zlib beside the static zlib xberg-libwpd links. xberg.exe therefore
  # imports the vcpkg zlib DLL (z.dll), which the CLI packaging step stages explicitly;
  # the archive closure gate rejects a ZIP whose executable imports it without it. ~keep
  if (-not (Retry-Command { & $vcpkgExe install "libheif[core,aom,libde265]:x64-windows" "zlib:x64-windows" --recurse } -MaxAttempts 2)) {
    throw "Failed to install libheif via the verified vcpkg baseline"
  }
  else {
    Write-Host "✓ libheif installed via vcpkg (x64-windows, dynamic)"
  }
  # boost-spirit + boost-serialization (header-only, used by librevenge/libwpd:
  # RVNGBinaryData.cpp needs boost/archive/iterators/*) and zlib (linked
  # by librevenge's zip stream) — needed by xberg-libwpd's build.rs.
  # boost/zlib stay on the static triplet: xberg-libwpd links them into its own static
  # archive and probes `installed\x64-windows-static-md\lib` for zlibstatic.lib. ~keep
  if (-not (Retry-Command { & $vcpkgExe install "boost-spirit:x64-windows-static-md" "boost-serialization:x64-windows-static-md" "zlib:x64-windows-static-md" --recurse } -MaxAttempts 2)) {
    throw "Failed to install boost/zlib via the verified vcpkg baseline"
  }
  else {
    Write-Host "✓ boost-spirit + boost-serialization + zlib installed via vcpkg (x64-windows-static-md)"
  }
  Add-Content -Path $env:GITHUB_ENV -Value "VCPKG_ROOT=$vcpkgRoot"
  Add-Content -Path $env:GITHUB_ENV -Value "VCPKG_INSTALLATION_ROOT=$vcpkgRoot"
}
else {
  Write-Host "✓ libheif/boost/zlib found in cache"
  $vcpkgRoot = 'C:\xberg-vcpkg-5363acb7'
  Add-Content -Path $env:GITHUB_ENV -Value "VCPKG_ROOT=$vcpkgRoot"
  Add-Content -Path $env:GITHUB_ENV -Value "VCPKG_INSTALLATION_ROOT=$vcpkgRoot"
}

$actualBaseline = & git -C $vcpkgRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $actualBaseline -ne '5363acb796e576328288fae60c25304e227e9839') {
  throw 'The cached vcpkg checkout does not match its verified baseline'
}
$vcpkgExe = Join-Path $vcpkgRoot 'vcpkg.exe'
# `vcpkg list` prints one row per installed feature and filters by substring of the
# display name, so query the bare port name: `libheif:triplet` would return only the
# core row and the feature assertion below would see an empty set. ~keep
$heifPackages = @(& $vcpkgExe list 'libheif')
if ($LASTEXITCODE -ne 0) { throw 'Failed to query the installed libheif port' }
# A no-match query may print an informational line and still exit 0, so keep only
# real package rows rather than treating any output as a hit. ~keep
$x265Packages = @(& $vcpkgExe list 'x265:x64-windows' | Where-Object { $_ -match '^x265' })
Assert-LibheifVersion -Packages $heifPackages -X265Packages $x265Packages

# A cached tree can predate the triplet change and a vcpkg call that merely does not
# throw proves nothing about the files the linker, the loader and the packaging read
# (same trap as the zlib probe below). ~keep
Assert-LibheifArtifacts -VcpkgRoot $vcpkgRoot
$heifDllDir = Join-Path $vcpkgRoot 'installed\x64-windows\bin'
$heifNoticeDir = Join-Path $vcpkgRoot 'installed\x64-windows\share'

Add-Content -Path $env:GITHUB_ENV -Value "XBERG_LIBHEIF_DLL_DIR=$heifDllDir"
Add-Content -Path $env:GITHUB_ENV -Value "XBERG_LIBHEIF_NOTICE_DIR=$heifNoticeDir"
$env:XBERG_LIBHEIF_DLL_DIR = $heifDllDir
$env:XBERG_LIBHEIF_NOTICE_DIR = $heifNoticeDir

# vcpkg-rs still defaults to x64-windows-static-md and refuses a dynamic triplet unless
# VCPKGRS_DYNAMIC is set. The one job that compiles `heic` on Windows exports both on its
# build step (publish.yaml, cli-binaries) instead of here, so every other Windows cargo
# invocation keeps probing the static triplet its crates were built against. ~keep
Write-Host "Set VCPKGRS_TRIPLET=x64-windows and VCPKGRS_DYNAMIC=1 to build the dynamic libheif."

# Neither branch above proves zlib is on disk: the install path prints its "✓" whenever the
# vcpkg call merely does not throw, and the cache path prints one unconditionally. When the port
# is actually missing, nothing notices until `xberg-libwpd`'s build.rs probe finds no archive and
# emits its deliberate loud-failure stem -- surfacing ~45 minutes later, in a different step, as
# `rust-lld: could not open 'zlibstatic.lib'`, which reads like a wrong-library-name bug rather
# than a missing dependency. That is how the Elixir NIF Windows leg failed for v1.1.0 and v1.1.1,
# taking the Hex publish down with it. Check the artifact itself, using the same stem list
# build.rs probes (crates/xberg-libwpd/build.rs ZLIB_RELEASE_STEMS). ~keep
$zlibLibDir = Join-Path $vcpkgRoot "installed\x64-windows-static-md\lib"
$zlibStems = @("zlibstatic", "zlib", "zs", "z")
$zlibFound = $zlibStems | Where-Object { Test-Path (Join-Path $zlibLibDir "$_.lib") } | Select-Object -First 1
if ($zlibFound) {
  Write-Host "✓ zlib archive verified: $(Join-Path $zlibLibDir "$zlibFound.lib")"
}
else {
  Write-Host "::error::no zlib archive ($($zlibStems -join '/')).lib found in $zlibLibDir -- vcpkg reported success but the port is not on disk; xberg-libwpd cannot link"
  exit 1
}

foreach ($header in @('boost/spirit/include/qi.hpp', 'boost/archive/iterators/base64_from_binary.hpp')) {
  $path = Join-Path $vcpkgRoot "installed/x64-windows-static-md/include/$header"
  if (-not (Test-Path -LiteralPath $path)) { throw "Required Boost header is missing: $path" }
}

Write-Host "Installing PHP..."
$phpInstalled = $false
try {
  & php --version 2>$null
  Write-Host "✓ PHP already installed"
  $phpInstalled = $true
}
catch {
  Write-Host "PHP not found, installing via Chocolatey..."
  if (-not (Retry-Command { choco install -y php --no-progress } -MaxAttempts 3)) {
    Write-Host "::warning::Failed to install PHP via Chocolatey, will rely on shivammathur/setup-php action"
  }
  else {
    Write-Host "✓ PHP installed via Chocolatey"
    $phpInstalled = $true
  }
}

Ensure-BuildTool -Package 'cmake' -Executable 'C:\Program Files\CMake\bin\cmake.exe' -Version '4.4.4'

Ensure-Tesseract -Executable 'C:\Program Files\Tesseract-OCR\tesseract.exe' `
  -SourceArchive (Join-Path $env:TEMP 'xberg-tesseract-5.5.4.zip') -VcpkgRoot $vcpkgRoot
Ensure-TesseractData -Directory 'C:\Program Files\Tesseract-OCR\tessdata'

Write-Host "Configuring PATH and environment variables..."
$paths = @(
  "C:\Program Files\CMake\bin",
  "C:\Program Files\Tesseract-OCR",
  "C:\Program Files\LLVM\bin",
  "C:\tools\php",
  "C:\Program Files\PHP",
  # Dynamically linked libheif needs its DLLs at run time for cargo test binaries and the
  # packaged CLI smoke test, both of which run before (or outside) the artifact staging. ~keep
  $heifDllDir
)

foreach ($path in $paths) {
  if (Test-Path $path) {
    Write-Host "  Adding to PATH: $path"
    Write-Output $path | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
    $env:PATH = "$path;$env:PATH"
  }
  else {
    Write-Host "  Path not found (skipping): $path"
  }
}

$tesseractPath = "C:\Program Files\Tesseract-OCR"
if (Test-Path $tesseractPath) {
  $tessdataPath = "$tesseractPath\tessdata"
  if (Test-Path $tessdataPath) {
    Write-Host "  Setting TESSDATA_PREFIX for tests: $tessdataPath"
    Add-Content -Path $env:GITHUB_ENV -Value "TESSDATA_PREFIX=$tessdataPath"
    $env:TESSDATA_PREFIX = $tessdataPath
  }
}

Write-Host "::endgroup::"

Write-Host "::group::Verifying Windows installations"

Write-Host "Tesseract (optional for build):"
try {
  $tesseractCmd = Get-Command tesseract -ErrorAction Stop
  $tesseractPath = $tesseractCmd.Path
  Write-Host "  Found at: $tesseractPath"
  Write-Host "  Command type: $($tesseractCmd.CommandType)"

  $tesseractDir = Split-Path -Parent $tesseractPath
  Write-Host "  Installation directory: $tesseractDir"

  $tessdataPath = Join-Path $tesseractDir "tessdata"
  if (Test-Path $tessdataPath) {
    Write-Host "  tessdata directory: $tessdataPath"
    Write-Host "  Available language files:"
    Get-ChildItem "$tessdataPath\*.traineddata" -ErrorAction SilentlyContinue | ForEach-Object {
      Write-Host "    - $($_.Name)"
    }
  }
  else {
    Write-Host "  tessdata directory not found at: $tessdataPath"
  }

  try {
    $version = & tesseract --version 2>&1
    Write-Host "  Version output: $version"
    Write-Host "✓ Tesseract available and working"

    Write-Host ""
    Write-Host "Available Tesseract languages:"
    & tesseract --list-langs 2>&1 | ForEach-Object { Write-Host "  $_" }
  }
  catch {
    Write-Host "⚠ Warning: Tesseract found but failed to run: $($_.Exception.Message)"
  }

  if (Test-Path $tessdataPath) {
    Write-Host ""
    Write-Host "Setting TESSDATA_PREFIX environment variable..."
    Add-Content -Path $env:GITHUB_ENV -Value "TESSDATA_PREFIX=$tessdataPath"
    Write-Host "✓ Set TESSDATA_PREFIX=$tessdataPath in GITHUB_ENV"
    $env:TESSDATA_PREFIX = $tessdataPath
  }
}
catch {
  Write-Host "⚠ Tesseract not found on PATH (not required for build)"
  Write-Host "  Error details: $($_.Exception.Message)"
  Write-Host "  Searching common installation locations..."

  $commonPaths = @(
    "C:\Program Files\Tesseract-OCR\tesseract.exe",
    "C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
    "${env:ProgramFiles}\Tesseract-OCR\tesseract.exe",
    "${env:ProgramFiles(x86)}\Tesseract-OCR\tesseract.exe"
  )

  $found = $false
  foreach ($path in $commonPaths) {
    if (Test-Path $path) {
      Write-Host "  Found Tesseract at: $path (not on PATH)"
      $tesseractDir = Split-Path -Parent $path
      $tessdataPath = Join-Path $tesseractDir "tessdata"
      if (Test-Path $tessdataPath) {
        Write-Host "  Found tessdata at: $tessdataPath"
        Add-Content -Path $env:GITHUB_ENV -Value "TESSDATA_PREFIX=$tessdataPath"
        Write-Host "✓ Set TESSDATA_PREFIX=$tessdataPath in GITHUB_ENV"
        $env:TESSDATA_PREFIX = $tessdataPath
      }
      $found = $true
      break
    }
  }

  if (-not $found) {
    Write-Host "  Tesseract not found in common locations"
  }
}

Write-Host ""
Write-Host "CMake:"
try {
  & cmake --version
  Write-Host "✓ CMake available"
  $cmakePath = (Get-Command cmake -ErrorAction Stop).Source
  if ($cmakePath) {
    Add-Content -Path $env:GITHUB_ENV -Value "CMAKE=$cmakePath"
    Write-Host "✓ Set CMAKE=$cmakePath in GITHUB_ENV"
  }
}
catch {
  Write-Host "::error::CMake not found after installation"
  throw "CMake verification failed"
}

Write-Host ""
Write-Host "Clang:"
try {
  & clang --version
  Write-Host "✓ Clang available"
}
catch {
  Write-Host "⚠ Warning: Clang not currently available on PATH"
}

Write-Host ""
Write-Host "PHP:"
try {
  & php --version
  Write-Host "✓ PHP available"
}
catch {
  Write-Host "⚠ Warning: PHP not currently available on PATH (will be set up by shivammathur/setup-php action)"
}

Write-Host "::endgroup::"
