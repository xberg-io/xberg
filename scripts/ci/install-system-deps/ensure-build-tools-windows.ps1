function Test-BuildToolVersion {
  param([string]$Executable, [string]$Version, [string[]]$Arguments = @('--version'))
  try {
    $global:LASTEXITCODE = 0
    $output = @(& $Executable @Arguments 2>$null)
    if ($LASTEXITCODE -ne 0 -or $output.Count -eq 0) { return $false }
    return ($output[0] -match ('(?:^|\s)' + [regex]::Escape($Version) + '(?:\s|$)'))
  }
  catch { return $false }
}

function Ensure-BuildTool {
  param([string]$Package, [string]$Executable, [string]$Version)
  if (-not (Test-BuildToolVersion -Executable $Executable -Version $Version)) {
    if (-not (Retry-Command {
      choco upgrade -y $Package --version=$Version --allow-downgrade --no-progress
      if ($LASTEXITCODE -ne 0) { throw "Failed to install $Package $Version with exit $LASTEXITCODE" }
      if (-not (Test-BuildToolVersion -Executable $Executable -Version $Version)) {
        throw "$Executable does not report required version $Version after installation"
      }
    } -MaxAttempts 3)) {
      throw "Failed to install $Package $Version"
    }
  }
  if (-not (Test-BuildToolVersion -Executable $Executable -Version $Version)) {
    throw "$Executable does not report required version $Version"
  }
  Write-Host "Verified $Package $Version from $Executable"
}
