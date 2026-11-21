BeforeAll {
    $path = Join-Path $PSScriptRoot '../../ci/install-system-deps/install-windows.ps1'
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
    if ($errors.Count -ne 0) { throw 'Windows installer did not parse' }
    $functions = @($ast.FindAll({ param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -in @('Retry-Command', 'Assert-LibheifVersion')
    }, $true))
    if ($functions.Count -ne 2) { throw 'Expected both actual dependency validation functions' }
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
    It 'should accept one versioned core row and version-less feature rows' {
        # Real `vcpkg list libheif` shape: the core row carries the version, each
        # enabled feature is a separate row with no version column.
        { Assert-LibheifVersion -Packages @(
            'libheif:x64-windows                                1.23.7#1   HEIF decoder',
            'libheif[aom]:x64-windows                                       AVIF support',
            'libheif[libde265]:x64-windows                                  libde265 HEVC decoder'
        ) } | Should -Not -Throw
    }

    It 'should reject a feature row travelling on the core row instead of on its own' {
        { Assert-LibheifVersion -Packages @(
            'libheif[core,aom,libde265]:x64-windows 1.23.7#1 HEIF decoder'
        ) } | Should -Throw '*exactly one installed libheif 1.23.7*'
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
