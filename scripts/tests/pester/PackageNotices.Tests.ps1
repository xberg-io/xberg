#Requires -Version 7.4

BeforeAll {
    $script:RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
    $script:NoticeTool = Join-Path $script:RepoRoot 'scripts/ci/package-notices.py'
    $script:Pwsh = (Get-Process -Id $PID).Path
}

Describe 'PowerShell archive notices' {
    It 'should ship canonical bytes in the Windows CLI ZIP' {
        $root = Join-Path $TestDrive 'cli'
        New-Item -ItemType Directory -Force "$root/target/x86_64-pc-windows-msvc/release" | Out-Null
        Set-Content "$root/target/x86_64-pc-windows-msvc/release/xberg.exe" 'native fixture'
        Set-Content "$root/README.md" 'readme fixture'
        Push-Location $root
        try {
            & $script:Pwsh -NoProfile -Command "`$env:CLI_TARGET='x86_64-pc-windows-msvc'; & '$script:RepoRoot/scripts/publish/cli/package-cli-windows.ps1'"
            $LASTEXITCODE | Should -Be 0
            & python3 $script:NoticeTool verify 'xberg-cli-x86_64-pc-windows-msvc.zip' .
            $LASTEXITCODE | Should -Be 0
        } finally {
            Pop-Location
        }
    }

    It 'should ship canonical bytes and retain the npm allowlist in a platform tarball' {
        $root = Join-Path $TestDrive 'node'
        $platform = "$root/crates/xberg-node/npm/win32-x64-msvc"
        New-Item -ItemType Directory -Force -Path $platform, "$root/crates/xberg-node/artifacts" | Out-Null
        Set-Content "$root/crates/xberg-node/artifacts/xberg-node.win32-x64-msvc.node" 'native fixture'
        Set-Content "$platform/package.json" '{"files":["binding.node"]}'
        Push-Location $root
        try {
            & $script:Pwsh -NoProfile -Command "`$env:TARGET='x86_64-pc-windows-msvc'; & '$script:RepoRoot/scripts/publish/node/package-artifacts.ps1'"
            $LASTEXITCODE | Should -Be 0
            & python3 $script:NoticeTool verify 'node-bindings-x86_64-pc-windows-msvc.tar.gz' win32-x64-msvc
            $LASTEXITCODE | Should -Be 0
            $package = Get-Content "$platform/package.json" -Raw | ConvertFrom-Json
            $package.files | Should -Contain 'LICENSE'
            $package.files | Should -Contain 'THIRD_PARTY_LICENSES.md'
            $package.files | Should -Contain 'binding.node'
        } finally {
            Pop-Location
        }
    }
}
