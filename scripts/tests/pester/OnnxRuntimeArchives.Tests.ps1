BeforeAll {
    $script:Sut = Join-Path $PSScriptRoot '../../ci/actions/setup-onnx-runtime/windows.ps1'

    function New-OrtPackage([string]$Architecture) {
        $Package = Join-Path $TestDrive 'package'
        New-Item -ItemType Directory -Force -Path (Join-Path $Package "runtimes/win-$Architecture/native"), (Join-Path $Package 'runtimes/win-other/native'), (Join-Path $Package 'build/native/include') | Out-Null
        Set-Content (Join-Path $Package "runtimes/win-$Architecture/native/onnxruntime.dll") 'nuget runtime'
        Set-Content (Join-Path $Package "runtimes/win-$Architecture/native/onnxruntime.lib") 'nuget import library'
        Set-Content (Join-Path $Package 'runtimes/win-other/native/onnxruntime.dll') 'wrong runtime'
        Set-Content (Join-Path $Package 'build/native/include/onnxruntime_c_api.h') 'header'
        $Fixture = Join-Path $TestDrive 'package.zip'
        Compress-Archive -Path (Join-Path $Package '*') -DestinationPath $Fixture -Force
        $Digest = (Get-FileHash -LiteralPath $Fixture -Algorithm SHA256).Hash.ToLowerInvariant()
        Set-Content -LiteralPath $env:ORT_CHECKSUMS_FILE -Value "$Digest  microsoft.ml.onnxruntime.1.32.0.nupkg"
    }
}

Describe 'Windows ORT NuGet package verification' {
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

    It 'rejects an unpinned version before downloading or staging' {
        Remove-Item Env:ORT_CHECKSUMS_FILE
        Mock Invoke-WebRequest { throw 'unexpected download' }
        Mock Expand-Archive { throw 'unexpected extraction' }
        { & $script:Sut '9.9.9' 'native' 'x64' 'system' } | Should -Throw '*exactly one pinned SHA-256 for microsoft.ml.onnxruntime.9.9.9.nupkg*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 0
        Should -Invoke Expand-Archive -Exactly -Times 0
        Test-Path $env:GITHUB_ENV | Should -BeFalse
    }

    It 'rejects mismatched package bytes before extraction or exports' {
        New-OrtPackage 'x64'
        Mock Invoke-WebRequest { Set-Content -LiteralPath $OutFile -Value 'tampered bytes' }
        Mock Expand-Archive { throw 'unexpected extraction' }
        { & $script:Sut '1.32.0' 'native' 'x64' 'system' } | Should -Throw '*SHA-256 mismatch for microsoft.ml.onnxruntime.1.32.0.nupkg*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1
        Should -Invoke Expand-Archive -Exactly -Times 0
        Test-Path (Join-Path $env:TEMP 'onnxruntime') | Should -BeFalse
        Test-Path $env:GITHUB_ENV | Should -BeFalse
    }

    It 'stages the verified ARM64 NuGet package with the system exports' {
        New-OrtPackage 'arm64'
        Mock Invoke-WebRequest { Copy-Item -LiteralPath (Join-Path (Split-Path $env:TEMP) 'package.zip') -Destination $OutFile }
        & $script:Sut '1.32.0' 'native' '' 'system'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1 -ParameterFilter { "$Uri" -eq 'https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/1.32.0/microsoft.ml.onnxruntime.1.32.0.nupkg' }
        $Root = Join-Path $env:TEMP 'onnxruntime/onnxruntime-win-arm64-1.32.0'
        Get-Content (Join-Path $Root 'lib/onnxruntime.dll') | Should -Be 'nuget runtime'
        Get-Content (Join-Path $Root 'lib/onnxruntime.lib') | Should -Be 'nuget import library'
        Get-Content (Join-Path $Root 'include/onnxruntime_c_api.h') | Should -Be 'header'
        Get-Content (Join-Path $env:GITHUB_WORKSPACE 'native/onnxruntime.dll') | Should -Be 'nuget runtime'
        Get-Content $env:GITHUB_ENV | Should -Contain 'ORT_STRATEGY=system'
        Get-Content $env:GITHUB_ENV | Should -Contain "ORT_LIB_LOCATION=$(Join-Path $Root 'lib')"
    }

    It 'fails without a PyPI fallback when the NuGet download fails, since the wheel has no import library' {
        New-OrtPackage 'x64'
        Mock Invoke-WebRequest { throw 'package not found' }
        Mock Expand-Archive { throw 'unexpected extraction' }
        { & $script:Sut '1.32.0' 'native' 'x64' 'system' } | Should -Throw '*NuGet download of microsoft.ml.onnxruntime.1.32.0.nupkg failed and the PyPI wheel has no import library*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1
        Should -Invoke Invoke-WebRequest -Exactly -Times 0 -ParameterFilter { "$Uri" -like '*pythonhosted*' -or "$Uri" -like '*github.com*' }
        Should -Invoke Expand-Archive -Exactly -Times 0
        Test-Path (Join-Path $env:TEMP 'onnxruntime') | Should -BeFalse
        Test-Path $env:GITHUB_ENV | Should -BeFalse
    }

    It 'keeps cached bin DLL staging and bundled exports without another download' {
        $Root = Join-Path $env:TEMP 'onnxruntime/onnxruntime-win-x64-1.32.0'
        New-Item -ItemType Directory -Path (Join-Path $Root 'bin'), (Join-Path $Root 'lib') -Force | Out-Null
        Set-Content (Join-Path $Root 'bin/onnxruntime.dll') 'cached runtime'
        Set-Content (Join-Path $Root 'lib/onnxruntime.lib') 'cached import library'
        Mock Invoke-WebRequest { throw 'unexpected download' }
        & $script:Sut '1.32.0' 'native' 'x64' 'bundled'
        Should -Invoke Invoke-WebRequest -Exactly -Times 0
        Get-Content (Join-Path $env:GITHUB_WORKSPACE 'native/onnxruntime.dll') | Should -Be 'cached runtime'
        Get-Content $env:GITHUB_ENV | Should -Not -Contain 'ORT_STRATEGY=system'
    }
}
