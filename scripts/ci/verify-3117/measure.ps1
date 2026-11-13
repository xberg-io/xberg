$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$evidence = Join-Path $env:RUNNER_TEMP "xberg-3117"
$scratch = Join-Path $env:RUNNER_TEMP "probe-3117"
$null = New-Item -ItemType Directory -Force $evidence, $scratch
$fixtures = Join-Path $scratch "fixtures"
$root = Join-Path $scratch "source"
git worktree add --detach $root $env:SOURCE_SHA
if ((git -C $root rev-parse HEAD) -cne $env:SOURCE_SHA) { throw "Unexpected source SHA" }

$target = "x86_64-pc-windows-msvc"
$env:CARGO_TARGET_DIR = Join-Path $scratch "target"
Push-Location $root
try {
    cargo build --locked -p xberg-ffi --release --target $target 2>&1 |
        Tee-Object (Join-Path $evidence "native-build.log")
} finally {
    Pop-Location
}
$native = Get-Item "$env:CARGO_TARGET_DIR/$target/release/xberg_ffi.dll"

@(
    "source_sha=$env:SOURCE_SHA"
    "workflow_sha=$env:WORKFLOW_SHA"
    "label=$env:SAMPLE_LABEL"
    "runner=$env:RUNNER_OS/$env:RUNNER_ARCH"
    "runner_image=$env:ImageOS/$env:ImageVersion"
    "dotnet=$(dotnet --version)"
    "rustc=$(rustc --version)"
    "native_sha256=$((Get-FileHash $native.FullName).Hash)"
) | Set-Content (Join-Path $evidence "provenance.txt")

python "$PSScriptRoot/fixtures.py" "$fixtures"
Get-ChildItem "$fixtures/*.pdf" | ForEach-Object {
    [pscustomobject]@{ name = $_.Name; bytes = $_.Length; sha256 = (Get-FileHash $_.FullName).Hash }
} | Export-Csv -NoTypeInformation (Join-Path $evidence "fixtures.csv")

$project = Join-Path $scratch "Probe.csproj"
@"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net10.0</TargetFramework>
    <PlatformTarget>x64</PlatformTarget>
    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="Program.cs" />
    <ProjectReference Include="$root/packages/csharp/Xberg/Xberg.csproj" />
  </ItemGroup>
</Project>
"@ | Set-Content $project
Copy-Item "$PSScriptRoot/Program.cs" $scratch
$compileItems = (dotnet msbuild $project -getItem:Compile | ConvertFrom-Json).Items.Compile
if ($compileItems.Count -ne 1 -or $compileItems[0].Identity -cne "Program.cs") {
    throw "Expected Program.cs as the only Probe.csproj compile item; found $($compileItems.Count) items"
}
$output = Join-Path $scratch "app"
dotnet build $project --configuration Release --output $output |
    Tee-Object (Join-Path $evidence "managed-build.log")
Copy-Item $native.FullName, $env:ORT_DYLIB_PATH $output

$rows = foreach ($mib in 0, 40, 90) {
    foreach ($sample in 1..3) {
        $result = dotnet "$output/Probe.dll" "$fixtures/unref0.pdf" "$fixtures/unref$mib.pdf"
        $row = $result | ConvertFrom-Json
        if ($row.fixture -cne "unref$mib.pdf" -or $row.resultCount -ne 1 -or
            $row.content -cne "Hello" -or $row.deltaBytes -lt 0) {
            throw "Invalid probe result for $mib MiB sample $sample"
        }
        $row | Add-Member -NotePropertyName sample -NotePropertyValue $sample
        $row | ConvertTo-Json -Compress | Add-Content (Join-Path $evidence "samples.jsonl")
        $row
    }
}
if ($rows.Count -ne 9) { throw "Expected nine fresh-process results" }
$medians = foreach ($mib in 0, 40, 90) {
    $values = @($rows | Where-Object fixture -CEQ "unref$mib.pdf" | ForEach-Object deltaBytes | Sort-Object)
    if ($values.Count -ne 3) { throw "Expected three samples for $mib MiB" }
    [pscustomobject]@{ streamMiB = $mib; samples = $values.Count; medianDeltaBytes = $values[1] }
}
$medians | Export-Csv -NoTypeInformation (Join-Path $evidence "summary.csv")
$medians | Format-Table
