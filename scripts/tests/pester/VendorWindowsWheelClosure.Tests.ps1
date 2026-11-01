#Requires -Version 7.4
#
# Contract tests for scripts/ci/vendor-windows-wheel-closure.ps1.
#
# The subject is a param() script with top-level side effects and an explicit `exit 0`, which is
# the shape of nearly every .ps1 in this repository. Dot-sourcing it would simply run it, so every
# test invokes it as a child process and asserts on the exit code plus the bytes it leaves behind.
# That is the template the other PowerShell suites copy.
#
# Authoritative on windows-latest in CI. It also passes under pwsh on macOS and Linux, which is
# deliberate: the script touches nothing Windows-specific, so it can be developed and debugged
# locally instead of blind against a runner.

BeforeAll {
    $script:RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' '..' '..')).Path
    $script:Sut = Join-Path $script:RepoRoot 'scripts/ci/vendor-windows-wheel-closure.ps1'
    $script:Pwsh = (Get-Process -Id $PID).Path

    function Write-FakePe([string]$Path) {
        & python (Join-Path $script:RepoRoot 'scripts/ci/test_windows_wheel_closure.py') --write-pe $Path
        if ($LASTEXITCODE -ne 0) { throw 'failed to create portable PE fixture' }
    }

    function New-FakeWheel {
        param(
            [Parameter(Mandatory)][string]$Root,
            [string]$NativeName = 'xberg._native.pyd',
            [switch]$OmitRecord
        )
        $stage = Join-Path $Root ([Guid]::NewGuid().ToString('N'))
        $package = Join-Path $stage 'xberg'
        $distInfo = Join-Path $stage 'xberg-1.2.0.dist-info'
        New-Item -ItemType Directory -Path $package, $distInfo -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $package '__init__.py') -Value '' -NoNewline
        Write-FakePe (Join-Path $package $NativeName)
        if (-not $OmitRecord) {
            # RECORD is CSV and every row is terminated, including the last -- confirmed against
            # the pip and setuptools wheels vendored under integrations/. Dropping the trailing
            # newline here would make Add-Content concatenate onto the final row and the test
            # would be asserting a property of the fixture rather than of the script. ~keep
            Set-Content -LiteralPath (Join-Path $distInfo 'RECORD') -Value "xberg/__init__.py,,`n" -NoNewline
        }
        $wheel = Join-Path $Root ('xberg-1.2.0-cp313-cp313-win_amd64-' + [Guid]::NewGuid().ToString('N') + '.whl')
        Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $wheel -Force
        Remove-Item -Recurse -Force $stage
        return $wheel
    }

    function Invoke-Sut {
        param([Parameter(Mandatory)][string[]]$Arguments)
        $output = & $script:Pwsh -NoProfile -NonInteractive -File $script:Sut @Arguments 2>&1
        [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($output | Out-String) }
    }

    function Expand-Wheel {
        param([Parameter(Mandatory)][string]$Wheel)
        $destination = Join-Path $TestDrive ([Guid]::NewGuid().ToString('N'))
        Expand-Archive -Path $Wheel -DestinationPath $destination -Force
        return $destination
    }

    function Get-RecordText {
        param([Parameter(Mandatory)][string]$Wheel)
        $expanded = Expand-Wheel -Wheel $Wheel
        $record = Get-ChildItem -Path $expanded -Recurse -File -Filter 'RECORD' |
            Where-Object { $_.DirectoryName -like '*.dist-info' } | Select-Object -First 1
        if ($null -eq $record) { return $null }
        Get-Content -LiteralPath $record.FullName -Raw
    }

    # The wheel spec's RECORD hash is urlsafe-base64 of the raw digest with padding stripped, not
    # the hex string Get-FileHash returns. Computing it independently here means the test would
    # catch the script emitting hex, or forgetting the +/ -> -_ substitution. ~keep
    function Get-ExpectedRecordHash {
        param([Parameter(Mandatory)][string]$Path)
        $digest = [System.Security.Cryptography.SHA256]::HashData([System.IO.File]::ReadAllBytes($Path))
        [Convert]::ToBase64String($digest).TrimEnd('=').Replace('+', '-').Replace('/', '_')
    }
}

Describe 'vendor-windows-wheel-closure.ps1' {

    BeforeEach {
        $script:Work = Join-Path $TestDrive ([Guid]::NewGuid().ToString('N'))
        $script:DllDir = Join-Path $script:Work 'dlls'
        New-Item -ItemType Directory -Path $script:Work, $script:DllDir -Force | Out-Null
        Write-FakePe (Join-Path $script:DllDir 'onnxruntime.dll')
        Write-FakePe (Join-Path $script:DllDir 'onnxruntime_providers_shared.dll')
    }

    It 'should_exit_zero_when_the_wheel_contains_a_matching_native_extension' {
        $wheel = New-FakeWheel -Root $script:Work
        (Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd')).ExitCode | Should -Be 0
    }

    It 'should_place_every_source_dll_beside_the_native_extension_when_vendoring' {
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd') | Out-Null

        $names = Get-ChildItem -Path (Join-Path (Expand-Wheel -Wheel $wheel) 'xberg') -File |
            ForEach-Object { $_.Name } | Sort-Object
        # Sorted with the same comparer on both sides: Sort-Object is culture-aware and places
        # '_' before '.', so a hand-ordered expectation disagrees for reasons that have nothing
        # to do with the script. ~keep
        $expected = @('__init__.py', 'onnxruntime.dll', 'onnxruntime_providers_shared.dll', 'xberg._native.pyd') | Sort-Object
        $names | Should -Be $expected
    }

    It 'should_append_a_urlsafe_base64_sha256_record_line_for_each_vendored_dll' {
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd') | Out-Null

        $dll = Join-Path $script:DllDir 'onnxruntime.dll'
        $expected = Get-ExpectedRecordHash -Path $dll
        $size = (Get-Item $dll).Length

        $lines = (Get-RecordText -Wheel $wheel) -split "`r?`n"
        $lines | Should -Contain "xberg/onnxruntime.dll,sha256=$expected,$size"
    }

    It 'should_use_forward_slashes_in_record_paths_regardless_of_host_separator' {
        # The script normalises GetRelativePath's output, which is backslash-separated on Windows.
        # A RECORD written with backslashes installs but fails `pip check`-style verification.
        #
        # This assertion can only FAIL on Windows: everywhere else GetRelativePath already returns
        # forward slashes, so deleting the `-replace` from the script leaves this test green.
        # Verified by mutation -- three of the four controls this suite guards fail locally when
        # removed, and this is the one that does not. It is left running on every host because it
        # costs nothing and is the real gate on the windows-latest leg, but a green run on macOS
        # or Linux is not evidence that the normalisation is present. ~keep
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd') | Out-Null

        $added = ((Get-RecordText -Wheel $wheel) -split "`r?`n") | Where-Object { $_ -like '*onnxruntime.dll*' }
        $added | Should -Not -BeNullOrEmpty
        $added | ForEach-Object { $_ | Should -Not -Match '\\' }
    }

    It 'should_preserve_the_preexisting_record_entries_when_appending' {
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd') | Out-Null
        ((Get-RecordText -Wheel $wheel) -split "`r?`n") | Should -Contain 'xberg/__init__.py,,'
    }

    It 'should_skip_a_dll_without_adding_a_second_record_line_when_it_is_already_present' {
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd') | Out-Null
        $first = Get-RecordText -Wheel $wheel

        $second = Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd')

        $second.ExitCode | Should -Be 0
        $second.Output | Should -Match 'skipping onnxruntime\.dll, already present'
        Get-RecordText -Wheel $wheel | Should -Be $first
    }

    It 'should_exit_nonzero_and_name_the_glob_when_no_native_file_matches' {
        $wheel = New-FakeWheel -Root $script:Work
        $result = Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.nope')
        $result.ExitCode | Should -Not -Be 0
        $result.Output | Should -Match "no native file matching '\*\.nope'"
    }

    It 'should_exit_nonzero_when_the_wheel_has_no_dist_info_record' {
        $wheel = New-FakeWheel -Root $script:Work -OmitRecord
        $result = Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.pyd')
        $result.ExitCode | Should -Not -Be 0
        $result.Output | Should -Match 'no \*\.dist-info/RECORD found'
    }

    It 'should_exit_nonzero_when_the_dll_source_directory_holds_no_dlls' {
        $wheel = New-FakeWheel -Root $script:Work
        $empty = Join-Path $script:Work 'empty'
        New-Item -ItemType Directory -Path $empty -Force | Out-Null
        $result = Invoke-Sut -Arguments @($wheel, $empty, '*.pyd')
        $result.ExitCode | Should -Not -Be 0
        $result.Output | Should -Match 'no \*\.dll files found'
    }

    It 'should_exit_nonzero_when_the_wheel_path_does_not_exist' {
        $result = Invoke-Sut -Arguments @((Join-Path $script:Work 'absent.whl'), $script:DllDir, '*.pyd')
        $result.ExitCode | Should -Not -Be 0
        $result.Output | Should -Match 'wheel not found'
    }

    It 'should_exit_nonzero_when_the_dll_source_directory_does_not_exist' {
        $wheel = New-FakeWheel -Root $script:Work
        $result = Invoke-Sut -Arguments @($wheel, (Join-Path $script:Work 'absent'), '*.pyd')
        $result.ExitCode | Should -Not -Be 0
        $result.Output | Should -Match 'DLL source dir not found'
    }

    It 'should_leave_no_temporary_working_directory_behind_when_it_fails' {
        # The subject builds its scratch dir under the system temp and removes it in `finally`.
        # A failure that skipped the cleanup would leak an expanded wheel per CI run.
        $before = @(Get-ChildItem -Path ([System.IO.Path]::GetTempPath()) -Directory -Filter 'vendor-wheel-*').Count
        $wheel = New-FakeWheel -Root $script:Work
        Invoke-Sut -Arguments @($wheel, $script:DllDir, '*.nope') | Out-Null
        $after = @(Get-ChildItem -Path ([System.IO.Path]::GetTempPath()) -Directory -Filter 'vendor-wheel-*').Count
        $after | Should -Be $before
    }
}
