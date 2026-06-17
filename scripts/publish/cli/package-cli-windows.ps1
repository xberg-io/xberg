$ErrorActionPreference = "Stop"

$target = $env:CLI_TARGET
if ([string]::IsNullOrWhiteSpace($target)) { throw "CLI_TARGET is required" }

$stage = "kreuzberg-cli-$target"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage | Out-Null

$candidates = @(
  ("target/" + $target + "/release/kreuzberg.exe"),
  ("target/release/kreuzberg.exe"),
  ("target/" + $target + "/release/kreuzberg-cli.exe"),
  ("target/release/kreuzberg-cli.exe")
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
Copy-Item "LICENSE" $stage
Copy-Item "README.md" $stage

if (Test-Path ("target/" + $target + "/release/pdfium.dll")) {
  Copy-Item ("target/" + $target + "/release/pdfium.dll") $stage
}

Compress-Archive -Path "$stage/*" -DestinationPath ($stage + ".zip") -Force
Remove-Item -Recurse -Force $stage
