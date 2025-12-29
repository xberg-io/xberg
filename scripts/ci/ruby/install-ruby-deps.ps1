#!/usr/bin/env pwsh
# Install Ruby dependencies via bundle (Windows)
# Used by: ci-ruby.yaml - Install Ruby deps step (Windows)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Windows Ruby Dependencies Installation (Verbose) ===" -ForegroundColor Cyan

# Print environment info
Write-Host ""
Write-Host "=== Ruby Environment ===" -ForegroundColor Yellow
ruby --version
Write-Host "Ruby platform: $(ruby -e 'puts RUBY_PLATFORM')"
Write-Host "Gem home: $(gem env home)"

Write-Host ""
Write-Host "=== Rust Toolchain ===" -ForegroundColor Yellow
rustc --version
cargo --version
rustc -vV | Select-String "host"

Write-Host ""
Write-Host "=== System Info ===" -ForegroundColor Yellow
$osInfo = Get-ComputerInfo
Write-Host "OS: $($osInfo.OsName)"
$osArch = (Get-CimInstance Win32_OperatingSystem).OSArchitecture
Write-Host "Architecture: $osArch"
Write-Host "Compiler: x86_64-pc-windows-gnu"

Write-Host ""
Write-Host "=== Environment Variables ===" -ForegroundColor Yellow
Write-Host "RUST_BACKTRACE: $($env:RUST_BACKTRACE)"
Write-Host "CARGO_BUILD_JOBS: $($env:CARGO_BUILD_JOBS)"
Write-Host "RB_SYS_VERBOSE: $($env:RB_SYS_VERBOSE)"

Write-Host ""
Write-Host "=== Bundler Configuration ===" -ForegroundColor Yellow
Write-Host "Setting bundle config for Windows..."
bundle config set deployment false
bundle config set path vendor/bundle

if ($env:GITHUB_ENV) {
  if (-not $env:BUNDLE_GEMFILE) {
    Add-Content -Path $env:GITHUB_ENV -Value "BUNDLE_GEMFILE=$PSScriptRoot\..\..\packages\ruby\Gemfile"
  }
  if (-not $env:BUNDLE_PATH) {
    Add-Content -Path $env:GITHUB_ENV -Value "BUNDLE_PATH=$PSScriptRoot\..\..\packages\ruby\vendor\bundle"
  }
}

Write-Host ""
Write-Host "=== Bundle Configuration (current) ===" -ForegroundColor Yellow
bundle config list

Write-Host ""
Write-Host "=== Installing gems (verbose mode) ===" -ForegroundColor Yellow
bundle install --jobs 4 --verbose

Write-Host ""
Write-Host "=== Installed gems ===" -ForegroundColor Yellow
gem list

Write-Host ""
Write-Host "Ruby dependencies installed successfully" -ForegroundColor Green
