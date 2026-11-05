param(
    [Parameter(Mandatory = $true)][string]$WheelDirectory,
    [Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceSha,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [ValidatePattern('^[0-9a-f]{40}$')][string]$WheelSourceSha = $SourceSha
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# ~keep This exact Server Core 2022 manifest shares the hosted 2022 kernel, never its registry/filesystem.
$image = 'mcr.microsoft.com/windows/servercore@sha256:df8887a9e3ea17894720a023154fecce9dc6cc685ca5ff718a7f197f1dcff182'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit $LASTEXITCODE" }
}
$actual = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -ne $SourceSha) { throw 'Checkout is not the exact requested source SHA' }
$wheels = @(Get-ChildItem -LiteralPath $WheelDirectory -Filter '*.whl')
if ($wheels.Count -ne 1 -or $wheels[0].Name -notmatch 'win_amd64\.whl$') { throw 'Expected exactly one Windows x64 wheel' }
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Output path already exists; refusing stale receipts' }
$null = New-Item -ItemType Directory -Path $OutputDirectory
$output = (Resolve-Path $OutputDirectory).Path
$pythonRoot = (& mise where python).Trim()
if ($LASTEXITCODE -ne 0 -or -not (Test-Path "$pythonRoot/python.exe")) { throw 'Locked mise Python installation unavailable' }
$lock = & "$pythonRoot/python.exe" -c 'import json,tomllib; p=tomllib.load(open("mise.toml","rb"))["tools"]["python"]; l=tomllib.load(open("mise.lock","rb"))["tools"]["python"]; print(json.dumps({"version":p,"lock":l}))'
if ($LASTEXITCODE -ne 0) { throw 'Cannot read pinned Python provenance' }
$pythonInfo = (& "$pythonRoot/python.exe" -c 'import platform; print(platform.python_version())').Trim()
if ($LASTEXITCODE -ne 0 -or $pythonInfo -ne ($lock | ConvertFrom-Json).version) { throw 'Mise Python differs from repository pin' }
$pythonHash = (Get-FileHash "$pythonRoot/python.exe" -Algorithm SHA256).Hash.ToLowerInvariant()
$dockerOs = (& docker info --format '{{.OSType}}').Trim()
if ($LASTEXITCODE -ne 0 -or $dockerOs -ne 'windows') { throw 'Windows Docker service is required' }
Invoke-Checked -Program docker -Arguments @('pull', $image)
$imageInfo = (& docker image inspect $image | ConvertFrom-Json)[0]
if ($LASTEXITCODE -ne 0 -or $imageInfo.Architecture -ne 'amd64' -or $imageInfo.OsVersion -notmatch '^10\.0\.20348\.') { throw 'Wrong container architecture or OS build' }
$cases = @()
foreach ($case in @('intact', 'missing-runtime')) {
    $container = 'xberg-wheel-' + [guid]::NewGuid().ToString('N')
    $payload = Join-Path $output $case
    $null = New-Item -ItemType Directory -Path $payload
    Copy-Item $wheels[0].FullName $payload
    foreach ($script in @('windows_wheel_clean_host.py', 'windows_wheel_closure.py', 'smoke_test_python_wheel.py')) {
        Copy-Item (Join-Path $PSScriptRoot $script) $payload
    }
    Copy-Item (Join-Path $PSScriptRoot 'fixtures/musl-python-smoke.pdf') $payload
    try {
        Invoke-Checked -Program docker -Arguments @('create', '--isolation=process', '--name', $container, $image, 'cmd', '/c', 'ping -t 127.0.0.1 >NUL')
        Invoke-Checked -Program docker -Arguments @('start', $container)
        Invoke-Checked -Program docker -Arguments @('exec', $container, 'cmd', '/c', 'mkdir C:\qualification')
        Invoke-Checked -Program docker -Arguments @('cp', "$payload/.", "${container}:C:\qualification")
        Invoke-Checked -Program docker -Arguments @('cp', $pythonRoot, "${container}:C:\qualification\python")
        Invoke-Checked -Program docker -Arguments @('exec', '--env', 'PATH=C:\qualification\python;C:\Windows\System32;C:\Windows', '--env', 'PYTHONPATH=', '--env', 'ORT_LIB_LOCATION=', $container, 'C:\qualification\python\python.exe', '-E', '-s', 'C:\qualification\windows_wheel_clean_host.py', 'qualify', '--root', 'C:\qualification', '--case', $case)
        Invoke-Checked -Program docker -Arguments @('cp', "${container}:C:\qualification\receipt.json", "$payload/receipt.json")
        $receipt = Get-Content "$payload/receipt.json" -Raw | ConvertFrom-Json
        if ($receipt.case -ne $case -or $receipt.wheel_sha256 -ne (Get-FileHash $wheels[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()) { throw 'Receipt case/wheel mismatch' }
        if ($receipt.python_sha256 -ne $pythonHash) { throw 'Container Python differs from the copied locked interpreter' }
        $cases += $receipt
    } finally {
        & docker rm --force $container
        if ($LASTEXITCODE -ne 0) { throw "Failed to remove disposable container $container" }
    }
}
if ($cases.Count -ne 2 -or $cases[0].extractions -ne 1 -or $cases[1].extractions -ne 0 -or $cases[1].expected_native_failures -ne 1) { throw 'Incomplete qualification case/extraction counts' }
@{
    source_sha = $WheelSourceSha
    verifier_sha = $actual
    wheel_sha256 = (Get-FileHash $wheels[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    image = $image
    image_id = $imageInfo.Id
    image_os_version = $imageInfo.OsVersion
    isolation = 'process'
    python = ($lock | ConvertFrom-Json)
    container_cases = 2
    successful_extractions = 1
    expected_native_failures = 1
    cases = $cases
} | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $output 'qualification.json')
