---
status: accepted
date: 2026-11-21
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Link the Windows libheif decoders dynamically to meet LGPL-3.0

## Context and Problem Statement

The Windows x64 CLI ZIP links libheif and libde265, both LGPL-3.0, through
vcpkg. It used the `x64-windows-static-md` triplet, so the codec code was
embedded in `xberg.exe`. LGPL-3.0 section 4(d)(0) then requires relinkable
application objects or the Minimal Corresponding Source plus the application
code, and the archive carried neither, only `LICENSE` and a descriptive
`THIRD_PARTY_LICENSES.md` (#3339). Decode-only linking (#3338) removed the GPL
x265 encoder but not the LGPL obligation. The library must be conveyed as a
replaceable shared object, or the archive must ship the section 4(d)(0) material.

## Decision Drivers

* The released ZIP must satisfy LGPL-3.0 section 4 for libheif and libde265.
* The release gate must prove it from the shipped bytes, not from the build
  script's intent, without a Windows runner in the verification loop.
* libheif-sys probes vcpkg through vcpkg-rs, whose Windows default is the static
  `x64-windows-static-md` triplet; no out-of-tree patch to that crate is allowed.
* Keep the no-GPL archive free of every codec and keep HEIC/HEIF decoding.

## Considered Options

* Link the codecs from vcpkg's dynamic `x64-windows` triplet, bundle the DLLs
  and the LGPL-3.0 and GPL-3.0 texts, and gate the archive on its PE imports and
  notices (the section 4(d)(1) route).
* Keep the static link and ship relinkable objects or a written corresponding
  source offer plus both texts and the exception in `THIRD_PARTY_LICENSES.md`
  (the section 4(d)(0) route).
* Drop HEIC/HEIF from the Windows CLI entirely.

## Decision Outcome

Chosen option: "Link the codecs from vcpkg's dynamic `x64-windows` triplet,
bundle the DLLs and the LGPL-3.0 and GPL-3.0 texts, and gate the archive on its
PE imports and notices", because it needs no external change: libheif-sys
already drives vcpkg-rs, which honours `VCPKGRS_TRIPLET=x64-windows` and
`VCPKGRS_DYNAMIC=1`, so `heif.lib` (the import library) is linked and the DLLs
travel as separate files. The section 4(d)(0) route would have required
publishing relinkable objects for a 40-minute MSVC fat-LTO build, or a written
offer whose acceptance is harder to verify, for a ZIP whose codec linkage was
the only obstacle.

The Windows vcpkg install is `libheif[core,aom,libde265]:x64-windows` and stays
decode-only: `core` suppresses the port's default `x265`/`hevc` encoder. The
DLLs, `LGPL-3.0.txt`, `GPL-3.0.txt` and the upstream per-port `copyright` files
are staged in the ZIP, and `THIRD_PARTY_LICENSES.md` records the method. The
`-no-gpl-lgpl` variant still compiles `heic` out and carries no codec.

### Consequences

* Good, because the archive needs no relink material or written offer, and its
  DLLs remain replaceable, satisfying section 4(d)(1)(b).
* Good, because every Windows cargo invocation keeps the static
  `x64-windows-static-md` default except the CLI build step, so no other crate's
  libz-sys probe or xberg-libwpd link changes.
* Bad, because the codec set now depends on a second vcpkg triplet, doubling the
  vcpkg build for Windows (one cache entry) and adding a DLL closure to the ZIP.
* Bad, because a library that shipped as one file now ships as a directory of
  files, so distributors must preserve the DLLs beside `xberg.exe`.

### Confirmation

`scripts/ci/verify_windows_heif_archive.py` parses the shipped `xberg.exe` import
table (requiring `heif.dll`), walks the codec DLL closure, and requires
`LGPL-3.0.txt`, `GPL-3.0.txt`, `libheif-COPYING`, `libde265-COPYING`,
`aom-COPYING` and a `THIRD_PARTY_LICENSES.md` that names the DLL and Windows. Its
negative controls live in `scripts/ci/test_verify_windows_heif_archive.py`
(static link, missing/placeholder text, missing DLL, incomplete closure), and
`mise run release:test:contracts` runs them. The release job runs the gate on
every default Windows CLI ZIP; the no-GPL ZIP keeps its own
`verify_no_gpl_cli_archive.py` audit. `scripts/ci/install-system-deps/install-windows.ps1`
refuses a cached tree without `heif.dll`, `libde265.dll` or the three `copyright`
files.

## Pros and Cons of the Options

### Dynamic triplet with bundled DLLs and notices

* Good, because no change to libheif-sys or vcpkg-rs is needed.
* Good, because the archive is verifiable by PE imports and file contents.
* Bad, because the ZIP carries more files and a second vcpkg triplet.

### Static link with relinkable objects or a written offer

* Good, because the shipped binary stays a single file.
* Bad, because relinkable MSVC objects for a fat-LTO build must be published and
  kept reproducible, or an offer must be maintained and honoured, neither of
  which the repository had any mechanism for.

### Drop HEIC/HEIF from the default Windows CLI

* Good, because it removes the obligation entirely.
* Bad, because the default Windows archive would lose a documented capability
  that the permissive-licence argument no longer forces.

## More Information

* [MADR](https://adr.github.io/madr/)
* xberg-io/xberg#3339, #3294, #3338.
* GNU LGPL-3.0 sections 4(a)–4(e), shipped as `scripts/ci/notices/LGPL-3.0.txt`.
