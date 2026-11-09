# Type0 mixed-page retention controls

`negative.rs` reuses the tracked `outlined_prose_2313.pdf` fixture. Its five
readable sentences are authored outline content, not an OCR snapshot. The test
pins the fixture and font hashes, adds different plausible native sentences and
eight unmapped Type0 glyphs, covers that layer with opaque white, and draws the
original page afterward. The resulting page must render pixel-identically to the
original outline source while native extraction still holds the covered words.

Both document orders run Plain and Markdown through native, automatic, and real
forced Tesseract extraction. Forced OCR must recover each independently specified
visible sentence once and none of the covered sentences. Automatic extraction
must select that successful read, preserve the untouched sibling, and report no
native-retention warning for the bad page.

Run the two negative tests with a caller-owned target and one build job:

```sh
cargo test --locked -p xberg --no-default-features \
  --features pdf,ocr,bundle-tessdata-eng,tokio-runtime \
  --test pdf_type0_scope_2533 negative::should_choose_visible_outline_words \
  -- --test-threads=1 --nocapture
```

To verify sensitivity locally, temporarily return `true` at entry to
`native_text_paints_all_read_ink` in `ocr/native_word_coverage.rs`, leaving
candidate selection and final retention gates unchanged. Both negatives must
fail: `successful read must replace bad marker layer` expects `Mixed` but gets
`Native`; visible sentence counts expect one but get zero; covered sentence
counts expect zero but get one; the forbidden retention warning count becomes
one. The readable-word and pixel prerequisites must still pass. Restore the exact
production file before rerunning the target; never commit the mutation.

The target's three NIST/forced-selection tests also require the hash-pinned public
NIST page. Focused PDF CI checks out the corpus manifest, fetches only
`pdf/regression_pages/nist-sp811-2008-p7.pdf`, verifies SHA-256, and explicitly sets
`XBERG_NIST_FIXTURE`. It lists and executes all five target cases separately with
`--exact`, requiring one collected and one passed case each. A missing fixture or
excluded feature configuration is a failure, never a skip. The PDF suite requires
ten exact cases in total, including these five.
