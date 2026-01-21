$ErrorActionPreference = "Stop"

# Convert Windows path to MSYS2 format (C:\foo\bar -> /c/foo/bar)
function ConvertTo-Msys2Path {
  param([string]$WindowsPath)

  # Normalize path separators
  $normalized = $WindowsPath -replace '\\', '/'

  # Convert drive letter (C: -> /c)
  if ($normalized -match '^([A-Za-z]):(.*)$') {
    $drive = $matches[1].ToLower()
    $path = $matches[2]
    return "/$drive$path"
  }

  return $normalized
}

$ffiLibDir = $args[0]
if ([string]::IsNullOrWhiteSpace($ffiLibDir)) { $ffiLibDir = "target/release" }

$repoRoot = $env:GITHUB_WORKSPACE
$ffiPath = Join-Path $repoRoot $ffiLibDir

$gnuTargetPath = Join-Path $repoRoot "target/x86_64-pc-windows-gnu/release"
if (Test-Path $gnuTargetPath) {
  $ffiPath = $gnuTargetPath
  Write-Host "Using Windows GNU target path: $ffiPath"
} elseif (-not (Test-Path $ffiPath)) {
  throw "Error: FFI library directory not found: $ffiPath"
}

# Convert paths to MSYS2 format for pkg-config compatibility
$msys2RepoRoot = ConvertTo-Msys2Path $repoRoot
$pkgConfigDir = "$msys2RepoRoot/crates/kreuzberg-ffi"

# Use colon separator for MSYS2 (not semicolon)
if ([string]::IsNullOrWhiteSpace($env:PKG_CONFIG_PATH)) {
  $pkgConfigPath = $pkgConfigDir
} else {
  # If PKG_CONFIG_PATH already exists, preserve it with colon separator
  $pkgConfigPath = "${pkgConfigDir}:$($env:PKG_CONFIG_PATH)"
}

$env:PATH = "${ffiPath};$($env:PATH)"

# Persist FFI path to GITHUB_PATH for subsequent steps
if (Test-Path $ffiPath) {
  Add-Content -Path $env:GITHUB_PATH -Value $ffiPath -Encoding utf8
}

# Convert FFI path to MSYS2 format for CGO flags
$msys2FfiPath = ConvertTo-Msys2Path $ffiPath
# Note: Windows Go builds use internal/ffi for the header via CGO directives
# We set the include path but it's primarily used for verification
$msys2IncludePath = "$msys2RepoRoot/packages/go/v4/internal/ffi"

# Verify FFI header is accessible to Go
$headerPath = Join-Path $repoRoot "packages/go/v4/internal/ffi\kreuzberg.h"
if (-not (Test-Path $headerPath)) {
  Write-Host "⚠ Warning: FFI header not found at $headerPath"
  Write-Host "  This may cause compilation failures if header is not available"
  Write-Host "  Expected: packages/go/v4/internal/ffi/kreuzberg.h"
} else {
  Write-Host "✓ FFI header verified at packages/go/v4/internal/ffi/kreuzberg.h"
}

$mingwBin = "C:\msys64\mingw64\bin"
if (Test-Path (Join-Path $mingwBin "x86_64-w64-mingw32-gcc.exe")) {
  $env:PATH = "${mingwBin};$($env:PATH)"
  # Persist MinGW bin to GITHUB_PATH for subsequent steps
  Add-Content -Path $env:GITHUB_PATH -Value $mingwBin -Encoding utf8
  $env:CC = "x86_64-w64-mingw32-gcc"
  $env:CXX = "x86_64-w64-mingw32-g++"
  $env:AR = "x86_64-w64-mingw32-ar"
  $env:RANLIB = "x86_64-w64-mingw32-ranlib"
  Write-Host "Using MinGW64 toolchain for Go cgo: $mingwBin"
}

$cgoEnabled = "1"
# Note: We don't set CGO_CFLAGS here because ffi.go has #cgo windows CFLAGS directive
# But we keep this for potential use by other tools
$cgoCflags = "-I$msys2IncludePath"
$importLibName = "libkreuzberg_ffi.dll.a"
$importLibPath = Join-Path $ffiPath $importLibName
# FIXME: Verbose linker flags (-Wl,-v) cause "invalid flag in go:cgo_ldflag" errors on Windows
# These flags are incompatible with Windows Go CGO compilation
# See: https://github.com/kreuzberg-dev/kreuzberg/pull/316
$linkerVerboseFlags = ""
# Temporarily disabled due to Go Windows CGO incompatibility
# if ($env:KREUZBERG_GO_LINKER_VERBOSE -eq "1") {
#   $linkerVerboseFlags = "-Wl,-v -Wl,--verbose"
# }
# Only set the library search path (-L) here. The ffi.go CGO directives
# already specify -lkreuzberg_ffi and Windows system libraries (-lws2_32, etc)
# Environment variable flags are prepended to CGO directive flags, so we just need -L
$cgoLdflags = "-L$msys2FfiPath $linkerVerboseFlags".Trim()

# Add libraries to PATH for runtime discovery
# Note: PATH modifications are now handled via GITHUB_PATH above
Add-Content -Path $env:GITHUB_ENV -Value "PKG_CONFIG_PATH=$pkgConfigPath"
Add-Content -Path $env:GITHUB_ENV -Value "CGO_ENABLED=$cgoEnabled"
Add-Content -Path $env:GITHUB_ENV -Value "CGO_CFLAGS=$cgoCflags"
if ($env:CC) { Add-Content -Path $env:GITHUB_ENV -Value "CC=$env:CC" }
if ($env:CXX) { Add-Content -Path $env:GITHUB_ENV -Value "CXX=$env:CXX" }
if ($env:AR) { Add-Content -Path $env:GITHUB_ENV -Value "AR=$env:AR" }
if ($env:RANLIB) { Add-Content -Path $env:GITHUB_ENV -Value "RANLIB=$env:RANLIB" }

# CRITICAL: Replace CGO_LDFLAGS entirely, never append
# This prevents duplication if the script is called multiple times
# or if other scripts have already set CGO_LDFLAGS
Write-Host "Setting CGO_LDFLAGS (replacing any existing value)"
@"
CGO_LDFLAGS=$cgoLdflags
"@ | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding UTF8

Write-Host "✓ Go cgo environment configured (Windows)"
Write-Host "  FFI Library Path (Windows): $ffiPath"
Write-Host "  FFI Library Path (MSYS2): $msys2FfiPath"
Write-Host "  PKG_CONFIG_PATH: $pkgConfigPath"
Write-Host "  CGO_CFLAGS: $cgoCflags"
Write-Host "  CGO_LDFLAGS: $cgoLdflags"
