BeforeAll {
    $path = Join-Path $PSScriptRoot '../../ci/install-system-deps/install-windows.ps1'
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
    if ($errors.Count -ne 0) { throw 'Windows installer did not parse' }
    $functions = @($ast.FindAll({ param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -in @('Retry-Command', 'Assert-LibheifVersion', 'Assert-LibheifArtifacts')
    }, $true))
    if ($functions.Count -ne 3) { throw 'Expected all three actual dependency validation functions' }
    foreach ($function in $functions) { . ([scriptblock]::Create($function.Extent.Text)) }
}

Describe 'Windows native dependency retries' {
    It 'should not report a native nonzero exit as successful' {
        Retry-Command { & (Join-Path $PSHOME 'pwsh') -NoProfile -Command 'exit 7' } -MaxAttempts 1 |
            Should -BeFalse
    }

    It 'should not let native stdout hide a failed exit status' {
        Retry-Command {
            & (Join-Path $PSHOME 'pwsh') -NoProfile -Command 'Write-Output "native diagnostic"; exit 7'
        } -MaxAttempts 1 | Should -BeFalse
    }

    It 'should accept an actual successful native command' {
        Retry-Command { & (Join-Path $PSHOME 'pwsh') -NoProfile -Command 'exit 0' } -MaxAttempts 1 |
            Should -BeTrue
    }
}

Describe 'Installed Windows libheif validation' {
    It 'should accept a combined feature row carrying the version' {
        # Older vcpkg collapses the features onto the versioned row instead of
        # emitting separate version-less rows.
        { Assert-LibheifVersion -Packages @(
            'libheif[aom,libde265]:x64-windows                   1.23.7#1   HEIF decoder'
        ) } | Should -Not -Throw
    }

    It 'should accept one versioned core row and version-less feature rows' {
        # Real `vcpkg list libheif` shape: the core row carries the version, each
        # enabled feature is a separate row with no version column.
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                                1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                                       AVIF support',
            'libheif[libde265]:x64-windows                                  libde265 HEVC decoder'
        ) } | Should -Not -Throw
    }

    It 'should ignore the core marker on a combined versioned row' {
        { Assert-LibheifVersion -Packages @(
            'libheif[core,aom,libde265]:x64-windows 1.23.7#1 HEIF decoder'
        ) } | Should -Not -Throw
    }

    It 'should reject the GPL x265 hevc feature' {
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                       1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                              AVIF support',
            'libheif[libde265]:x64-windows                         libde265 HEVC decoder',
            'libheif[hevc]:x64-windows                             HEVC encoding via x265'
        ) } | Should -Throw '*GPL-2.0 x265*'
    }

    It 'should reject a separately installed x265 port' {
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                       1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                              AVIF support',
            'libheif[libde265]:x64-windows                         libde265 HEVC decoder'
        ) -X265Packages @('x265:x64-windows 4.1#2 x265 video encoder') } |
            Should -Throw '*installed the GPL-2.0 x265 port*'
    }

    It 'should reject a decode-only set that lost libde265' {
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                       1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                              AVIF support'
        ) } | Should -Throw "*expected exactly 'aom,libde265'*"
    }

    It 'should reject an unexpected codec feature' {
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                       1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                              AVIF support',
            'libheif[dav1d]:x64-windows                            AV1 decoder',
            'libheif[libde265]:x64-windows                         libde265 HEVC decoder'
        ) } | Should -Throw "*expected exactly 'aom,libde265'*"
    }

    It 'should reject a cache containing an old installed base port' {
        { Assert-LibheifVersion -Packages @('libheif:x64-windows 1.23.0 HEIF decoder') } |
            Should -Throw '*exactly one installed libheif 1.23.7*'
    }

    It 'should reject an empty query and a feature-only output' {
        { Assert-LibheifVersion -Packages @() } | Should -Throw '*exactly one*'
        { Assert-LibheifVersion -Packages @('libheif[aom]:x64-windows AVIF support') } |
            Should -Throw '*exactly one installed libheif 1.23.7*'
    }
}

Describe 'Installed Windows libheif artifacts' {
    BeforeEach {
        $script:vcpkg = Join-Path $TestDrive 'vcpkg'
        New-Item -ItemType Directory -Force -Path (Join-Path $vcpkg 'installed\x64-windows\bin') | Out-Null
        New-Item -ItemType Directory -Force -Path (Join-Path $vcpkg 'installed\x64-windows\lib') | Out-Null
        # A static-md tree coexists for boost/zlib and must not confuse the check.
        New-Item -ItemType Directory -Force -Path (Join-Path $vcpkg 'installed\x64-windows-static-md\lib') | Out-Null
        Set-Content -Path (Join-Path $vcpkg 'installed\x64-windows-static-md\lib\zlibstatic.lib') -Value 'zlib'
        foreach ($dll in @('heif.dll', 'libde265.dll', 'aom.dll')) {
            Set-Content -Path (Join-Path $vcpkg "installed\x64-windows\bin\$dll") -Value 'dll'
        }
        Set-Content -Path (Join-Path $vcpkg 'installed\x64-windows\lib\heif.lib') -Value 'import'
        foreach ($port in @('libheif', 'libde265', 'aom')) {
            New-Item -ItemType Directory -Force -Path (Join-Path $vcpkg "installed\x64-windows\share\$port") | Out-Null
            Set-Content -Path (Join-Path $vcpkg "installed\x64-windows\share\$port\copyright") -Value 'license'
        }
    }

    It 'should accept a complete dynamic tree beside the static boost/zlib tree' {
        { Assert-LibheifArtifacts -VcpkgRoot $vcpkg } | Should -Not -Throw
    }

    It 'should reject a missing runtime DLL' {
        Remove-Item -LiteralPath (Join-Path $vcpkg 'installed\x64-windows\bin\aom.dll')
        { Assert-LibheifArtifacts -VcpkgRoot $vcpkg } | Should -Throw '*aom.dll is missing*'
    }

    It 'should reject a missing import library' {
        Remove-Item -LiteralPath (Join-Path $vcpkg 'installed\x64-windows\lib\heif.lib')
        { Assert-LibheifArtifacts -VcpkgRoot $vcpkg } | Should -Throw '*heif.lib is missing*'
    }

    It 'should reject a missing codec copyright file' {
        Remove-Item -LiteralPath (Join-Path $vcpkg 'installed\x64-windows\share\libde265\copyright')
        { Assert-LibheifArtifacts -VcpkgRoot $vcpkg } | Should -Throw '*libde265*copyright is missing*'
    }

    It 'should reject an x265 DLL beside the dynamic codecs' {
        Set-Content -Path (Join-Path $vcpkg 'installed\x64-windows\bin\libx265.dll') -Value 'gpl'
        { Assert-LibheifArtifacts -VcpkgRoot $vcpkg } | Should -Throw '*GPL-2.0 x265 file*'
    }
}
