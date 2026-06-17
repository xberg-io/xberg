$platform = "win"
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq "ARM64") {
  $archId = "arm64"
} else {
  $archId = "x64"
}
"PDFIUM_PLATFORM=$platform" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
"PDFIUM_ARCH=$archId" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
