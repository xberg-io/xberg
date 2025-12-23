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

# Convert FFI path to MSYS2 format for CGO flags
$msys2FfiPath = ConvertTo-Msys2Path $ffiPath
$msys2IncludePath = "$msys2RepoRoot/crates/kreuzberg-ffi/include"

$cgoEnabled = "1"
$cgoCflags = "-I$msys2IncludePath"
$importLibName = "libkreuzberg_ffi.dll.a"
$importLibPath = Join-Path $ffiPath $importLibName
$linkerVerboseFlags = "-Wl,-v -Wl,--verbose -Wl,--trace -Wl,--print-map"
if (Test-Path $importLibPath) {
  $cgoLdflags = "-L$msys2FfiPath -l:$importLibName -static-libgcc -static-libstdc++ -lws2_32 -luserenv -lbcrypt $linkerVerboseFlags"
} else {
  $cgoLdflags = "-L$msys2FfiPath -lkreuzberg_ffi -static-libgcc -static-libstdc++ -lws2_32 -luserenv -lbcrypt $linkerVerboseFlags"
}

# Add libraries to PATH for runtime discovery
Add-Content -Path $env:GITHUB_ENV -Value "PATH=$env:PATH"
Add-Content -Path $env:GITHUB_ENV -Value "PKG_CONFIG_PATH=$pkgConfigPath"
Add-Content -Path $env:GITHUB_ENV -Value "CGO_ENABLED=$cgoEnabled"
Add-Content -Path $env:GITHUB_ENV -Value "CGO_CFLAGS=$cgoCflags"

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
