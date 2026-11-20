# OCR regression verification for #3143

Run each named test at baseline, apply its mutation in an isolated review worktree, then rerun the same test. Expect
one collected test and an assertion failure in each mutated run. Restore the baseline between runs. The tests use
ordinary OCR reads and generated images.

For library tests, use the OCR features from `scripts/ci/rust/run-explicit-sideways-tests.sh`, with
`auto-rotate-tract` for the table retry module. Use the final test name as `FILTER` and check the collected full
name with `-- --list` before running it.

```sh
RUST_MIN_STACK=16777216 cargo test --locked -p xberg --no-default-features \
  --features pdf,pdf-native,ocr,ocr-pipeline,tokio-runtime,bundle-tessdata-eng,language-detection,html,image-encode,auto-rotate-tract \
  --lib FILTER
RUST_MIN_STACK=16777216 cargo test --locked -p xberg --no-default-features \
  --features pdf,ocr,tokio-runtime,bundle-tessdata-eng,layout-types --test TARGET FILTER
```

For the cached orientation integration, use `mise run rust:test:explicit-sideways native` and
`mise run rust:test:explicit-sideways warm` with the pinned models required by the task.

## Added or strengthened tests

| Clause | Test | Semantic mutation and expected failure |
| --- | --- | --- |
| #2989 dominated-page gate | `ocr_direct_route_symbol_noise_2921::a_flagged_direct_read_keeps_isolated_noise_when_prose_dominates` | In the direct prune loop in `pdf/ocr/pipeline.rs`, drop the `page_is_dominated_by_noise_lines` condition. The flagged prose page loses its last line and emits a prune warning. |
| #2989 noise-verdict gate | `ocr_direct_route_symbol_noise_2921::a_symbol_heavy_direct_read_keeps_its_lines_without_a_noise_verdict` | In the same loop, replace `for page in &noise_verdict_pages` with `for page in &accepted_replacements.keys().copied().collect::<Vec<_>>()`. The unflagged page loses its symbol lines. |
| #3031 Markdown noise band | `ocr_photo_symbol_noise_2312::photograph_pdf_keeps_at_most_a_quarter_of_its_symbol_noise` | At the integration test's read boundary, substitute `let content = if plain { content } else { TITLE.to_owned() };` after extraction to model a Markdown read that drops all surviving noise. The Markdown lower bound fails. |
| #3031 two-thirds threshold | `a_word_with_initials_needs_two_thirds_plausible_tokens` | In `symbol_noise_lines.rs`, replace `plausible * 3 >= tokens.len() * 2` with `plausible * 2 >= tokens.len()`. The four-token line is incorrectly kept. |
| #3135 blank-text warning swap | `discarded_retry_warning_claims_native_recovery_only_when_ocr_text_is_blank` | In `merge_returned_page_failures`, remove `&& page_text.trim().is_empty()`. The nonempty read incorrectly changes to the native-retention warning. |
| #3136 mode after two attempts | `a_retained_first_read_does_not_publish_its_mode_after_a_second_attempt` | In `rendering.rs`, temporarily seed the mode unconditionally by replacing `(outcome.attempted == 1)` in the collection assignment with `true`. Confirm the test stays green with that control, then replace the publication `if outcome.attempted == 1` with `if true`. The test observes `Some(6)`. Either guard alone suppresses publication. |
| #3136 published preprocessing bytes | `publishing_preprocessing_uses_the_remaining_output_budget` | In `recover_image_xobjects`, remove the `account_xobject_preprocessing` call using `context.budget.growth` at publication. The one-byte-short read incorrectly succeeds. |
| #3137 leader left edge | `should_not_let_a_read_leader_that_starts_inside_the_previous_word_close_a_gap` | In `native_word_coverage.rs`, replace `if part.left >= reach - slack` with `if true`. The overhanging leader incorrectly closes the gap. |
| #3137 leader right-edge equality | `should_measure_a_read_leader_that_starts_after_the_previous_word_by_how_far_past_the_next_word_it_ends` | In the same function, change `leader_right > part.left + slack` to `>=`. The exact-slack leader incorrectly leaves a gap. |
| #3140 unshrunk verdict | `a_noisy_unshrunk_read_survives_the_per_image_verdict_when_noise_is_kept` | In `judge_xobject_read`, set `possibly_illegible` to `true`. The per-image verdict drops the text, table, and formula. |
| #2983 TIFF target boundary | `tiff_upscale_clamps_one_byte_below_the_target_workspace` | In `encoded_image.rs::can_shrink_upscale`, remove `ImageFormat::Tiff` from the PNG/TIFF arm. The below-peak read is refused instead of clamping to 299 DPI. |
| #2983 binarized target boundary | `binarized_png_upscale_clamps_one_byte_below_the_target_workspace` | In `budgeted_upscale`, add `|| policy.binarization_method == "otsu"` to the early return condition. The below-peak read is refused instead of clamping to 299 DPI. |
| #2983 JPEG target boundary | `jpeg_refuses_the_requested_upscale_one_byte_below_the_target_workspace` | In `known_native_fits`, change `peak <= self.limits.max_content_size as u64` to `<`. The exact-fit JPEG is refused. |
| #2983 refusal remedy | `budgeted_gray_png_upscale_tracks_the_workspace_budget_boundary` and `budgeted_colour_png_uses_the_colour_workspace_boundary` | In `encoded_image.rs::native_error`, remove the `Raise security_limits.max_content_size` sentence. Each source-below-peak case reports a mismatch. Run each filter separately. |
| #2949 cached-context pruning | `ocr_explicit_sideways_2496::cached_engine_bytes_recover_when_osd_is_installed_without_changing_the_configuration` | In `cacheable_orientation_ocr`, set `quality_thresholds` to `None`. The unrotated read loses lines and no longer matches the original word count and digest. |
| #2977 prose pass factor | `the_prose_work_budget_alone_refuses_the_mixed_two_table_read` | In `two_tables.rs`, change `PROSE_WORK_PASSES` from 3 to 2. The independently expected cost is 220 work units. |
| #2977 touching tables | `table_boxes_must_have_positive_vertical_separation` | In `canonical_table_boxes`, change `boxes[0].y1 >= boxes[1].y0` to `>`. Touching boxes are incorrectly accepted. |
| #2977 configured retry limits | `implicit_retry_uses_the_callers_two_table_work_limit` | In `accepts_implicit`, pass `&default_limits` to `two_tables::complete_text` in place of `limits`. The tight-limit run returns the retry instead of the primary read. |

## Existing coverage to rerun

- #3118. `final_failed_page_keeps_its_warning_and_marks_the_run_cut_short` already checks that the two successful
  pages receive no unread records. Force the stop branch to supply an error when `failure` is absent and verify that
  only page 3 can be reported. `unread_pages_are_recorded_as_failures_with_the_time_limit_error` checks the recovered
  callback. Set `recovered: false` inside `mark_pages_unread` to check that assertion. The skipped-retry call-site
  argument alone is currently masked by the preceding failure record's recovered bit and the merge's OR.
- #3065/#3148. `ocr_page_failures::discarded_noise_2908::discarded_noisy_all_pages_pipeline_retry_reports_native_text_kept`
  already asserts the recovered native page and exact warning. In `pdf/mod.rs::report_restored_native_page_failures`,
  change `failure.recovered = true` to `false` to check the restored-page assertion.

## Deferred clauses

- #2989. Structured hOCR element pruning, paragraph pruning, retained-read synchronization on a Type0 page, and
  the relationships guard.
- #3106. Matched-layer pipeline wiring and per-clause controls.
- #3135. Mixed-route call sites passing the retained OCR text into the warning helper.
- #3136. Candidate and early metadata accounting, member/page fragmentation parity, numeric member filtering,
  failure-cause deduplication, and member trimming. The independent publication-guard arm requires the control above.
- #2983. Post-upscale guard and the eight independent search-clause controls.
