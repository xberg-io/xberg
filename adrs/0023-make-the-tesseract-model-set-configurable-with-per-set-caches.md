---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Make the Tesseract model set configurable with per-set caches

## Context and Problem Statement

Tesseract publishes two model sets for the same 126 languages, both
Apache-2.0: `tessdata_fast` (integer LSTM, 362 MB in all) and `tessdata_best`
(float LSTM, 1.15 GB in all, the largest file 15.1 MiB). xberg downloaded only
`tessdata_fast`, into a flat `<cache>/tessdata`, and searched system tessdata
directories, which hold `tessdata_fast` too. `best` may read some documents
better at a higher cost, and which one should be the default is not settled.
How should xberg offer both without one request mixing sets, without results
depending on what happens to be cached or installed, and with the default
left to a measurement?

## Decision Drivers

* A request for one set is never served the other set's files.
* Every download is pinned and hash-verified.
* Existing caches and installs keep working for `fast` without a re-download.
* Changing the default later is a one-line change on native targets and does
  not move WebAssembly or mobile.
* OCR results cached for one set are never returned for the other.

## Considered Options

* A model-set option with a cache directory per set.
* A model-set option sharing the flat cache, keyed by file name suffix.
* No option: users point `tessdata_path` or `TESSDATA_PREFIX` at `best` data.

## Decision Outcome

Chosen option: "A model-set option with a cache directory per set", because
directory identity alone then keeps the sets apart in every resolver, probe
and cache key that already works per directory.

`ExtractionConfig.tessdata_model` (`"fast"` or `"best"`, lowercase everywhere)
selects the set for every OCR an extraction performs, automatic and fallback
OCR included, and never enables OCR: creating an `ocr` section would change
whether OCR runs. `OcrConfig.tessdata_model` overrides it for one section;
`None` inherits, and a per-file `ocr` section that names none keeps the base
section's set. `XBERG_OCR_TESSDATA_MODEL` (read by every CLI command that
extracts: `extract`, `batch`, `serve`, `mcp` and `doctor`; for `extract` and
`batch` it is the only `XBERG_*` override applied) and `--ocr-tessdata-model`
set the top-level field, with the flag above the variable above the config
file, so an image that sets the variable runs every command with that set. The name
sits beside `tessdata_path` and `tessdata_bytes`, and the values are the
upstream repository suffixes. An unset set is not serialized.

Every fallback to a default `OcrConfig` goes through
`ExtractionConfig::ocr_or_default`, which fills in the effective set, and
`extract_file`/`extract_bytes` copy the inherited set into an `ocr` section
that names none, so code that only receives the `OcrConfig` sees it. A
process-wide default was rejected: it did not reach every fallback site and
made results depend on what else ran in the process.

On a server the set is operator-owned: a REST or MCP request may restate what
the operator's config holds in either field (its effective set where it holds
none) but is rejected when it names another, and both fields are restored from
the operator's config after a merge. A request that drops the `ocr` section
keeps the operator's section set on the top level. A per-input config has no
top-level set: one that differs is rejected with a message saying the
request's set applies, and a restated one is ignored. Unlike PaddleOCR's `model_tier`, which callers
may still choose, a set change makes the server download and keep a second copy
of every language (1.15 GB for all of `best`); whether Paddle model selection
should become operator-owned too is a follow-up.

Resolution takes the first directory holding every requested language:

1. `tessdata_path`, then `TESSDATA_PREFIX`, used as given for either set. A
   `best` request served from one of them logs a warning once per directory,
   and doctor warns when `TESSDATA_PREFIX`, or a `tessdata_path` naming a
   system package directory, serves `best`.
2. For `fast`: the flat `<cache>/tessdata` older releases filled, then
   `<cache>/tessdata/fast`, then the system directories. For `best`:
   `<cache>/tessdata/best`, then, on Unix, the read-only
   `/usr/share/xberg/tessdata/best` an image can ship. The flat cache and the
   system directories hold `tessdata_fast`, so they never satisfy `best`.
3. Otherwise the missing packs are filled into one directory: for `fast`, the
   flat cache when it already holds some of the requested languages (so a pack
   placed there by hand keeps serving), else `<cache>/tessdata/fast`; for
   `best`, `<cache>/tessdata/best`. A pack is copied from a local directory
   when the bytes read from it match the set's SHA-256 pin (the flat cache; for
   `best` also the `fast` cache, the built-in `best` directory and the system
   paths), written to a temporary file and published without replacing
   anything; for `fast`, `eng` can come from the bundled model; the rest is
   downloaded from one pinned revision per set (`tessdata_fast` 8741641,
   `tessdata_best` e12c65a). The pins are generated by
   `scripts/generate_tessdata_pins.py`.

`osd` and `equ` are byte-identical in both sets. Each set keeps its own copy,
and a managed set directory adopts them from verified local copies on every
resolution where they are missing (retried when a source file changes), so
orientation detection still finds `osd` beside the primary models; it never
downloads them for that. The script retries (Han, Khmer, vertical Japanese)
read only the requested set's directories, so with `best` they run only when
those models are in a `best` directory. `frk` is fetched from `deu_latf` in
both sets: upstream keeps it only as a symlink in `tessdata_fast` and dropped
it from `tessdata_best`. The language capability probe never downloads: with
`best` it answers from files already present.

A non-fast set is hashed into the OCR result cache key next to the resolved
directory, so `fast` keys are unchanged; the set is also part of the
language-probe memo key. The default is the `#[default]` variant of
`TessdataModel`, `Fast`, mirrored by the `DEFAULT_TESSDATA_MODEL` constant;
WebAssembly asserts at compile time that the constant is `Fast`. The
WebAssembly backend bundles only `tessdata_fast` English (a bundled `best`
English model would put the module at about 54.7 MB, over the 51,380,224-byte
publish limit and jsDelivr's 52,428,800-byte cap) and refuses `best` unless the
caller supplies the bytes in `tessdata_bytes`; its tessdata cache is keyed by
set and language.

Whether native builds default to `best` is decided by an OCR quality and
latency measurement, not by this record.

### Consequences

* Good, because a `best` request reads only xberg-verified `best` files, the
  read-only `best` directory or the directory the caller named, so its output
  does not depend on cache history. A caller-named directory is used as given
  even when it holds `tessdata_fast`, with a warning.
* Good, because a cache filled before model sets existed resolves `fast`
  exactly as before, and its OCR result cache entries stay valid.
* Good, because `xberg cache warm --tessdata-model` and
  `--tessdata-languages` prefetch either set for offline hosts.
* Bad, because `best` cannot use the system packages and downloads 3-4 times
  as many bytes per language on first use.
* Bad, because new `fast` packs land in `tessdata/fast` while the flat
  directory stays, so an upgraded cache can hold a pack twice.
* Bad, because the set lives in two fields, and code building a fresh
  `OcrConfig` from defaults must take it from `ocr_or_default` to keep it.
* Bad, because routes whose thresholds were certified on `tessdata_fast`
  output (`pdf_gap::eligible_tesseract`,
  `symbol_noise_paint::recognition_matches`, the caption-grid and paint
  certifiers) run with `best` unmeasured. The pending quality A/B that decides
  the default covers them; no route is gated on the set until then.

### Confirmation

Unit tests cover config, environment and CLI parsing (including the flag and
the variable leaving `ocr` unset), the set reaching the main PDF OCR config,
the ink probe and the Tesseract engine config with and without an `ocr`
section, per-file inheritance, the operator check, the per-set search order
with injected system directories
(`test_best_search_dirs_hold_no_fast_only_directory`,
`test_best_request_never_resolves_to_a_fast_only_directory`), override and
`TESSDATA_PREFIX` precedence, a hand-placed flat-cache pack still serving
`fast`, verified adoption through injected pins and a downloader that fails
the test if called (including shared packs arriving after a first attempt and a
`fast` request split between the flat and `fast` caches), the offline `best` probe, the
doctor warning, the unchanged `fast` cache key, both pin tables, and the
WebAssembly refusal of `best` without caller bytes. Downloads themselves are
covered against local test servers, not against GitHub. The pin generator
reproduces the committed `tessdata_fast` table byte for byte and checks every
download against the Git blob size and SHA-1 of the pinned tree.

To switch native builds to `best`: move `#[default]` to `Best`, change
`DEFAULT_TESSDATA_MODEL`, regenerate the bindings, and update the field
documentation and the OCR guide. WebAssembly, Android and iOS then need an
explicit `fast` default (the wasm assert fails until they have one). Decide
whether `scripts/lib/tessdata.sh` (which exports `TESSDATA_PREFIX` to
`tessdata_fast` data), `ensure-tesseract-windows.ps1` and the Docker images
provision `best` packs, since system packages then stop counting; re-check
tests that rely on the flat cache through the default set; and leave
`bundle-tessdata-eng` and the notices unchanged.

## Pros and Cons of the Options

### A model-set option with a cache directory per set

* Good, because directory identity alone separates the sets, so every
  existing resolver, probe and fingerprint keeps working per directory.
* Bad, because the default set needs a migration path from the flat cache.

### A model-set option sharing the flat cache

* Good, because no new directories.
* Bad, because Tesseract loads `<lang>.traineddata` by name, so a suffix
  scheme needs a renamed copy or a per-request staging directory.

### No option

* Good, because nothing changes.
* Bad, because `best` then needs hand-provisioned, unverified files, and a
  partially provisioned directory silently falls back to `fast` packs
  elsewhere in the chain.

## More Information

* [tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast) and
  [tessdata_best](https://github.com/tesseract-ocr/tessdata_best), Apache-2.0.
* [MADR](https://adr.github.io/madr/)
