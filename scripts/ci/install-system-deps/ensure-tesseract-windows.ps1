Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-TesseractVersion {
  param([string]$Executable)
  if (-not (Test-Path -LiteralPath $Executable)) { return $null }
  $output = @(& $Executable --version 2>&1)
  if ($LASTEXITCODE -ne 0) { throw "Tesseract version probe failed: $LASTEXITCODE" }
  if ($output.Count -eq 0 -or $output[0] -notmatch '^tesseract\s+v?(\d+\.\d+\.\d+)') {
    throw 'Tesseract returned no recognized version'
  }
  return [version]$Matches[1]
}

function Assert-TesseractRuntime {
  param([string]$Executable)
  $output = @(& $Executable --version 2>&1)
  if ($LASTEXITCODE -ne 0) { throw "Tesseract runtime probe failed: $LASTEXITCODE" }
  if ($output.Count -eq 0 -or $output[0] -notmatch '^tesseract\s+v?5\.5\.4(?:\D|$)' -or
      @($output | Where-Object { $_ -match '^\s*leptonica-1\.87\.0(?:\D|$)' }).Count -ne 1) {
    throw 'Expected actual Tesseract 5.5.4 and Leptonica 1.87.0 runtimes'
  }
}

function Invoke-TesseractBuildCommand {
  param([string]$Executable, [string[]]$Arguments)
  $previous = [Environment]::GetEnvironmentVariable('_CL_', 'Process')
  try {
    # Upstream adds /MP; the trailing MSVC environment flags bound its compiler processes too. ~keep
    $env:_CL_ = ($previous + ' /MP1').Trim()
    $global:LASTEXITCODE = 0
    & $Executable @Arguments | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Tesseract source command failed with exit ${LASTEXITCODE}: $Executable" }
  }
  finally { $env:_CL_ = $previous }
}

function Install-TesseractLeptonica {
  param([string]$VcpkgRoot)
  $previous = [Environment]::GetEnvironmentVariable('VCPKG_MAX_CONCURRENCY', 'Process')
  try {
    $env:VCPKG_MAX_CONCURRENCY = '1'
    Invoke-TesseractBuildCommand -Executable (Join-Path $VcpkgRoot 'vcpkg.exe') `
      -Arguments @('install', 'leptonica:x64-windows-static-md', '--recurse')
  }
  finally { $env:VCPKG_MAX_CONCURRENCY = $previous }
}

function Build-TesseractSource {
  param([string]$SourceArchive, [string]$VcpkgRoot, [string]$InstallRoot)
  $work = Join-Path ([IO.Path]::GetTempPath()) ('xberg-tesseract-' + [guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $work | Out-Null
  try {
    Expand-Archive -LiteralPath $SourceArchive -DestinationPath $work
    $source = Join-Path $work 'tesseract-7e548e6eff172f38f97664afe7f7ce40db36c8d6'
    if (-not (Test-Path -LiteralPath (Join-Path $source 'CMakeLists.txt'))) { throw 'Tesseract source archive extracted no expected source' }
    Install-TesseractLeptonica -VcpkgRoot $VcpkgRoot
    $build = Join-Path $work 'build'
    $cmake = 'C:\Program Files\CMake\bin\cmake.exe'
    Invoke-TesseractBuildCommand -Executable $cmake -Arguments @(
      '-S', $source, '-B', $build, '-A', 'x64',
      "-DCMAKE_TOOLCHAIN_FILE=$VcpkgRoot/scripts/buildsystems/vcpkg.cmake",
      '-DVCPKG_TARGET_TRIPLET=x64-windows-static-md', '-DVCPKG_MANIFEST_MODE=OFF',
      "-DCMAKE_INSTALL_PREFIX=$InstallRoot", '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL',
      '-DBUILD_SHARED_LIBS=ON', '-DBUILD_TRAINING_TOOLS=OFF', '-DBUILD_TESTS=OFF',
      '-DOPENMP_BUILD=OFF', '-DDISABLE_CURL=ON', '-DDISABLE_ARCHIVE=ON'
    )
    Invoke-TesseractBuildCommand -Executable $cmake `
      -Arguments @('--build', $build, '--config', 'Release', '--target', 'install', '--parallel', '1')
    # Upstream CMake installs into bin; retain the established CLI location used by consumers. ~keep
    Get-ChildItem -LiteralPath (Join-Path $InstallRoot 'bin') -File | Copy-Item -Destination $InstallRoot
    $notices = Join-Path $InstallRoot 'share/licenses/xberg-system-ocr/tesseract-5.5.4'
    New-Item -ItemType Directory -Path $notices -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $source 'LICENSE'), (Join-Path $source 'AUTHORS') -Destination $notices
    $leptNotices = Join-Path $InstallRoot 'share/licenses/xberg-system-ocr/leptonica-1.87.0'
    New-Item -ItemType Directory -Path $leptNotices -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $VcpkgRoot 'installed/x64-windows-static-md/share/leptonica/copyright') `
      -Destination (Join-Path $leptNotices 'leptonica-license.txt')
  }
  finally { Remove-Item -LiteralPath $work -Recurse -Force }
}

function Ensure-Tesseract {
  param([string]$Executable, [string]$SourceArchive, [string]$VcpkgRoot)
  $required = [version]'5.5.4'
  $current = Get-TesseractVersion -Executable $Executable
  if ($null -ne $current -and $current -ge $required) {
    Assert-TesseractRuntime -Executable $Executable
    Write-Host "Verified Tesseract $current"
    return
  }
  # 5.5.4 has no published Windows installer; build its signed-tag source instead. ~keep
  $url = 'https://codeload.github.com/tesseract-ocr/tesseract/zip/7e548e6eff172f38f97664afe7f7ce40db36c8d6'
  $expected = 'ef082fa5b1ddf469e8e5174298242afdc7605b278470693badf2dbf50e17721b'
  Invoke-WebRequest -Uri $url -OutFile $SourceArchive -ErrorAction Stop
  if ((Get-Item -LiteralPath $SourceArchive).Length -ne 2603004 -or
      (Get-FileHash -LiteralPath $SourceArchive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
    throw 'Tesseract source archive size or checksum mismatch'
  }
  Build-TesseractSource -SourceArchive $SourceArchive -VcpkgRoot $VcpkgRoot -InstallRoot (Split-Path -Parent $Executable)
  $current = Get-TesseractVersion -Executable $Executable
  if ($null -eq $current -or $current -lt $required) {
    throw "Tesseract version $current is below required $required after source installation"
  }
  Assert-TesseractRuntime -Executable $Executable
  Write-Host "Verified Tesseract $current"
}

function Ensure-TesseractData {
  param([string]$Directory)
  New-Item -ItemType Directory -Path $Directory -Force | Out-Null
  $models = @(
    @{ Name = 'eng'; Size = 4113088; Hash = '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2' }
    @{ Name = 'osd'; Size = 10562727; Hash = '9cf5d576fcc47564f11265841e5ca839001e7e6f38ff7f7aacf46d15a96b00ff' }
    # The Han second pass test (gh2300_ocr_han_second_pass) reads Chinese and needs this model. ~keep
    @{ Name = 'chi_sim'; Size = 2469156; Hash = 'a5fcb6f0db1e1d6d8522f39db4e848f05984669172e584e8d76b6b3141e1f730' }
  )
  foreach ($model in $models) {
    $path = Join-Path $Directory ($model.Name + '.traineddata')
    if (-not (Test-Path -LiteralPath $path) -or (Get-Item -LiteralPath $path).Length -ne $model.Size -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $model.Hash) {
      $url = 'https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/87416418657359cb625c412a48b6e1d6d41c29bd/' + $model.Name + '.traineddata'
      Invoke-WebRequest -Uri $url -OutFile $path -ErrorAction Stop
    }
    if ((Get-Item -LiteralPath $path).Length -ne $model.Size -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $model.Hash) {
      throw "Tesseract model $($model.Name) size or checksum mismatch"
    }
  }
}
