---
status: accepted
date: 2026-11-21
decision-makers: ["Na'aman Hirschfeld"]
---

# Publish the encoder-enabled Elixir NIF under GPL-3.0-or-later with a written source offer

## Context and Problem Statement

The macOS arm64 Elixir NIF is built with the `elixir-encoders` profile, which links libheif and libde265 (LGPL-3.0-or-later) and libaom (BSD-2-Clause) against x265 4.3 (GPL-2.0-or-later) so the NIF can write HEIC/AVIF output. Combining GPL-2.0-or-later with LGPL-3.0-or-later means the conveyed combined work is GPL-3.0-or-later, not the MIT license the archive was labelled with. The archive's `sources.tsv` and upstream `COPYING` files are notices, not Corresponding Source, and GPL-3.0 section 6 requires conveying the complete corresponding source (including the build scripts) or a written offer to do so. The decode-only CLI and Docker artifacts must not carry x265 at all (#3338), so the Elixir encoder build cannot share their closure.

## Considered Options

* Convey the encoder-enabled NIF archive under GPL-3.0-or-later and ship a written offer of Corresponding Source, valid for three years, alongside the pinned source notices.
* Build x265 as a libheif plugin, load it only when the caller enables encoding, and keep the NIF MIT while publishing the plugin as GPL-3.0-or-later.
* Remove HEIC/HEIF output from the Elixir NIF.

## Decision Outcome

Chosen option: "Convey the encoder-enabled NIF archive under GPL-3.0-or-later and ship a written offer of Corresponding Source, valid for three years, alongside the pinned source notices", because HEIF output is a deliberate feature of that artifact, the offer is self-contained in the archive (so the release gate can require and validate it), and the pinned upstream URLs and SHA-256 digests in `sources.tsv` already give direct access to the four codec releases. Shipping the upstream tarballs as a seventh release asset would also require extending the six-asset Elixir publication contract; the written offer covers the one macOS encoder archive without changing that contract.

`GPL-SOURCE-OFFER.txt` ships in the archive. It states the GPL-3.0-or-later conveyance, offers the complete Corresponding Source (the four codec releases at the versions in `sources.tsv`, `scripts/ci/build-macos-heif-deps.sh` from the release tag, interface files and build/install scripts) on a durable medium for no more than the cost of distribution under section 6(b), is valid for three years from distribution, and names `contact@xberg.io` as the way to obtain it. Because the offer also names the pinned LGPL components, it satisfies LGPL-3.0 section 4(d). The NIF's Rust and Elixir sources remain available under MIT, which is GPL-compatible.

The offer is sourced from the workflow revision (`.release-tools/GPL-SOURCE-OFFER.txt`) rather than the possibly older release checkout, so a tagged republish ships the current, complete offer.

### Consequences

* Good, because the artifact's stated license matches the combined work it conveys, and the source obligation has a concrete, three-year mechanism.
* Good, because the archive carries and validates its own offer, so it cannot ship without one.
* Good, because no plugin-loading protocol, runtime option or release-asset contract change is needed.
* Bad, because consumers of the encoder-enabled NIF must accept GPL-3.0-or-later for that artifact.
* Bad, because the macOS Elixir archive differs in licensing from the decode-only NIFs, so the notices must state the per-artifact difference.
* Neutral, because the `elixir-encoders` build profile and the `missing-x265` smoke control stay unchanged.

### Confirmation

`GPL-SOURCE-OFFER.txt` contains the required license, scope, three-year term, section 6(b) reference and `contact@xberg.io` contact; `scripts/ci/macos_elixir_archive.py` (`SOURCE_OFFER_PHRASES`, `read_project_notices`) refuses to stage or verify an archive whose offer is absent or missing any of them, and `scripts/ci/test_macos_elixir_archive.py` exercises each missing-phrase case. `THIRD_PARTY_LICENSES.md` states the GPL-3.0-or-later conveyance and the offer. `.github/workflows/publish.yaml` passes the workflow-revision offer to both the stage and verify invocations, and the `elixir-natives` job fails if the archive loads without `libx265`. The decode-only artifacts are enforced by `scripts/ci/verify_no_gpl_codecs.py`.
