# verify-windows-dll-closure.ps1
#
# Windows analogue of scripts/ci/verify-glibc-floor.sh: proves a Windows native
# artifact (wheel, PHP/Elixir/C-FFI archive, or CLI zip) is loadable on a clean
# Windows host by inspecting the PE import table of every native library it
# ships, rather than trusting that a build succeeded.
#
# It checks two independent things, both required to fix xberg-io/xberg#1456:
#   1. ABSENCE: no shipped .pyd/.dll/.exe statically imports $ForbiddenDll
#      (default DirectML.dll). A hard PE import is resolved by the Windows
#      loader before a single instruction of the module runs, so an unshipped
#      import here is fatal at `import xberg` / DllMain, not a runtime error
#      you can catch.
#   2. PRESENCE: $RequiredDll (default onnxruntime.dll) exists somewhere under
#      the extracted artifact tree, because moving off the pyke static-link
#      strategy (which baked ORT in) onto system dynamic linking makes that a
#      real runtime dependency that must be vendored.
#
# Usage:
#   verify-windows-dll-closure.ps1 <artifact> <NativeGlob> [RequiredDll] [ForbiddenDll]
#
#   <artifact>     Path to a .whl, .zip, .tar.gz, or an already-extracted directory.
#   <NativeGlob>   Filename glob identifying the native library to inspect,
#                  e.g. "*.pyd", "php_xberg.dll", "xberg_ffi.dll", "xberg.exe".
#
# Exit 0 only if every matched native file passes both checks. Exit 1 with a
# specific reason otherwise.

param(
  [Parameter(Mandatory = $true)][string]$Artifact,
  [Parameter(Mandatory = $true)][string]$NativeGlob,
  [string]$RequiredDll = "onnxruntime.dll",
  [string]$ForbiddenDll = "DirectML.dll"
)

$ErrorActionPreference = "Stop"

function Write-Log([string]$Message) {
  Write-Host "verify-windows-dll-closure: $Message"
}

function Get-PeImportedDllNames([string]$Path) {
  # Minimal COFF/PE import-directory parser. Returns the literal DLL name
  # strings the file's import table references (case as stored, usually
  # mixed-case as written by the linker that produced the .lib).
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $stream = New-Object System.IO.MemoryStream(, $bytes)
  $br = New-Object System.IO.BinaryReader($stream)
  try {
    $stream.Seek(0x3C, [System.IO.SeekOrigin]::Begin) | Out-Null
    $peOffset = $br.ReadInt32()
    $stream.Seek($peOffset, [System.IO.SeekOrigin]::Begin) | Out-Null
    $sig = $br.ReadUInt32()
    if ($sig -ne 0x00004550) { throw "not a PE file (bad signature) at '$Path'" }

    $machine = $br.ReadUInt16()
    $numberOfSections = $br.ReadUInt16()
    $stream.Seek(12, [System.IO.SeekOrigin]::Current) | Out-Null # TimeDateStamp, PointerToSymbolTable, NumberOfSymbols
    $sizeOfOptionalHeader = $br.ReadUInt16()
    $stream.Seek(2, [System.IO.SeekOrigin]::Current) | Out-Null # Characteristics
    $optionalHeaderStart = $stream.Position

    $magic = $br.ReadUInt16()
    $isPe32Plus = ($magic -eq 0x20B)
    # DataDirectory[0] starts at offset 0x60 (PE32, IMAGE_OPTIONAL_HEADER32) or
    # 0x70 (PE32+, IMAGE_OPTIONAL_HEADER64) from the optional header start.
    # The +16 comes from SizeOfStackReserve/StackCommit/HeapReserve/HeapCommit,
    # which are 4 bytes each in PE32 and 8 each in PE32+ (4 fields x +4 = +16).
    # ImageBase widening (4 -> 8) and the dropped 4-byte BaseOfData field cancel
    # each other out and contribute nothing to the shift -- do not "correct" this
    # back, the two effects are genuinely independent.
    # NumberOfRvaAndSizes is the 4-byte field immediately before DataDirectory[0].
    $dataDirectoryStart = $optionalHeaderStart + $(if ($isPe32Plus) { 0x70 } else { 0x60 })
    $stream.Position = $dataDirectoryStart - 4
    $numberOfRvaAndSizes = $br.ReadUInt32()
    if ($numberOfRvaAndSizes -lt 2) { return @() } # no import directory at all

    # DataDirectory[1] = Import Table; each entry is 8 bytes (VirtualAddress, Size).
    $stream.Position = $dataDirectoryStart + 8 # entry index 1
    $importTableRva = $br.ReadUInt32()
    $importTableSize = $br.ReadUInt32()
    if ($importTableRva -eq 0 -or $importTableSize -eq 0) { return @() }

    $sectionHeadersStart = $optionalHeaderStart + $sizeOfOptionalHeader
    $sections = @()
    $stream.Position = $sectionHeadersStart
    for ($i = 0; $i -lt $numberOfSections; $i++) {
      $nameBytes = $br.ReadBytes(8)
      $virtualSize = $br.ReadUInt32()
      $virtualAddress = $br.ReadUInt32()
      $stream.Seek(4, [System.IO.SeekOrigin]::Current) | Out-Null # SizeOfRawData
      $pointerToRawData = $br.ReadUInt32()
      $stream.Seek(16, [System.IO.SeekOrigin]::Current) | Out-Null # remaining fields to next 40-byte header
      $sections += [PSCustomObject]@{
        VirtualAddress   = $virtualAddress
        VirtualSize      = $virtualSize
        PointerToRawData = $pointerToRawData
      }
    }

    function Rva2Offset([uint32]$Rva) {
      foreach ($s in $sections) {
        if ($Rva -ge $s.VirtualAddress -and $Rva -lt ($s.VirtualAddress + [Math]::Max($s.VirtualSize, 1))) {
          return [int64]($Rva - $s.VirtualAddress + $s.PointerToRawData)
        }
      }
      throw "RVA 0x$($Rva.ToString('X')) not found in any section of '$Path'"
    }

    function ReadAsciiZ([int64]$Offset) {
      $stream.Position = $Offset
      $sb = New-Object System.Text.StringBuilder
      while ($true) {
        $b = $br.ReadByte()
        if ($b -eq 0) { break }
        [void]$sb.Append([char]$b)
      }
      return $sb.ToString()
    }

    $importDirOffset = Rva2Offset $importTableRva
    $names = @()
    $descriptorOffset = $importDirOffset
    while ($true) {
      $stream.Position = $descriptorOffset
      $originalFirstThunk = $br.ReadUInt32()
      $stream.Seek(8, [System.IO.SeekOrigin]::Current) | Out-Null # TimeDateStamp, ForwarderChain
      $nameRva = $br.ReadUInt32()
      $stream.Seek(4, [System.IO.SeekOrigin]::Current) | Out-Null # FirstThunk
      # An all-zero 20-byte descriptor terminates the array.
      if ($originalFirstThunk -eq 0 -and $nameRva -eq 0) { break }
      if ($nameRva -ne 0) {
        $names += ReadAsciiZ (Rva2Offset $nameRva)
      }
      $descriptorOffset += 20
    }
    return $names
  }
  finally {
    $br.Dispose()
    $stream.Dispose()
  }
}

function Expand-Artifact([string]$ArtifactPath, [string]$Dest) {
  New-Item -ItemType Directory -Path $Dest -Force | Out-Null
  switch -Regex ($ArtifactPath) {
    '\.(whl|zip)$' {
      Expand-Archive -Path $ArtifactPath -DestinationPath $Dest -Force
    }
    '\.tar\.gz$|\.tgz$' {
      & tar -xzf $ArtifactPath -C $Dest
      if ($LASTEXITCODE -ne 0) { throw "tar extraction failed for '$ArtifactPath'" }
    }
    default {
      throw "unsupported artifact type: '$ArtifactPath'"
    }
  }
}

if ($NativeGlob -like '*.pyd') {
  & python (Join-Path $PSScriptRoot 'windows_wheel_closure.py') audit --artifact $Artifact --native-glob $NativeGlob --required $RequiredDll --forbidden $ForbiddenDll
  exit $LASTEXITCODE
}

$root = $Artifact
$workDir = $null
if (Test-Path -PathType Container $Artifact) {
  $root = (Resolve-Path $Artifact).Path
}
else {
  $workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("verify-windows-dll-closure-" + [Guid]::NewGuid().ToString("N"))
  Expand-Artifact -ArtifactPath $Artifact -Dest $workDir
  $root = $workDir
}

try {
  $natives = @(Get-ChildItem -Path $root -Recurse -File -Filter $NativeGlob -ErrorAction SilentlyContinue)
  if ($natives.Count -eq 0) {
    Write-Error "no native file matching '$NativeGlob' found under '$root' (artifact: '$Artifact')"
    exit 1
  }

  $failures = @()
  foreach ($native in $natives) {
    Write-Log "inspecting $($native.FullName)"
    $imports = Get-PeImportedDllNames $native.FullName
    $forbidden = $imports | Where-Object { $_ -ieq $ForbiddenDll }
    if ($forbidden) {
      $failures += "$($native.Name) imports $ForbiddenDll, which is never shipped -- the Windows loader will refuse to load this module (xberg-io/xberg#1456)"
      continue
    }

    $requiredPresent = @(Get-ChildItem -Path $native.Directory.FullName -Filter $RequiredDll -File -ErrorAction SilentlyContinue).Count -gt 0
    if (-not $requiredPresent) {
      $failures += "$($native.Name) is dynamically linked but $RequiredDll is not shipped alongside it in $($native.Directory.FullName)"
    }
    else {
      Write-Log "OK $($native.Name) (no $ForbiddenDll import, $RequiredDll present)"
    }
  }

  if ($failures.Count -gt 0) {
    foreach ($f in $failures) { Write-Error $f }
    exit 1
  }
  Write-Log "artifact passes the Windows DLL closure gate"
  exit 0
}
finally {
  if ($workDir -and (Test-Path $workDir)) { Remove-Item -Recurse -Force $workDir }
}
