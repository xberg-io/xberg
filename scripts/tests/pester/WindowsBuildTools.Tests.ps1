BeforeAll {
    . (Join-Path $PSScriptRoot "../../ci/install-system-deps/ensure-build-tools-windows.ps1")
    $installer = Join-Path $PSScriptRoot '../../ci/install-system-deps/install-windows.ps1'
    $tokens = $null
    $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($installer, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) { throw 'Windows dependency installer must parse' }
    $retry = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Retry-Command'
    }, $false))
    if ($retry.Count -ne 1) { throw 'Expected one production Retry-Command function' }
    . ([scriptblock]::Create($retry[0].Extent.Text))
    function choco {}
}

Describe 'Windows build tool installation retries' {
    BeforeEach {
        $script:InstallCalls = 0
        Mock Start-Sleep {}
        Mock choco {
            $script:InstallCalls++
            $global:LASTEXITCODE = 0
        }
    }

    It 'retries a success-looking install until the required version exists' {
        Mock Test-BuildToolVersion { $script:InstallCalls -ge 2 }

        { Ensure-BuildTool -Package llvm -Executable fixture.exe -Version '23.1.3' } | Should -Not -Throw

        Should -Invoke choco -Exactly -Times 2
        Should -Invoke Start-Sleep -Exactly -Times 1
    }

    It 'fails after three success-looking installs leave the required version missing' {
        Mock Test-BuildToolVersion { $false }

        { Ensure-BuildTool -Package llvm -Executable fixture.exe -Version '23.1.3' } |
            Should -Throw '*Failed to install llvm 23.1.3*'

        Should -Invoke choco -Exactly -Times 3
        Should -Invoke Start-Sleep -Exactly -Times 2
    }

    It 'does not install an already verified tool' {
        Mock Test-BuildToolVersion { $true }

        { Ensure-BuildTool -Package llvm -Executable fixture.exe -Version '23.1.3' } | Should -Not -Throw

        Should -Invoke choco -Exactly -Times 0
        Should -Invoke Start-Sleep -Exactly -Times 0
    }

    It 'preserves native installation failures even when a tool becomes available' {
        Mock Test-BuildToolVersion { $script:InstallCalls -gt 0 }
        Mock choco {
            $script:InstallCalls++
            $global:LASTEXITCODE = 7
        }

        { Ensure-BuildTool -Package llvm -Executable fixture.exe -Version '23.1.3' } |
            Should -Throw '*Failed to install llvm 23.1.3*'

        Should -Invoke choco -Exactly -Times 3
    }

    It 'still verifies the installed version after the retry completes' {
        $script:VersionChecks = 0
        Mock Test-BuildToolVersion {
            $script:VersionChecks++
            $script:VersionChecks -eq 2
        }

        { Ensure-BuildTool -Package llvm -Executable fixture.exe -Version '23.1.3' } |
            Should -Throw '*fixture.exe does not report required version 23.1.3*'
    }
}
Describe 'Windows build tool runtime selection' {
    It 'accepts exactly the selected runtime' {
        Test-BuildToolVersion -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'Write-Output "clang version 23.1.3"') -Version '23.1.3' | Should -BeTrue
    }
    It 'rejects stale cached versions' {
        Test-BuildToolVersion -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'Write-Output "clang version 22.1.8"') -Version '23.1.3' | Should -BeFalse
    }
    It 'rejects a successful-looking version with native failure' {
        Test-BuildToolVersion -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'Write-Output "cmake version 4.4.4"; exit 7') -Version '4.4.4' | Should -BeFalse
    }
    It 'rejects absent executables and empty output' {
        Test-BuildToolVersion -Executable 'missing-xberg-build-tool' -Version '4.4.4' | Should -BeFalse
        Test-BuildToolVersion -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'exit 0') -Version '4.4.4' | Should -BeFalse
    }
}
