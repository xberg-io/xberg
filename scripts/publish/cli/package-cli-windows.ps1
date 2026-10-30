$ErrorActionPreference = "Stop"

$target = $env:CLI_TARGET
if ([string]::IsNullOrWhiteSpace($target)) { throw "CLI_TARGET is required" }

$stage = "xberg-cli-$target"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage | Out-Null

$candidates = @(
  ("target/" + $target + "/release/xberg.exe"),
  ("target/release/xberg.exe"),
  ("target/" + $target + "/release/xberg-cli.exe"),
  ("target/release/xberg-cli.exe")
)

$exePath = $null
foreach ($p in $candidates) {
  if (Test-Path $p) { $exePath = $p; break }
}

if (-not $exePath) {
  Write-Host "CLI binary not found. Searched:"
  $candidates | ForEach-Object { Write-Host "  - $_" }
  Write-Host ""
  Write-Host "Directory listing (target):"
  if (Test-Path "target") { Get-ChildItem -Recurse -Depth 3 "target" | Select-Object FullName | Format-Table -AutoSize } else { Write-Host "  (target directory missing)" }
  throw "CLI binary not found for target $target"
}

Copy-Item $exePath $stage
$noticeTool = Join-Path $PSScriptRoot "../../ci/package-notices.py"
& python3 $noticeTool stage $stage
if ($LASTEXITCODE -ne 0) { throw "Canonical notice staging failed" }
Copy-Item "README.md" $stage

if (Test-Path ("target/" + $target + "/release/pdfium.dll")) {
  Copy-Item ("target/" + $target + "/release/pdfium.dll") $stage
}

Compress-Archive -Path "$stage/*" -DestinationPath ($stage + ".zip") -Force
& python3 $noticeTool verify ($stage + ".zip") .
if ($LASTEXITCODE -ne 0) { throw "Canonical archive notice verification failed" }
Remove-Item -Recurse -Force $stage
