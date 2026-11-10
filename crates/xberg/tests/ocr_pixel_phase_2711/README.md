# PDF OCR source-density controls

`fixture.rs` embeds the repository's DejaVu Sans and draws four known English lines at 7 pt,
with zero character and word spacing. The page is 432 × 216 pt: 900 × 450 pixels at 150 DPI
and 1800 × 900 at 300 DPI. The origin control moves the text by one 150-DPI pixel (0.48 pt).
The localized controls move one glyph or word boundary while retaining the source text.
The integration tests verify the source layer, bundled English model hash, source words,
render dimensions and explicit-image/default-route precedence.

`qualification-pins.json` lists all 13 public qualification cases, their PDF hashes,
independent source-text hashes and expected word counts, plus the font and model pins.
The additional cases cover complete/partial/missing CID maps, outlined and raster prose,
and ligature encodings. Quality qualification compares source-word occurrence multisets
before and after the change; confidence does not select a winner. Deliberately removing a
source word or adding an invented word must fail that comparison.

This is a bounded correction for forced whole-document OCR with a configured preprocessing
block, including its default 300-DPI target. No preprocessing block retains the 150-DPI
render route. These controls do not establish invariance for all fonts or resolve the
remaining default-density glyph-shift cases in #2711.

Run the separate paired qualification harness explicitly with a writable external report
folder in `XBERG_2711_QUALIFICATION` and an arm name in `XBERG_2711_ARM`. Its test is ignored
by ordinary runs because it requires that report destination. Select
`--test qualification_2711_public_corpus -- --ignored --nocapture --test-threads=1` with
`pdf,ocr,tokio-runtime,bundle-tessdata-eng` or `full,formula-recognition,bundle-tessdata-eng`.
`XBERG_2711_DENOISE=1` selects the separate denoise comparison. That comparison preserves the
narrow-gap case's existing “ina” error in both arms; it establishes no new source-word loss,
not recovery of that boundary under denoising.
