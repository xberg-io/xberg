$ErrorActionPreference = "Stop"

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
      & $Command
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

$tesseractCacheHit = $env:TESSERACT_CACHE_HIT -eq "true"
$libreofficeInstalled = Test-Path "C:\Program Files\LibreOffice\program\soffice.exe"

if (-not $tesseractCacheHit) {
  Write-Host "Tesseract cache miss, installing..."
  if (-not (Retry-Command { choco install -y tesseract --no-progress } -MaxAttempts 3)) {
    throw "Failed to install Tesseract after 3 attempts"
  }
  Write-Host "✓ Tesseract installed"
}
else {
  Write-Host "✓ Tesseract found in cache"
}

if (-not $libreofficeInstalled) {
  Write-Host "LibreOffice not found, installing (timeout: 20min)..."

  $job = Start-Job -ScriptBlock {
    choco install -y libreoffice --no-progress
  }

  $completed = $job | Wait-Job -Timeout 1200

  if (-not $completed) {
    $job | Stop-Job -Force
    throw "LibreOffice installation timed out after 20 minutes"
  }

  $result = $job | Receive-Job
  $exitCode = $job.JobStateInfo.State

  if ($exitCode -ne "Completed") {
    Write-Host "LibreOffice installation failed"
    Write-Host "Output: $result"
    throw "LibreOffice installation failed"
  }

  Write-Host "✓ LibreOffice installed"
}
else {
  Write-Host "✓ LibreOffice already installed"
}

Write-Host "Configuring PATH..."
$paths = @(
  "C:\Program Files\LibreOffice\program",
  "C:\Program Files\Tesseract-OCR"
)

foreach ($path in $paths) {
  if (Test-Path $path) {
    Write-Output $path | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
    $env:PATH = "$path;$env:PATH"
  }
}

Write-Host "::endgroup::"

Write-Host "::group::Verifying Windows installations"

Write-Host "LibreOffice:"
try {
  & soffice --version 2>$null
  Write-Host "✓ LibreOffice available"
}
catch {
  Write-Host "⚠ Warning: LibreOffice verification failed"
}

Write-Host ""
Write-Host "Tesseract:"
& tesseract --version

Write-Host ""
Write-Host "Available Tesseract languages:"
& tesseract --list-langs

Write-Host ""
Write-Host "Tesseract installation location:"
$tesseractPath = (Get-Command tesseract -ErrorAction SilentlyContinue).Path
if ($tesseractPath) {
  Write-Host "  $tesseractPath"
}
else {
  Write-Host "  ⚠ Could not determine tesseract path"
}

Write-Host "::endgroup::"
