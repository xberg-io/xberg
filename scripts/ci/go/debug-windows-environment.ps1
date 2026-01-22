#!/usr/bin/env pwsh
# Comprehensive Windows build environment diagnostics for Go CI
# Helps debug MinGW toolchain setup and native library linking issues
#
# Usage: scripts/ci/go/debug-windows-environment.ps1
# Output: Prints detailed Windows build environment information

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Color functions
function Write-Status {
	param(
		[Parameter(Mandatory=$true)][ValidateSet("OK", "WARN", "FAIL", "INFO")][string]$Status,
		[Parameter(Mandatory=$true)][string]$Message
	)

	$color = switch ($Status) {
		"OK" { "Green" }
		"WARN" { "Yellow" }
		"FAIL" { "Red" }
		"INFO" { "Cyan" }
	}

	$prefix = "[$Status]"
	Write-Host $prefix -ForegroundColor $color -NoNewline
	Write-Host " $Message"
}

function Write-Section {
	param([Parameter(Mandatory=$true)][string]$Title)
	Write-Host ""
	Write-Host "========================================" -ForegroundColor Cyan
	Write-Host $Title -ForegroundColor Cyan
	Write-Host "========================================" -ForegroundColor Cyan
	Write-Host ""
}

# Set repository root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = if ($env:REPO_ROOT) { $env:REPO_ROOT } else { Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $scriptDir)) }

$startTime = Get-Date

# 1. System Information
Write-Section "Windows Build Environment Diagnostics"

Write-Section "System Information"
Write-Status "INFO" "OS: $([System.Environment]::OSVersion.VersionString)"
Write-Status "INFO" "Architecture: $([System.Environment]::Is64BitOperatingSystem ? 'x64' : 'x86')"
Write-Status "INFO" "Processor Count: $([Environment]::ProcessorCount)"
Write-Status "INFO" "Current User: $([System.Security.Principal.WindowsIdentity]::GetCurrent().Name)"
Write-Host ""

# 2. Tool Versions
Write-Section "Tool Versions"
Write-Status "INFO" "Rust:"
try {
	$rustcVersion = (& rustc --version 2>&1)
	Write-Host "  $rustcVersion"
	Write-Status "OK" "rustc available"
} catch {
	Write-Status "WARN" "rustc not found"
}

try {
	$cargoVersion = (& cargo --version 2>&1)
	Write-Host "  $cargoVersion"
	Write-Status "OK" "cargo available"
} catch {
	Write-Status "WARN" "cargo not found"
}

Write-Status "INFO" "Go:"
try {
	$goVersion = (& go version 2>&1)
	Write-Host "  $goVersion"
	Write-Status "OK" "go available"
} catch {
	Write-Status "WARN" "go not found"
}

Write-Status "INFO" "MinGW Toolchain:"
try {
	$gccVersion = (& gcc --version 2>&1 | Select-Object -First 1)
	Write-Host "  $gccVersion"
	Write-Status "OK" "gcc available"
} catch {
	Write-Status "WARN" "gcc not found"
}

try {
	$arVersion = (& ar --version 2>&1 | Select-Object -First 1)
	Write-Host "  $arVersion"
	Write-Status "OK" "ar available"
} catch {
	Write-Status "WARN" "ar not found"
}

try {
	$nasmVersion = (& nasm --version 2>&1 | Select-Object -First 1)
	Write-Host "  $nasmVersion"
	Write-Status "OK" "nasm available"
} catch {
	Write-Status "WARN" "nasm not found"
}
Write-Host ""

# 3. MSYS2 Installation
Write-Section "MSYS2 Installation"
$msys2Path = "C:\msys64"
if (Test-Path $msys2Path) {
	Write-Status "OK" "MSYS2 found at $msys2Path"

	$mingw64Bin = "$msys2Path\mingw64\bin"
	Write-Status "INFO" "MinGW64 bin directory: $mingw64Bin"
	if (Test-Path $mingw64Bin) {
		Write-Status "OK" "MinGW64 bin exists"
		Write-Host "Sample executables in MinGW64 bin:"
		Get-ChildItem -Path $mingw64Bin -Filter "*.exe" | Select-Object -First 15 | ForEach-Object {
			Write-Host "  - $($_.Name)"
		}
	} else {
		Write-Status "FAIL" "MinGW64 bin not found"
	}
} else {
	Write-Status "WARN" "MSYS2 not found at standard location $msys2Path"
}
Write-Host ""

# 4. Environment Variables - Windows Toolchain
Write-Section "Environment Variables - Windows Toolchain"
Write-Status "INFO" "CC: $($env:CC ?? '<not set>')"
Write-Status "INFO" "CXX: $($env:CXX ?? '<not set>')"
Write-Status "INFO" "AR: $($env:AR ?? '<not set>')"
Write-Status "INFO" "RANLIB: $($env:RANLIB ?? '<not set>')"
Write-Status "INFO" "TARGET_CC: $($env:TARGET_CC ?? '<not set>')"
Write-Status "INFO" "TARGET_AR: $($env:TARGET_AR ?? '<not set>')"
Write-Status "INFO" "TARGET_RANLIB: $($env:TARGET_RANLIB ?? '<not set>')"
Write-Status "INFO" "TARGET_CXX: $($env:TARGET_CXX ?? '<not set>')"
Write-Status "INFO" "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER: $($env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER ?? '<not set>')"
Write-Status "INFO" "CC_PREFER_CLANG: $($env:CC_PREFER_CLANG ?? '<not set>')"
Write-Host ""

# 5. Environment Variables - Build
Write-Section "Environment Variables - Build"
Write-Status "INFO" "RUSTFLAGS: $($env:RUSTFLAGS ?? '<not set>')"
Write-Status "INFO" "RUSTC_WRAPPER: $($env:RUSTC_WRAPPER ?? '<not set>')"
Write-Status "INFO" "CARGO_BUILD_TARGET: $($env:CARGO_BUILD_TARGET ?? '<not set>')"
Write-Status "INFO" "ZSTD_DISABLE_LEGACY: $($env:ZSTD_DISABLE_LEGACY ?? '<not set>')"
Write-Status "INFO" "ZSTD_SYS_USE_CMAKE: $($env:ZSTD_SYS_USE_CMAKE ?? '<not set>')"
Write-Host ""

# 6. Environment Variables - CGO
Write-Section "Environment Variables - Go CGO"
Write-Status "INFO" "CGO_ENABLED: $($env:CGO_ENABLED ?? '<not set>')"
Write-Status "INFO" "CGO_CFLAGS: $($env:CGO_CFLAGS ?? '<not set>')"
Write-Status "INFO" "CGO_LDFLAGS: $($env:CGO_LDFLAGS ?? '<not set>')"
Write-Status "INFO" "PKG_CONFIG_PATH: $($env:PKG_CONFIG_PATH ?? '<not set>')"
Write-Status "INFO" "GOFLAGS: $($env:GOFLAGS ?? '<not set>')"
Write-Host ""

# 7. PATH Variable Analysis
Write-Section "PATH Analysis"
Write-Status "INFO" "Full PATH length: $($env:PATH.Length) characters"
Write-Host "First 5 entries in PATH:"
$pathEntries = $env:PATH -split ';'
$pathEntries | Select-Object -First 5 | ForEach-Object { Write-Host "  - $_" }
Write-Host "..."
Write-Host "MSYS2/MinGW64 entries in PATH:"
$mingwEntries = $pathEntries | Where-Object { $_ -like "*msys*" -or $_ -like "*mingw*" }
if ($mingwEntries) {
	$mingwEntries | ForEach-Object { Write-Host "  ✓ $_" }
} else {
	Write-Status "WARN" "No MSYS2/MinGW64 entries found in PATH"
}
Write-Host ""

# 8. Build Directories
Write-Section "Build Directories"
$targetDir = Join-Path $repoRoot "target"
Write-Status "INFO" "Checking target directory: $targetDir"

if (Test-Path "$targetDir\release") {
	Write-Status "OK" "target\release exists"
	Write-Host "FFI libraries in target\release:"
	Get-ChildItem -Path "$targetDir\release" -Filter "libkreuzberg_ffi.*" -ErrorAction SilentlyContinue |
		ForEach-Object { Write-Host "  - $($_.Name) ($([math]::Round($_.Length / 1MB, 2)) MB)" }
} else {
	Write-Status "WARN" "target\release does not exist"
}

$gnuTargetDir = Join-Path $repoRoot "target\x86_64-pc-windows-gnu\release"
if (Test-Path $gnuTargetDir) {
	Write-Status "OK" "target\x86_64-pc-windows-gnu\release exists"
	Write-Host "FFI libraries in x86_64-pc-windows-gnu\release:"
	Get-ChildItem -Path $gnuTargetDir -Filter "libkreuzberg_ffi.*" -ErrorAction SilentlyContinue |
		ForEach-Object { Write-Host "  - $($_.Name) ($([math]::Round($_.Length / 1MB, 2)) MB)" }
} else {
	Write-Status "INFO" "target\x86_64-pc-windows-gnu\release does not exist (expected before MinGW build)"
}
Write-Host ""

# 9. FFI Configuration
Write-Section "FFI Library Configuration"
$ffiDir = Join-Path $repoRoot "crates\kreuzberg-ffi"
Write-Status "INFO" "FFI source directory: $ffiDir"

if (Test-Path $ffiDir) {
	Write-Status "OK" "FFI directory exists"

	$ffiCargoToml = Join-Path $ffiDir "Cargo.toml"
	if (Test-Path $ffiCargoToml) {
		Write-Host "Cargo.toml info:"
		(Get-Content $ffiCargoToml) | Select-String "^name|^version" | ForEach-Object { Write-Host "  $_" }
	}
} else {
	Write-Status "FAIL" "FFI directory not found"
}

$ffiHeader = Join-Path $ffiDir "kreuzberg.h"
if (Test-Path $ffiHeader) {
	Write-Status "OK" "FFI header exists: kreuzberg.h"
	$lineCount = (Get-Content $ffiHeader).Count
	Write-Host "  Size: $lineCount lines"
} else {
	Write-Status "FAIL" "FFI header not found"
}

$pkgConfigFile = Join-Path $ffiDir "kreuzberg-ffi.pc"
if (Test-Path $pkgConfigFile) {
	Write-Status "OK" "pkg-config file exists"
	Write-Host "Contents:"
	Get-Content $pkgConfigFile | ForEach-Object { Write-Host "  $_" }
} else {
	Write-Status "WARN" "pkg-config file not found (will be generated during Rust build)"
}
Write-Host ""

# 10. Go Module Configuration
Write-Section "Go Module Configuration"
$goModDir = Join-Path $repoRoot "packages\go\v4"
$goMod = Join-Path $goModDir "go.mod"

Write-Status "INFO" "Go module location: $goModDir"
if (Test-Path $goMod) {
	Write-Status "OK" "go.mod exists"
	Write-Host "go.mod (first 10 lines):"
	Get-Content $goMod | Select-Object -First 10 | ForEach-Object { Write-Host "  $_" }
} else {
	Write-Status "FAIL" "go.mod not found"
}

if (Test-Path $goModDir) {
	Write-Status "INFO" "Go source files in v4:"
	Get-ChildItem -Path $goModDir -Depth 1 -Filter "*.go" | ForEach-Object { Write-Host "  - $($_.Name)" }
}
Write-Host ""

# 11. Linker Configuration Verification
Write-Section "Linker Configuration Verification"
Write-Status "INFO" "Verifying CGO linker setup..."

if ($env:CGO_LDFLAGS) {
	Write-Status "OK" "CGO_LDFLAGS is set"
	Write-Host "  CGO_LDFLAGS: $($env:CGO_LDFLAGS)"

	# Parse flags
	if ($env:CGO_LDFLAGS -match "-L([^\s]+)") {
		$libPath = $matches[1]
		Write-Status "INFO" "Library search path: $libPath"
		if (Test-Path $libPath) {
			Write-Status "OK" "Library path exists"
		} else {
			Write-Status "FAIL" "Library path does not exist: $libPath"
		}
	}

	if ($env:CGO_LDFLAGS -match "-lkreuzberg_ffi") {
		Write-Status "OK" "kreuzberg_ffi library explicitly linked"
	} else {
		Write-Status "WARN" "kreuzberg_ffi library not in CGO_LDFLAGS"
	}
} else {
	Write-Status "WARN" "CGO_LDFLAGS not set - linker will use defaults"
}
Write-Host ""

# 12. Compiler Verification
Write-Section "Compiler Verification (Critical for MinGW)"
Write-Status "INFO" "Verifying MinGW vs MSVC toolchain separation..."

try {
	$ccPath = (Get-Command gcc -ErrorAction Stop).Source
	Write-Status "OK" "gcc found at: $ccPath"
	if ($ccPath -like "*msys*" -or $ccPath -like "*mingw*") {
		Write-Status "OK" "gcc is from MinGW/MSYS2 (correct)"
	} else {
		Write-Status "FAIL" "gcc is NOT from MinGW/MSYS2: $ccPath"
	}
} catch {
	Write-Status "FAIL" "gcc command not found in PATH"
}

try {
	$arPath = (Get-Command ar -ErrorAction Stop).Source
	Write-Status "OK" "ar found at: $arPath"
	if ($arPath -like "*msys*" -or $arPath -like "*mingw*") {
		Write-Status "OK" "ar is from MinGW/MSYS2 (correct)"
	} else {
		Write-Status "FAIL" "ar is NOT from MinGW/MSYS2: $arPath"
	}
} catch {
	Write-Status "FAIL" "ar command not found in PATH"
}

Write-Host ""

# 13. Dependencies
Write-Section "Dependencies"
Write-Status "INFO" "Checking external dependencies..."

if ($env:KREUZBERG_PDFIUM_PREBUILT -and (Test-Path $env:KREUZBERG_PDFIUM_PREBUILT)) {
	Write-Status "OK" "PDFium found: $($env:KREUZBERG_PDFIUM_PREBUILT)"
} else {
	Write-Status "WARN" "PDFium not configured (KREUZBERG_PDFIUM_PREBUILT not set or invalid)"
}

if ($env:ORT_LIB_LOCATION -and (Test-Path $env:ORT_LIB_LOCATION)) {
	Write-Status "OK" "ONNX Runtime found: $($env:ORT_LIB_LOCATION)"
} else {
	Write-Status "WARN" "ONNX Runtime not configured (ORT_LIB_LOCATION not set or invalid)"
}
Write-Host ""

# 14. Summary
Write-Section "Diagnostics Summary"
$endTime = Get-Date
$duration = $endTime - $startTime
Write-Status "INFO" "Diagnostics started at: $startTime"
Write-Status "INFO" "Diagnostics completed at: $endTime"
Write-Status "INFO" "Total duration: $([math]::Round($duration.TotalSeconds, 2))s"

Write-Host ""
Write-Host "To debug build issues, check:" -ForegroundColor Cyan
Write-Host "  1. MinGW toolchain is in PATH and is the version in MSYS2"
Write-Host "  2. CGO_LDFLAGS points to the correct library directory"
Write-Host "  3. FFI library has been built (libkreuzberg_ffi.* in target/release)"
Write-Host "  4. No MSVC tools (cl.exe, lib.exe) are being used instead of MinGW"
