---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Map large blocks directly in the macOS CLI

## Context and Problem Statement

macOS libmalloc parks freed large blocks in its large cache for reuse, and their
pages stay resident and charged to the process. OCR frees a stream of page
rasters, decode buffers and Leptonica images whose sizes rarely fit a later
request, so the cache grows instead of being reused. On Apple Silicon (macOS
27.2) `pdf/scanned.pdf` peaked at a 176 MB footprint with 54 MB live, and
vmmap showed 93 MB of "Malloc Large (empty)". Running the 13 OCR cohort
documents with `MallocLargeCache=0` in the environment cut peak RSS by 30-50%
(fast-b4 248/325/204/420 MB to 129/154/136/292 MB) with byte-identical content
and unchanged wall time. How should the CLI get that behaviour without asking
users to set an environment variable?

## Decision Drivers

* OCR peak RSS on macOS must not include freed memory.
* Output and latency must not change.
* The CLI must not depend on how it was launched.
* Native PDF runs, which allocate few large blocks, must not regress.

## Considered Options

* Set `MallocLargeCache=0` from inside the binary.
* Call `malloc_zone_pressure_relief` after each page or batch.
* Link mimalloc as the global allocator.
* A global allocator that maps Rust allocations of 1 MiB or more with
  `mmap`/`munmap`, plus a Leptonica pixel memory manager with the same policy.

## Decision Outcome

Chosen option: "A global allocator that maps large blocks, plus a Leptonica
pixel memory manager", because it is the only option measured to release the
memory from inside the process.

`crates/xberg-cli/src/large_blocks.rs` defines `MappedLargeBlocks`, the macOS
`#[global_allocator]` of the CLI. A layout of at least 1 MiB with an alignment
of at most 4096 bytes is mapped with an anonymous private `mmap` and released
with `munmap`; every other layout goes to `System`. The size class depends on
the layout alone, so `dealloc` and `realloc` always release a block the way it
was allocated, and `realloc` across the threshold moves the block between the
two. The first statement of `main` calls Leptonica's `setPixMemoryManager` with
a pair of functions that prefix each pixel buffer with its size and map buffers
of 1 MiB or more, before any Pix can exist.

The same policy, applied to the symbolized profiling binary through an
interposer, gave identical content and unchanged wall time, and peak RSS fell
from 295 to 173 MB (`non_searchable`), 544 to 388 MB (`ocr_test`), 849 to
564 MB (`docling_scanned`) and 384 to 188 MB (`nougat_010_scanned`). Native PDFs
allocate 0-18 blocks of 1 MiB or more per document, and their latency stayed
within noise.

### Consequences

* Good, because freed large blocks leave the footprint as soon as they are freed.
* Good, because output is unchanged: only where memory comes from changes.
* Bad, because a mapped block that grows is copied instead of extended in place.
* Bad, because the language bindings keep libmalloc's behaviour: each binding
  library would need its own allocator and pixel memory manager, and a host
  process such as Python may already own the Leptonica hook.
* Bad, because Tesseract's own C++ allocations still go through libmalloc; they
  account for the remaining gap to `MallocLargeCache=0` (about 25-85 MB on the
  structured cohort).
* Neutral, because Linux is unchanged. glibc serves requests above
  `M_MMAP_THRESHOLD` (128 KiB at start) with `mmap` and unmaps them on free, but
  each such free raises the threshold to the freed size, up to 32 MiB on 64-bit
  (`DEFAULT_MMAP_THRESHOLD_MAX`), so later rasters come from the arenas and
  stay resident until a trim at the top of an arena. Unlike macOS, the process
  can fix this itself: `mallopt(M_MMAP_THRESHOLD, 1 MiB)` at the start of `main`
  turns the dynamic adjustment off. Whether that is warranted needs a Linux
  measurement of the OCR cohorts, which this record did not make.

### Confirmation

The unit tests in `large_blocks.rs` pin the size class (zero-size, below and at
the threshold, alignment above 4096 bytes falling through to `System`), that a
block is released by the allocator its layout selects (`malloc_zone_from_ptr`
owns exactly the unmapped ones), that `realloc` moves a block across the
threshold in both directions and keeps its bytes, that a mapped block is zeroed,
and that the pixel functions round-trip on both sides of the threshold. To
measure, run an OCR cohort document through the CLI under `/usr/bin/time -l`
and compare its peak RSS with the same binary under `MallocLargeCache=0`; the
two should stay within the Tesseract share above.

## Pros and Cons of the Options

### Set `MallocLargeCache=0` from inside the binary

* Good, because it covers every allocation, Tesseract's included.
* Bad, because libmalloc reads it while libSystem initializes, before any code
  of the program runs. A test program that calls `setenv("MallocLargeCache",
  "0")` from a priority-101 constructor still held 250 MB after freeing 16 large
  blocks; the same program launched with the variable set held 1 MB.
* Bad, because the only in-process way to apply it is re-executing the binary,
  which adds about 5 ms to every run, native PDFs included.

### `malloc_zone_pressure_relief`

* Good, because it is a public call.
* Bad, because it returned 0 and released nothing, both for the default zone and
  for every registered zone, in the same test program and in the CLI.

### mimalloc

* Good, because it is a drop-in global allocator already used on musl.
* Bad, because it raised native PDF RSS by 2-11 MB in the benchmark campaign,
  and it does not cover Leptonica or Tesseract.

### Mapped large blocks plus a Leptonica pixel memory manager

* Good, because both halves can be installed from `main`.
* Bad, because it needs `unsafe` code for the allocator and the C callbacks.

## More Information

* [MADR](https://adr.github.io/madr/)
* glibc `malloc/malloc.c`: `M_MMAP_THRESHOLD`, `DEFAULT_MMAP_THRESHOLD_MAX`,
  and the threshold update in `free`.
* Leptonica `pix1.c`: `setPixMemoryManager`, `pixdata_malloc`, `pixdata_free`.
