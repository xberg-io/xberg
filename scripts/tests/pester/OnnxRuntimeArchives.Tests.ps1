BeforeAll {
    $script:Sut = Join-Path $PSScriptRoot '../../ci/actions/setup-onnx-runtime/windows.ps1'

    function New-OrtFixture([string]$Architecture, [string]$DllDirectory) {
        $Name = "onnxruntime-win-$Architecture-1.24.2"
        $Root = Join-Path $TestDrive $Name
        New-Item -ItemType Directory -Force -Path (Join-Path $Root 'lib'), (Join-Path $Root $DllDirectory) | Out-Null
        Set-Content (Join-Path $Root 'lib/onnxruntime.lib') 'fixture import library'
        Set-Content (Join-Path $Root "$DllDirectory/onnxruntime.dll") 'fixture runtime'
        $Fixture = Join-Path $TestDrive 'fixture.zip'
        Compress-Archive -LiteralPath $Root -DestinationPath $Fixture -Force
        $Digest = (Get-FileHash -LiteralPath $Fixture -Algorithm SHA256).Hash.ToLowerInvariant()
        Set-Content -LiteralPath $env:ORT_CHECKSUMS_FILE -Value "$Digest  $Name.zip"
    }
}

Describe 'Windows fresh ORT archive verification' {
    BeforeEach {
        $script:SavedEnvironment = @{}
        foreach ($Name in @('TEMP', 'GITHUB_WORKSPACE', 'GITHUB_ENV', 'ORT_CHECKSUMS_FILE', 'RUNNER_ARCH')) {
            $script:SavedEnvironment[$Name] = [Environment]::GetEnvironmentVariable($Name)
        }
        $env:TEMP = Join-Path $TestDrive 'runner'
        $env:GITHUB_WORKSPACE = Join-Path $TestDrive 'workspace'
        $env:GITHUB_ENV = Join-Path $TestDrive 'exports.txt'
        $env:ORT_CHECKSUMS_FILE = Join-Path $TestDrive 'fixture-pins.sha256'
        $env:RUNNER_ARCH = 'ARM64'
        Remove-Item -LiteralPath $env:TEMP, $env:GITHUB_WORKSPACE, $env:GITHUB_ENV -Force -Recurse -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Path $env:TEMP, $env:GITHUB_WORKSPACE | Out-Null
    }

    AfterEach {
        foreach ($Name in $script:SavedEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($Name, $script:SavedEnvironment[$Name])
        }
    }

    It 'rejects an unknown pin before downloading or staging' {
        Remove-Item Env:ORT_CHECKSUMS_FILE
        Mock Invoke-WebRequest { throw 'unexpected download' }
        Mock Expand-Archive { throw 'unexpected extraction' }
        { & $script:Sut '9.9.9' 'native' 'x64' 'system' } | Should -Throw '*exactly one pinned SHA-256 for onnxruntime-win-x64-9.9.9.zip*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 0
        Should -Invoke Expand-Archive -Exactly -Times 0
        Test-Path $env:GITHUB_ENV | Should -BeFalse
    }

    It 'rejects mismatched archive bytes before extraction or exports' {
        New-OrtFixture 'x64' 'lib'
        Mock Invoke-WebRequest { Set-Content -LiteralPath $OutFile -Value 'tampered bytes' }
        Mock Expand-Archive { throw 'unexpected extraction' }
        { & $script:Sut '1.24.2' 'native' 'x64' 'system' } | Should -Throw '*SHA-256 mismatch for onnxruntime-win-x64-1.24.2.zip*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1
        Should -Invoke Expand-Archive -Exactly -Times 0
        Test-Path (Join-Path $env:TEMP 'onnxruntime') | Should -BeFalse
        Test-Path $env:GITHUB_ENV | Should -BeFalse
    }

    It 'verifies a fresh ARM64 archive and preserves the system staging and exports' {
        New-OrtFixture 'arm64' 'lib'
        Mock Invoke-WebRequest { Copy-Item -LiteralPath (Join-Path (Split-Path $env:TEMP) 'fixture.zip') -Destination $OutFile }
        & $script:Sut '1.24.2' 'native' '' 'system'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1 -ParameterFilter { "$Uri" -like '*/v1.24.2/onnxruntime-win-arm64-1.24.2.zip' }
        Get-Content (Join-Path $env:GITHUB_WORKSPACE 'native/onnxruntime.dll') | Should -Be 'fixture runtime'
        Get-Content $env:GITHUB_ENV | Should -Contain 'ORT_STRATEGY=system'
    }

    It 'keeps cached bin DLL staging and bundled exports without another download' {
        $Root = Join-Path $env:TEMP 'onnxruntime/onnxruntime-win-x64-1.24.2'
        New-Item -ItemType Directory -Path (Join-Path $Root 'bin'), (Join-Path $Root 'lib') -Force | Out-Null
        Set-Content (Join-Path $Root 'bin/onnxruntime.dll') 'cached runtime'
        Set-Content (Join-Path $Root 'lib/onnxruntime.lib') 'cached import library'
        Mock Invoke-WebRequest { throw 'unexpected download' }
        & $script:Sut '1.24.2' 'native' 'x64' 'bundled'
        Should -Invoke Invoke-WebRequest -Exactly -Times 0
        Get-Content (Join-Path $env:GITHUB_WORKSPACE 'native/onnxruntime.dll') | Should -Be 'cached runtime'
        Get-Content $env:GITHUB_ENV | Should -Not -Contain 'ORT_STRATEGY=system'
    }
}
