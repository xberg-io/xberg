# Real-artifact negative controls

The synthetic suite (`report.json`, 14 mutations) proves the validators fire on a fixture. These
six controls mutate the **actual** frozen artifacts of this run and record what the real code did.
Each was executed against the same binary (`0755ab07…`) and the committed plan digests
(`4a71679b…` classification) as the reported run, and re-executed after the final build.

| control | mutation | observed result |
| --- | --- | --- |
| corrupted artifact | flipped one byte at offset 41,411,532 of the staged 82,823,065-byte `model_qint8_arm64.onnx` (in a copy of the HF cache, plan built with `XBERG_QUALIFICATION_CACHE` at that copy) | `prepare nli-minilm2-l6-h768 / int8: Validation error: Local NLI artifact checksum mismatch: nli-minilm2-l6-h768/model_qint8_arm64.onnx`; no report written |
| missing prediction | deleted the last row of `classification/report.json` | `ONNX qualification: inference row count mismatch` |
| skipped inference | set `rows[0].inference = "skipped"` | `ONNX qualification: inference must complete with positive finite measured duration` |
| unknown backend | set `rows[0].model.effective_backend = "provider"` | `ONNX qualification: parse report: unknown variant \`provider\`, expected \`onnx_cpu\`` |
| wrong expected count | set `classification.expected.total = 820` in a copy of the preregistration | `ONNX qualification: expected count does not equal its factors` |
| empty corpus | emptied `documents` and reset the digest to the empty-array digest | `ONNX qualification: corpus must not be empty` |

Sequence actually executed (abridged):

```sh
B=onnx-qualification
PLAN=qualification/evidence/classification/plan.json
SHA=$(cat qualification/evidence/classification/plan.sha256)

# corrupted artifact
XBERG_QUALIFICATION_CACHE=/tmp/corrupt-cache $B plan-classification ... /tmp/plan-corrupt.json
XBERG_QUALIFICATION_CACHE=/tmp/corrupt-cache $B run-classification /tmp/plan-corrupt.json "$plan_sha" /tmp/report-corrupt.json
# -> Local NLI artifact checksum mismatch; /tmp/report-corrupt.json absent

$B score-classification "$PLAN" "$SHA" /tmp/report-missing.json ...   # -> row count mismatch
$B score-classification "$PLAN" "$SHA" /tmp/report-skipped.json ...   # -> inference must complete
$B score-classification "$PLAN" "$SHA" /tmp/report-backend.json ...   # -> unknown variant `provider`
$B score-classification "$PLAN" "$SHA" report.json /tmp/prereg-wrong-count.json ...  # -> expected count
$B audit /tmp/manifest-empty.json                                     # -> corpus must not be empty
```

One control initially "passed" for the wrong reason and is recorded here rather than hidden: copying
the HF cache with `cp -R` preserved symlinks into the xet shard store, so the copied graphs resolved
to nothing and the refusal was `missing from the pre-staged cache`. Copying with `cp -RL` produced
real files, and the corrupted byte then produced the checksum refusal above.

No control produced a report file. A control that produces a report is a failed control.
