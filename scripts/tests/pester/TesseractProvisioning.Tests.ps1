BeforeAll {
    $script:Sut = Join-Path $PSScriptRoot '../../ci/install-system-deps/ensure-tesseract-windows.ps1'
    . $script:Sut
}

Describe 'Tesseract provisioning' {
    BeforeEach {
        Mock Get-TesseractVersion { [version]'5.5.5' }
        Mock Invoke-WebRequest {}
        Mock Get-Item { @{ Length = 2752925 } }
        Mock Get-FileHash { @{ Hash = 'D590AC276138238EB80E4D5833F65CB057CB90D02088D78A05027190A5318E41' } }
        Mock Build-TesseractSource {}
        Mock Assert-TesseractRuntime {}
        $script:Archive = Join-Path $TestDrive 'source.zip'
    }

    It 'should reuse a verified current binary without downloading' {
        Ensure-Tesseract -Executable 'fixture.exe' -SourceArchive $script:Archive -VcpkgRoot $TestDrive
        Should -Invoke Invoke-WebRequest -Exactly -Times 0
        Should -Invoke Build-TesseractSource -Exactly -Times 0
    }

    It 'should upgrade 5.5.3 from the verified published source and verify the binary' {
        $script:VersionCalls = 0
        Mock Get-TesseractVersion {
            $script:VersionCalls++
            if ($script:VersionCalls -eq 1) { return [version]'5.5.3' }
            return [version]'5.5.5'
        }
        Ensure-Tesseract -Executable (Join-Path $TestDrive 'fixture.exe') -SourceArchive $script:Archive -VcpkgRoot $TestDrive
        Should -Invoke Invoke-WebRequest -Exactly -Times 1 -ParameterFilter {
            $Uri -eq 'https://codeload.github.com/tesseract-ocr/tesseract/zip/4292f6890aa86ef64eba32090358f40f69dd6d28'
        }
        Should -Invoke Build-TesseractSource -Exactly -Times 1
        $script:VersionCalls | Should -BeExactly 2
    }

    It 'should reject a corrupt source before building' {
        Mock Get-TesseractVersion { [version]'5.5.3' }
        Mock Get-FileHash { @{ Hash = ('0' * 64) } }
        { Ensure-Tesseract -Executable 'fixture.exe' -SourceArchive $script:Archive -VcpkgRoot $TestDrive } |
            Should -Throw '*checksum mismatch*'
        Should -Invoke Build-TesseractSource -Exactly -Times 0
    }

    It 'should reject a truncated source before building' {
        Mock Get-TesseractVersion { [version]'5.5.3' }
        Mock Get-Item { @{ Length = 0 } }
        { Ensure-Tesseract -Executable 'fixture.exe' -SourceArchive $script:Archive -VcpkgRoot $TestDrive } |
            Should -Throw '*size or checksum mismatch*'
        Should -Invoke Build-TesseractSource -Exactly -Times 0
    }

    It 'should preserve a native build failure' {
        Mock Get-TesseractVersion { [version]'5.5.3' }
        Mock Build-TesseractSource { throw 'native build failed with exit 7' }
        { Ensure-Tesseract -Executable 'fixture.exe' -SourceArchive $script:Archive -VcpkgRoot $TestDrive } |
            Should -Throw '*exit 7*'
    }

    It 'should reject a stale binary after reported source installation success' {
        Mock Get-TesseractVersion { [version]'5.5.3' }
        { Ensure-Tesseract -Executable 'fixture.exe' -SourceArchive $script:Archive -VcpkgRoot $TestDrive } |
            Should -Throw '*below required 5.5.5*'
    }
}

Describe 'Tesseract source command status' {
    It 'should bound compiler processes and restore the caller environment after success' {
        $before = [Environment]::GetEnvironmentVariable('_CL_', 'Process')
        { Invoke-TesseractBuildCommand -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'if ($env:_CL_ -notmatch "/MP1$") { exit 9 }; exit 0') } |
            Should -Not -Throw
        [Environment]::GetEnvironmentVariable('_CL_', 'Process') | Should -BeExactly $before
    }

    It 'should reject a native failure' {
        { Invoke-TesseractBuildCommand -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'exit 7') } |
            Should -Throw '*exit 7*'
    }

    It 'should not allow stdout to hide a native failure' {
        { Invoke-TesseractBuildCommand -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', 'Write-Output "success-looking output"; exit 7') } |
            Should -Throw '*exit 7*'
    }
}

Describe 'Published Tesseract source build orchestration' {
    BeforeEach {
        $script:Commands = [System.Collections.Generic.List[object]]::new()
        $script:OriginalConcurrency = [Environment]::GetEnvironmentVariable('VCPKG_MAX_CONCURRENCY', 'Process')
        $env:VCPKG_MAX_CONCURRENCY = '13'
        $leptNotice = Join-Path $TestDrive 'installed/x64-windows-static-md/share/leptonica'
        New-Item -ItemType Directory -Path $leptNotice -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $leptNotice 'copyright') -Value 'controlled Leptonica notice'
        $script:InstallRoot = Join-Path $TestDrive 'install'
        New-Item -ItemType Directory -Path (Join-Path $script:InstallRoot 'bin') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $script:InstallRoot 'bin/tesseract.exe') -Value 'test executable'
        Mock Invoke-TesseractBuildCommand {
            param($Executable, $Arguments)
            $script:Commands.Add(@{ Executable = $Executable; Arguments = $Arguments; Concurrency = $env:VCPKG_MAX_CONCURRENCY })
        }
        Mock Expand-Archive {
            param($LiteralPath, $DestinationPath)
            $source = Join-Path $DestinationPath 'tesseract-4292f6890aa86ef64eba32090358f40f69dd6d28'
            New-Item -ItemType Directory -Path $source -Force | Out-Null
            foreach ($name in @('CMakeLists.txt', 'LICENSE', 'AUTHORS')) {
                Set-Content -LiteralPath (Join-Path $source $name) -Value "controlled $name"
            }
        }
    }

    AfterEach { [Environment]::SetEnvironmentVariable('VCPKG_MAX_CONCURRENCY', $script:OriginalConcurrency, 'Process') }

    It 'should provision Leptonica and build only the selected source with one native job' {
        Build-TesseractSource -SourceArchive 'verified-source.zip' -VcpkgRoot $TestDrive -InstallRoot $script:InstallRoot
        $script:Commands.Count | Should -BeExactly 3
        $script:Commands[0].Arguments | Should -Contain 'leptonica:x64-windows-static-md'
        $script:Commands[0].Concurrency | Should -BeExactly '1'
        $env:VCPKG_MAX_CONCURRENCY | Should -BeExactly '13'
        $script:Commands[1].Arguments | Should -Contain '-DVCPKG_TARGET_TRIPLET=x64-windows-static-md'
        $script:Commands[1].Arguments | Should -Contain '-DBUILD_TRAINING_TOOLS=OFF'
        $script:Commands[2].Arguments[-2] | Should -BeExactly '--parallel'
        $script:Commands[2].Arguments[-1] | Should -BeExactly '1'
        Test-Path -LiteralPath (Join-Path $script:InstallRoot 'tesseract.exe') | Should -BeTrue
        Get-Content -LiteralPath (Join-Path $script:InstallRoot 'share/licenses/xberg-system-ocr/tesseract-5.5.5/AUTHORS') |
            Should -BeExactly 'controlled AUTHORS'
        Get-Content -LiteralPath (Join-Path $script:InstallRoot 'share/licenses/xberg-system-ocr/leptonica-1.87.0/leptonica-license.txt') |
            Should -BeExactly 'controlled Leptonica notice'
    }

    It 'should reject an empty extraction before any native work' {
        Mock Expand-Archive {}
        { Build-TesseractSource -SourceArchive 'verified-source.zip' -VcpkgRoot $TestDrive -InstallRoot $script:InstallRoot } |
            Should -Throw '*no expected source*'
        $script:Commands.Count | Should -BeExactly 0
    }

    It 'should stop when Leptonica provisioning fails' {
        Mock Invoke-TesseractBuildCommand { throw 'native dependency exit 7' }
        { Build-TesseractSource -SourceArchive 'verified-source.zip' -VcpkgRoot $TestDrive -InstallRoot $script:InstallRoot } |
            Should -Throw '*exit 7*'
        Should -Invoke Invoke-TesseractBuildCommand -Exactly -Times 1
        $env:VCPKG_MAX_CONCURRENCY | Should -BeExactly '13'
    }
}

Describe 'Tesseract traineddata integrity' {
    BeforeEach {
        Mock Invoke-WebRequest {}
        Mock Test-Path { $false }
        Mock Get-Item { param($LiteralPath) @{ Length = if ($LiteralPath -like '*eng.traineddata') { 4113088 } else { 10562727 } } }
        Mock Get-FileHash {
            param($LiteralPath)
            @{ Hash = if ($LiteralPath -like '*eng.traineddata') {
                '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2'
            } else { '9cf5d576fcc47564f11265841e5ca839001e7e6f38ff7f7aacf46d15a96b00ff' } }
        }
    }

    It 'should retrieve both verified models from the immutable source revision' {
        Ensure-TesseractData -Directory (Join-Path $TestDrive 'data')
        Should -Invoke Invoke-WebRequest -Exactly -Times 2 -ParameterFilter {
            $Uri -like 'https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/87416418657359cb625c412a48b6e1d6d41c29bd/*.traineddata'
        }
    }

    It 'should reject a corrupt model before continuing' {
        Mock Get-FileHash { @{ Hash = ('0' * 64) } }
        { Ensure-TesseractData -Directory (Join-Path $TestDrive 'data') } | Should -Throw '*checksum mismatch*'
        Should -Invoke Invoke-WebRequest -Exactly -Times 1
    }
}

Describe 'Tesseract executable version detection' {
    BeforeAll {
        function Invoke-TesseractVersionFixture {
            $global:LASTEXITCODE = $script:ProbeExitCode
            $script:ProbeOutput
        }
    }

    BeforeEach {
        $script:ProbeExitCode = 0
        $script:ProbeOutput = @('tesseract 5.5.5', ' leptonica-1.87.0')
        Mock Test-Path { $true }
    }

    It 'should parse the native banner <Banner>' -ForEach @(
        @{ Banner = 'tesseract 5.5.5' }
        @{ Banner = 'tesseract v5.5.5.20261028' }
        @{ Banner = 'tesseract v5.5.0.20241111' }
    ) {
        $script:ProbeOutput = @($Banner, ' leptonica-1.87.0')
        $expected = if ($Banner -like '*5.5.0*') { [version]'5.5.0' } else { [version]'5.5.5' }
        Get-TesseractVersion -Executable 'Invoke-TesseractVersionFixture' | Should -BeExactly $expected
    }

    It 'should reject a successful unrecognized banner' {
        $script:ProbeOutput = @('not tesseract 5.5.5')
        { Get-TesseractVersion -Executable 'Invoke-TesseractVersionFixture' } |
            Should -Throw '*no recognized version*'
    }

    It 'should reject a failing native probe even when its banner is valid' {
        $script:ProbeExitCode = 2
        { Get-TesseractVersion -Executable 'Invoke-TesseractVersionFixture' } |
            Should -Throw '*probe failed: 2*'
    }
}

Describe 'Actual Windows OCR runtime versions' {
    BeforeAll {
        function Invoke-TesseractRuntimeFixture {
            $global:LASTEXITCODE = $script:RuntimeExit
            $script:RuntimeLines
        }
    }
    BeforeEach {
        $script:RuntimeExit = 0
        $script:RuntimeLines = @('tesseract 5.5.5', ' leptonica-1.87.0')
    }
    It 'should verify the actual selected Tesseract and Leptonica pair' {
        { Assert-TesseractRuntime -Executable 'Invoke-TesseractRuntimeFixture' } | Should -Not -Throw
    }
    It 'should reject current Tesseract linked to stale Leptonica' {
        $script:RuntimeLines = @('tesseract 5.5.5', ' leptonica-1.86.0')
        { Assert-TesseractRuntime -Executable 'Invoke-TesseractRuntimeFixture' } | Should -Throw '*actual Tesseract*'
    }
    It 'should reject a runtime command failure despite a valid banner' {
        $script:RuntimeExit = 7
        { Assert-TesseractRuntime -Executable 'Invoke-TesseractRuntimeFixture' } | Should -Throw '*probe failed: 7*'
    }
}

Describe 'Tesseract build environment restoration' {
    It 'should restore compiler flags <Value> after command failure=<Fails>' -ForEach @(
        @{ Value = $null; Fails = $false }
        @{ Value = ''; Fails = $false }
        @{ Value = '/DORIGINAL'; Fails = $false }
        @{ Value = $null; Fails = $true }
        @{ Value = ''; Fails = $true }
        @{ Value = '/DORIGINAL'; Fails = $true }
    ) {
        $original = $env:_CL_
        try {
            $env:_CL_ = $Value
            $expected = [Environment]::GetEnvironmentVariable('_CL_', 'Process')
            $code = if ($Fails) { 'exit 7' } else { 'exit 0' }
            $command = { Invoke-TesseractBuildCommand -Executable (Join-Path $PSHOME 'pwsh') -Arguments @('-NoProfile', '-Command', $code) }
            if ($Fails) { $command | Should -Throw '*exit 7*' }
            else { $command | Should -Not -Throw }
            [Environment]::GetEnvironmentVariable('_CL_', 'Process') | Should -BeExactly $expected
        }
        finally { $env:_CL_ = $original }
    }

    It 'should restore vcpkg concurrency <Value> after build failure=<Fails>' -ForEach @(
        @{ Value = $null; Fails = $false }
        @{ Value = ''; Fails = $false }
        @{ Value = '13'; Fails = $false }
        @{ Value = $null; Fails = $true }
        @{ Value = ''; Fails = $true }
        @{ Value = '13'; Fails = $true }
    ) {
        $original = $env:VCPKG_MAX_CONCURRENCY
        try {
            $env:VCPKG_MAX_CONCURRENCY = $Value
            $expected = [Environment]::GetEnvironmentVariable('VCPKG_MAX_CONCURRENCY', 'Process')
            Mock Invoke-TesseractBuildCommand {
                $env:VCPKG_MAX_CONCURRENCY | Should -BeExactly '1'
                if ($Fails) { throw 'controlled build failure' }
            }
            $command = { Install-TesseractLeptonica -VcpkgRoot $TestDrive }
            if ($Fails) { $command | Should -Throw '*controlled build failure*' }
            else { $command | Should -Not -Throw }
            [Environment]::GetEnvironmentVariable('VCPKG_MAX_CONCURRENCY', 'Process') | Should -BeExactly $expected
        }
        finally { $env:VCPKG_MAX_CONCURRENCY = $original }
    }
}
