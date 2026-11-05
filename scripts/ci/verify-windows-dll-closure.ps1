# Audit hard and delay-loaded PE imports in the exact artifact that will ship.
# This static check does not run the Windows loader.
param(
  [Parameter(Mandatory = $true)][string]$Artifact,
  [Parameter(Mandatory = $true)][string]$NativeGlob,
  [string]$RequiredDll = "onnxruntime.dll",
  [string]$ForbiddenDll = "DirectML.dll",
  [ValidateSet("wheel", "raw")][string]$ArtifactKind,
  [string]$PhpHost,
  [switch]$RequireCrtNotices,
  [string]$PhpVersion
)

$ErrorActionPreference = "Stop"
if (-not $ArtifactKind) {
  $ArtifactKind = if ($NativeGlob -like '*.pyd') { "wheel" } else { "raw" }
}
$auditArgs = @(
  'audit', '--artifact', $Artifact, '--native-glob', $NativeGlob, '--artifact-kind', $ArtifactKind,
  '--required', $RequiredDll, '--forbidden', $ForbiddenDll
)
if ($PhpHost) { $auditArgs += @('--php-host', $PhpHost) }
if ($PhpVersion) { $auditArgs += @('--php-version', $PhpVersion) }
if ($RequireCrtNotices) { $auditArgs += "--require-crt-notices" }
& python (Join-Path $PSScriptRoot 'windows_wheel_closure.py') @auditArgs
exit $LASTEXITCODE
