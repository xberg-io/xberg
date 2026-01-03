#!/usr/bin/env pwsh
# Configure bindgen compatibility headers for Windows
# Used by: ci-ruby.yaml - Configure bindgen compatibility headers step

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Configuring bindgen compatibility headers for Windows ===" -ForegroundColor Cyan
Write-Host ""

# Check environment
Write-Host "Environment check:" -ForegroundColor Yellow
Write-Host "  GITHUB_WORKSPACE: $env:GITHUB_WORKSPACE"
Write-Host "  MSYSTEM: $($env:MSYSTEM ?? 'not set')"
Write-Host "  MSYSTEM_PREFIX: $($env:MSYSTEM_PREFIX ?? 'not set')"
Write-Host ""

$includeRoot = "$env:GITHUB_WORKSPACE\packages\ruby\ext\kreuzberg_rb\native\include"
$compat = "$includeRoot\msvc_compat"

# Verify directories exist
Write-Host "Verifying include directories:" -ForegroundColor Yellow
if (Test-Path $includeRoot) {
    Write-Host "  [OK] Include root exists: $includeRoot" -ForegroundColor Green
} else {
    Write-Host "  [WARN] Include root missing: $includeRoot" -ForegroundColor Red
}

if (Test-Path $compat) {
    Write-Host "  [OK] MSVC compat directory exists: $compat" -ForegroundColor Green
} else {
    Write-Host "  [WARN] MSVC compat directory missing: $compat" -ForegroundColor Red
}
Write-Host ""

$includeRoot = $includeRoot -replace '\\','/'
$compatForward = $compat -replace '\\','/'

# Detect MinGW GCC include paths for C standard library headers
Write-Host "Detecting MinGW GCC include paths:" -ForegroundColor Yellow
$gccIncludePaths = @()

try {
    # Get GCC's built-in include search paths
    $gccOutput = & gcc -v -E -x c - 2>&1 | Out-String

    # Extract include paths from the output
    $inIncludeSection = $false
    foreach ($line in $gccOutput -split "`n") {
        if ($line -match '#include <\.\.\.> search starts here:') {
            $inIncludeSection = $true
            continue
        }
        if ($line -match 'End of search list') {
            break
        }
        if ($inIncludeSection -and $line.Trim() -ne '') {
            $path = $line.Trim()
            # Convert Windows paths to forward slashes
            $path = $path -replace '\\','/'
            $gccIncludePaths += $path
            Write-Host "  Found: $path" -ForegroundColor Green
        }
    }

    if ($gccIncludePaths.Count -eq 0) {
        Write-Host "  [WARN] No GCC include paths detected via -v flag" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  [WARN] Failed to detect GCC include paths: $_" -ForegroundColor Yellow
}

# Also try to get GCC's resource directory for compiler-specific headers (stdarg.h, stddef.h, etc.)
try {
    $gccResourceDir = & gcc -print-file-name=include 2>&1
    if ($gccResourceDir -and (Test-Path $gccResourceDir)) {
        $gccResourceDir = $gccResourceDir -replace '\\','/'
        if ($gccResourceDir -notin $gccIncludePaths) {
            $gccIncludePaths += $gccResourceDir
            Write-Host "  GCC resource dir: $gccResourceDir" -ForegroundColor Green
        }
    }
} catch {
    Write-Host "  [WARN] Failed to get GCC resource directory: $_" -ForegroundColor Yellow
}

Write-Host ""

# Build the extra clang args with all necessary paths and flags
$extra = "-I$includeRoot -I$compatForward -fms-extensions -fstack-protector-strong -fno-omit-frame-pointer -fno-fast-math"

# Add all detected GCC include paths
foreach ($path in $gccIncludePaths) {
    $extra += " -isystem$path"
}

# Check for MSYS2/MinGW sysroot
if ($env:MSYSTEM_PREFIX) {
    $sysroot = "$env:MSYSTEM_PREFIX" -replace '\\','/'
    $extra += " --target=x86_64-pc-windows-gnu --sysroot=$sysroot"
    Write-Host "MSYS2 detected:" -ForegroundColor Yellow
    Write-Host "  Sysroot: $sysroot" -ForegroundColor Green
    Write-Host "  Target: x86_64-pc-windows-gnu" -ForegroundColor Green
} else {
    Write-Host "MSYS2 not detected (MSYSTEM_PREFIX not set)" -ForegroundColor Yellow
}
Write-Host ""

# Check for clang
Write-Host "Checking for clang:" -ForegroundColor Yellow
$clangPath = Get-Command clang -ErrorAction SilentlyContinue
if ($clangPath) {
    Write-Host "  [OK] clang found: $($clangPath.Source)" -ForegroundColor Green
    try {
        $clangVersion = & clang --version 2>&1 | Select-Object -First 1
        Write-Host "  Version: $clangVersion" -ForegroundColor Green
    } catch {
        Write-Host "  [WARN] Could not get clang version" -ForegroundColor Yellow
    }
} else {
    Write-Host "  [WARN] clang not found in PATH" -ForegroundColor Red
    Write-Host "  PATH entries:" -ForegroundColor Yellow
    $env:PATH -split ';' | Select-Object -First 10 | ForEach-Object { Write-Host "    $_" }
}
Write-Host ""

# Set for all possible target formats (bindgen uses different naming conventions)
Write-Host "Setting BINDGEN_EXTRA_CLANG_ARGS environment variables:" -ForegroundColor Yellow
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64-pc-windows-msvc=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_msvc=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64-pc-windows-gnu=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_gnu=$extra"

Write-Host "  BINDGEN_EXTRA_CLANG_ARGS = $extra" -ForegroundColor Green
Write-Host ""
Write-Host "Configuration complete" -ForegroundColor Green
