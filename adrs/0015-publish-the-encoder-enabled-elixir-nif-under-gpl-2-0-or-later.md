---
status: accepted
date: 2026-11-20
decision-makers: ["Na'aman Hirschfeld"]
---

# Publish the encoder-enabled Elixir NIF under GPL-2.0-or-later

## Context and Problem Statement

The macOS arm64 Elixir NIF is built with the `elixir-encoders` profile, which links x265 4.3 (GPL-2.0-or-later) into libheif so the NIF can write HEIC/AVIF output. The archive already ships x265's `COPYING` and a pinned `sources.tsv`, but it is presented as MIT while linking a GPL-2.0 library. The decode-only CLI and Docker artifacts must not carry x265 at all (#3338), so the Elixir encoder build cannot share their closure.

## Considered Options

* Publish the encoder-enabled NIF archive under GPL-2.0-or-later, shipping x265's license text and source inventory with the artifact.
* Build x265 as a libheif plugin, load it only when the caller enables encoding, and keep the NIF MIT while publishing the plugin as GPL-2.0-or-later.
* Remove HEIC/HEIF output from the Elixir NIF.

## Decision Outcome

Chosen option: "Publish the encoder-enabled NIF archive under GPL-2.0-or-later, shipping x265's license text and source inventory with the artifact", because HEIF output is a deliberate feature of that artifact, the archive already carries x265's `COPYING` and pinned source provenance, and shipping the plugin would still distribute GPL code while adding a runtime configuration surface and a second artifact to document.

The NIF's Rust and Elixir sources remain available under MIT, which is GPL-compatible; recipients of the encoder archive receive the combined work under GPL-2.0-or-later. Decode-only platforms and the CLI, Docker and PHP artifacts do not include x265 and stay MIT.

### Consequences

* Good, because the artifact's stated license matches the code it links, and the decode-only artifacts stay free of x265.
* Good, because no plugin-loading protocol, runtime option or second published artifact is needed.
* Good, because the existing pinned source inventory and `COPYING` already satisfy GPL-2.0's source and notice distribution.
* Bad, because consumers of the encoder-enabled NIF must accept GPL-2.0-or-later for that artifact.
* Bad, because the macOS Elixir archive differs in licensing from the decode-only NIFs, so the notices must state the per-artifact difference.
* Neutral, because the `elixir-encoders` build profile and the `missing-x265` smoke control stay unchanged.

### Confirmation

`THIRD_PARTY_LICENSES.md` states that the encoder-enabled Elixir NIF archive is distributed under GPL-2.0-or-later and names x265's source. `scripts/ci/build-macos-heif-deps.sh` (profile `elixir-encoders`) pins x265 4.3 and stages its `COPYING` plus `sources.tsv`, and `scripts/ci/macos_elixir_archive.py` refuses an archive that omits either. The `elixir-natives` job in `.github/workflows/publish.yaml` fails if the archive loads without `libx265`, which keeps the encoder linkage deliberate rather than accidental. The decode-only artifacts are enforced by `scripts/ci/verify_no_gpl_codecs.py`.
