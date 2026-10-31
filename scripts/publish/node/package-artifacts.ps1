$ErrorActionPreference = "Stop"

if (-Not (Test-Path "crates/xberg-node/npm")) { throw "npm artifact directory missing" }

$target = $env:TARGET
if ([string]::IsNullOrWhiteSpace($target)) { throw "TARGET not set" }

switch ($target) {
	"aarch64-apple-darwin" { $platformDir = "darwin-arm64"; $nodeFile = "xberg-node.darwin-arm64.node"; break }
	"x86_64-apple-darwin" { $platformDir = "darwin-x64"; $nodeFile = "xberg-node.darwin-x64.node"; break }
	"x86_64-pc-windows-msvc" { $platformDir = "win32-x64-msvc"; $nodeFile = "xberg-node.win32-x64-msvc.node"; break }
	"aarch64-pc-windows-msvc" { $platformDir = "win32-arm64-msvc"; $nodeFile = "xberg-node.win32-arm64-msvc.node"; break }
	"x86_64-unknown-linux-gnu" { $platformDir = "linux-x64-gnu"; $nodeFile = "xberg-node.linux-x64-gnu.node"; break }
	"aarch64-unknown-linux-gnu" { $platformDir = "linux-arm64-gnu"; $nodeFile = "xberg-node.linux-arm64-gnu.node"; break }
	"armv7-unknown-linux-gnueabihf" { $platformDir = "linux-arm-gnueabihf"; $nodeFile = "xberg-node.linux-arm-gnueabihf.node"; break }
	default { throw ("Unsupported NAPI target: " + $target) }
}

$destDir = Join-Path "crates/xberg-node/npm" $platformDir
$dest = Join-Path $destDir $nodeFile

$srcCandidates = @(
	(Join-Path "crates/xberg-node/artifacts" $nodeFile),
	(Join-Path "crates/xberg-node" $nodeFile)
)

$src = $null
foreach ($candidate in $srcCandidates) {
	if (Test-Path $candidate) {
		$src = $candidate
		break
	}
}

if ($null -eq $src) {
	Write-Host ("Missing built NAPI binary: expected " + $nodeFile + " under crates/xberg-node/artifacts or crate root")
	Get-ChildItem -Path "crates/xberg-node" -Recurse -Depth 2 -Filter "*.node" -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName } | Out-Host
	throw "NAPI binary missing"
}

New-Item -ItemType Directory -Force -Path $destDir | Out-Null
Copy-Item -Force $src $dest

$pdfiumFile = $null
switch ($target) {
	"aarch64-apple-darwin" { $pdfiumFile = "libpdfium.dylib"; break }
	"x86_64-apple-darwin" { $pdfiumFile = "libpdfium.dylib"; break }
	"x86_64-pc-windows-msvc" { $pdfiumFile = "pdfium.dll"; break }
	"aarch64-pc-windows-msvc" { $pdfiumFile = "pdfium.dll"; break }
	"x86_64-unknown-linux-gnu" { $pdfiumFile = "libpdfium.so"; break }
	"aarch64-unknown-linux-gnu" { $pdfiumFile = "libpdfium.so"; break }
	"armv7-unknown-linux-gnueabihf" { $pdfiumFile = "libpdfium.so"; break }
}

if ($null -ne $pdfiumFile) {
	$pdfiumCandidates = @(
		(Join-Path "crates/xberg-node" $pdfiumFile),
		(Join-Path "target/release" $pdfiumFile),
		(Join-Path ("target/" + $target + "/release") $pdfiumFile)
	)

	$pdfiumSrc = $null
	foreach ($candidate in $pdfiumCandidates) {
		if (Test-Path $candidate) {
			$pdfiumSrc = $candidate
			Write-Host ("✓ Found PDFium: " + $candidate)
			break
		}
	}

	if ($null -eq $pdfiumSrc) {
		Write-Host ("⚠ Warning: " + $pdfiumFile + " not found in any expected location")
	} else {
		Write-Host ("Copying " + $pdfiumFile + " to platform directory...")
		Copy-Item -Force $pdfiumSrc (Join-Path $destDir $pdfiumFile)

		$pkgJsonPath = Join-Path $destDir "package.json"
		if (-Not (Test-Path $pkgJsonPath)) { throw ("Platform package.json missing: " + $pkgJsonPath) }
		$pkg = Get-Content $pkgJsonPath -Raw | ConvertFrom-Json
		if ($null -eq $pkg.files) { $pkg | Add-Member -NotePropertyName files -NotePropertyValue @() }
		if (-Not ($pkg.files -contains $pdfiumFile)) { $pkg.files += $pdfiumFile }
		($pkg | ConvertTo-Json -Depth 10) + "`n" | Set-Content -NoNewline -Path $pkgJsonPath
		Write-Host ("✓ Updated package.json to include " + $pdfiumFile)
	}
}

$noticeTool = Join-Path $PSScriptRoot "../../ci/package-notices.py"
& python3 $noticeTool stage $destDir
if ($LASTEXITCODE -ne 0) { throw "Canonical notice staging failed" }

Get-ChildItem -Path $destDir | Out-Host

tar -czf ("node-bindings-" + $env:TARGET + ".tar.gz") -C "crates/xberg-node/npm" $platformDir

& python3 $noticeTool verify ("node-bindings-" + $env:TARGET + ".tar.gz") $platformDir
if ($LASTEXITCODE -ne 0) { throw "Canonical archive notice verification failed" }
