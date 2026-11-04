# Stage verified ORT and release CRT DLLs beside raw native roots.
# ZIP and TAR inputs are replaced only after the repacked PE graph passes.
param(
  [Parameter(Mandatory = $true)][Alias("Artifact")][string]$Archive,
  [Parameter(Mandatory = $true)][string]$DllSourceDir,
  [Parameter(Mandatory = $true)][string]$NativeGlob,
  [string]$MsvcRuntimeDir,
  [string]$PhpHost,
  [string]$PhpVersion
)

$ErrorActionPreference = "Stop"
$stageArgs = @('--artifact', $Archive, '--dll-source-dir', $DllSourceDir, '--native-glob', $NativeGlob)
if ($MsvcRuntimeDir) { $stageArgs += @('--crt-dir', $MsvcRuntimeDir) }
if ($PhpHost) { $stageArgs += @('--php-host', $PhpHost) }
if ($PhpVersion) { $stageArgs += @('--php-version', $PhpVersion) }
& python (Join-Path $PSScriptRoot 'windows_raw_staging.py') @stageArgs
exit $LASTEXITCODE
