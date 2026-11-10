# PDF OCR source-density controls

`fixture.rs` embeds the repository's DejaVu Sans and draws four known English lines at 7 pt,
with zero character and word spacing. The page is 432 × 216 pt: 900 × 450 pixels at 150 DPI
and 1800 × 900 at 300 DPI. The origin control moves the text by one 150-DPI pixel (0.48 pt).
The localized controls move one glyph or word boundary while retaining the source text.
The integration tests verify the source layer, bundled English model hash, source words,
render dimensions and explicit-image/default-route precedence.

`qualification-pins.json` pins all 13 PDF fixtures, independent source texts, font and
English model, and the SHA-256 of both retained baselines and candidate recordings.
`baseline-default.json` and `baseline-denoise.json` retain actual OCR reads with the four
OCR production files from `a33f39373e`; the native PDF dependency stayed at `e3830c5b5f` in
both arms. The `recorded-*.json` candidates are actual `e3830c5b5f` reads. This isolates the
OCR correction rather than claiming a separately built whole-main baseline.

The committed comparator checks the complete, unique case set and fixture/source/model
identity, then compares source-word occurrence multisets against the pinned baseline for
the selected mode. It rejects lost correct occurrences and increased false occurrences,
including duplicated words already present in the source. Case folding and edge punctuation
normalization match the retained comparison; confidence and word order do not select a
winner. A failed comparison exits the command with failure and does not publish a checked
report. Thirteen nonempty rows alone are not a quality pass.

From the repository root, run the ordinary controls with the documented feature set:

```sh
mise exec -- cargo test --offline --locked -j1 -p xberg --no-default-features \
  --features pdf,ocr,tokio-runtime,bundle-tessdata-eng \
  --test qualification_2711_public_corpus -- --nocapture --test-threads=1
```

These controls launch the actual public qualification command for both retained recordings
and for deletion, repeated-word loss, invented-word insertion, excess source-word insertion,
and nonempty wrong-fixture-hash reports. Each negative must fail for its specific comparison
reason. Replay invokes the same checked-report pipeline as live OCR, without running OCR:

```sh
report_dir=$(mktemp -d)
XBERG_2711_QUALIFICATION="$report_dir" XBERG_2711_ARM=checked \
XBERG_2711_REPLAY="$PWD/crates/xberg/tests/ocr_pixel_phase_2711/recorded-default.json" \
  mise exec -- cargo test --offline --locked -j1 -p xberg --no-default-features \
  --features pdf,ocr,tokio-runtime,bundle-tessdata-eng \
  --test qualification_2711_public_corpus paired_public_source_word_qualification \
  -- --exact --ignored --nocapture --test-threads=1
```

For denoise replay, set `XBERG_2711_DENOISE=1` and use `recorded-denoise.json`. To qualify
fresh OCR, omit `XBERG_2711_REPLAY`: the command reads all 13 pinned inputs and applies the
same comparator before writing its report. The full feature alternative is
`--features full,formula-recognition,bundle-tessdata-eng`. The report-producing entry point
is explicitly ignored in ordinary runs because it requires an external output directory;
the comparison controls above run normally. Existing setup must provide the pinned tools
and bundled model; use an isolated Cargo target when peers are active.

This remains a bounded correction for forced whole-document OCR with a configured
preprocessing block, including its default 300 DPI target. Without a block, rendering
retains the 150 DPI default. Denoise retains the narrow-gap case's existing “ina” error in
both arms; its comparison proves no new source-word loss, not recovery of that boundary.
These controls do not prove invariance for all fonts or resolve the remaining cases in #2711.
