# vendor-windows-wheel-closure.ps1
#
# Windows counterpart to scripts/ci/vendor-native-closure.sh: copies the ONNX
# Runtime DLL closure into a Python wheel next to the compiled extension, then
# repacks the wheel. Needed because switching the Windows python-wheels leg
# from the pyke static-link strategy to system dynamic linking (the fix for
# xberg-io/xberg#1456) means onnxruntime.dll is no longer baked into the .pyd
# -- it must be vendored explicitly, the same way auditwheel/delocate do it on
# Linux/macOS, since no `delvewheel` step exists in this pipeline yet.
#
# Usage:
#   vendor-windows-wheel-closure.ps1 <wheel.whl> <DllSourceDir> <NativeGlob>
#
# Appends sha256/size RECORD entries for every DLL it adds, per the wheel spec
# (https://packaging.python.org/en/latest/specifications/binary-distribution-format/),
# so `pip install` does not see an internally-inconsistent RECORD.

param(
  [Parameter(Mandatory = $true)][string]$Wheel,
  [Parameter(Mandatory = $true)][string]$DllSourceDir,
  [Parameter(Mandatory = $true)][string]$NativeGlob,
  [string]$MsvcRuntimeDir
)

$ErrorActionPreference = "Stop"

function Write-Log([string]$Message) {
  Write-Host "vendor-windows-wheel-closure: $Message"
}

if (-not (Test-Path $Wheel)) { throw "wheel not found: '$Wheel'" }
if (-not (Test-Path $DllSourceDir)) { throw "DLL source dir not found: '$DllSourceDir'" }

$wheelPath = (Resolve-Path $Wheel).Path
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vendor-wheel-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $workDir -Force | Out-Null

try {
  Expand-Archive -Path $wheelPath -DestinationPath $workDir -Force

  $natives = @(Get-ChildItem -Path $workDir -Recurse -File -Filter $NativeGlob -ErrorAction SilentlyContinue)
  if ($natives.Count -eq 0) {
    throw "no native file matching '$NativeGlob' found in wheel '$wheelPath'"
  }
  $targetDirs = $natives | ForEach-Object { $_.Directory.FullName } | Select-Object -Unique

  $recordFile = @(Get-ChildItem -Path $workDir -Recurse -File -Filter "RECORD" -ErrorAction SilentlyContinue) |
    Where-Object { $_.DirectoryName -like "*.dist-info" } | Select-Object -First 1
  if (-not $recordFile) { throw "no *.dist-info/RECORD found in wheel '$wheelPath'" }

  $addedRecordLines = @()
  foreach ($dir in $targetDirs) {
    $dlls = @(Get-ChildItem -Path $DllSourceDir -Filter "*.dll" -File -ErrorAction SilentlyContinue)
    if ($dlls.Count -eq 0) { throw "no *.dll files found under DLL source dir '$DllSourceDir'" }
    foreach ($dll in $dlls) {
      $destPath = Join-Path $dir $dll.Name
      if (Test-Path $destPath) {
        Write-Log "skipping $($dll.Name), already present at $destPath"
        continue
      }
      Copy-Item -Path $dll.FullName -Destination $destPath -Force
      Write-Log "vendored $($dll.Name) into $dir"

      $hashBytes = (Get-FileHash -Path $destPath -Algorithm SHA256).Hash
      $rawBytes = [byte[]]::new($hashBytes.Length / 2)
      for ($i = 0; $i -lt $rawBytes.Length; $i++) {
        $rawBytes[$i] = [Convert]::ToByte($hashBytes.Substring($i * 2, 2), 16)
      }
      $b64 = [Convert]::ToBase64String($rawBytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
      $size = (Get-Item $destPath).Length
      $relPath = [System.IO.Path]::GetRelativePath($workDir, $destPath) -replace '\\', '/'
      $addedRecordLines += "$relPath,sha256=$b64,$size"
    }
  }

  $stageArgs = @('stage', '--artifact', $workDir, '--native-glob', $NativeGlob)
  if ($MsvcRuntimeDir) { $stageArgs += @('--crt-dir', $MsvcRuntimeDir) }
  $runtime = & python (Join-Path $PSScriptRoot 'windows_wheel_closure.py') @stageArgs
  if ($LASTEXITCODE -ne 0) { throw 'Microsoft runtime closure staging failed' }
  $runtimeFiles = ($runtime | ConvertFrom-Json).added
  foreach ($path in $runtimeFiles) {
    $rawBytes = [Convert]::FromHexString((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash)
    $b64 = [Convert]::ToBase64String($rawBytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
    $relPath = [System.IO.Path]::GetRelativePath($workDir, $path) -replace '\\', '/'
    $addedRecordLines += "$relPath,sha256=$b64,$((Get-Item -LiteralPath $path).Length)"
  }

  if ($addedRecordLines.Count -gt 0) {
    Add-Content -Path $recordFile.FullName -Value $addedRecordLines
    Write-Log "appended $($addedRecordLines.Count) RECORD entries"
  }

  $repackedZip = Join-Path ([System.IO.Path]::GetTempPath()) ("vendor-wheel-out-" + [Guid]::NewGuid().ToString("N") + ".zip")
  if (Test-Path $repackedZip) { Remove-Item $repackedZip -Force }
  # Compress-Archive from the directory contents (not the directory itself)
  # preserves the wheel's flat top-level layout (<pkg>/, *.dist-info/).
  Compress-Archive -Path (Join-Path $workDir '*') -DestinationPath $repackedZip -Force
  Move-Item -Path $repackedZip -Destination $wheelPath -Force
  Write-Log "repacked $wheelPath"
}
finally {
  if (Test-Path $workDir) { Remove-Item -Recurse -Force $workDir }
}

# This script only uses PowerShell cmdlets, which do not update the automatic
# $LASTEXITCODE variable. Set the process contract explicitly so callers do not
# mistake a stale native-command exit code for a vendoring failure.
exit 0
