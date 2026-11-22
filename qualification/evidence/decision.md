## Qualification result (#2334)

### classification: `NoGo`

| check | threshold | observed | result |
| --- | --- | --- | --- |
| nli-minilm2-l6-h768 / int8 macro-F1 | 0.65 | 0.3214 | FAIL |
| nli-minilm2-l6-h768 / int8 macro-F1 CI lower bound | 0.5 | 0.2305 | FAIL |
| nli-minilm2-l6-h768 / int8 recall final rule | 0.45 | 0.0204 | FAIL |
| nli-minilm2-l6-h768 / int8 recall proposed rule | 0.45 | 0.4583 | pass |
| nli-minilm2-l6-h768 / int8 recall public notice | 0.45 | 0.2195 | FAIL |
| nli-minilm2-l6-h768 / int8 ECE | 0.2 | 0.2044 | FAIL |
| nli-minilm2-l6-h768 / int8 abstention on known classes | 0.4 | 0.7101 | FAIL |
| nli-minilm2-l6-h768 / int8 unknown-class rejection | 0.7 | 0.7917 | pass |

- nli-minilm2-l6-h768 / int8 macro-F1 observed 0.3214 (threshold 0.65)
- nli-minilm2-l6-h768 / int8 macro-F1 CI lower bound observed 0.2305 (threshold 0.5)
- nli-minilm2-l6-h768 / int8 recall final rule observed 0.0204 (threshold 0.45)
- nli-minilm2-l6-h768 / int8 recall public notice observed 0.2195 (threshold 0.45)
- nli-minilm2-l6-h768 / int8 ECE observed 0.2044 (threshold 0.2)
- nli-minilm2-l6-h768 / int8 abstention on known classes observed 0.7101 (threshold 0.4)

### captioning: `NoGo`

| check | threshold | observed | result |
| --- | --- | --- | --- |
| smolvlm-256m-instruct / fp32 reference-number recall | 0.2 | 0.0500 | FAIL |
| smolvlm-256m-instruct / fp32 reference-term recall | 0.35 | 0.2222 | FAIL |
| smolvlm-256m-instruct / fp32 images with an invented number | 0.6 | 0.1333 | pass |
| smolvlm-256m-instruct / int8 reference-number recall | 0.2 | 0.0000 | FAIL |
| smolvlm-256m-instruct / int8 reference-term recall | 0.35 | 0.2180 | FAIL |
| smolvlm-256m-instruct / int8 images with an invented number | 0.6 | 0.0333 | pass |
| smolvlm-500m-instruct / fp32 reference-number recall | 0.2 | 0.0000 | FAIL |
| smolvlm-500m-instruct / fp32 reference-term recall | 0.35 | 0.2255 | FAIL |
| smolvlm-500m-instruct / fp32 images with an invented number | 0.6 | 0.0667 | pass |
| smolvlm-500m-instruct / int8 reference-number recall | 0.2 | 0.0000 | FAIL |
| smolvlm-500m-instruct / int8 reference-term recall | 0.35 | 0.1971 | FAIL |
| smolvlm-500m-instruct / int8 images with an invented number | 0.6 | 0.0667 | pass |

- smolvlm-256m-instruct / fp32 reference-number recall observed 0.0500 (threshold 0.2)
- smolvlm-256m-instruct / fp32 reference-term recall observed 0.2222 (threshold 0.35)
- smolvlm-256m-instruct / int8 reference-number recall observed 0.0000 (threshold 0.2)
- smolvlm-256m-instruct / int8 reference-term recall observed 0.2180 (threshold 0.35)
- smolvlm-500m-instruct / fp32 reference-number recall observed 0.0000 (threshold 0.2)
- smolvlm-500m-instruct / fp32 reference-term recall observed 0.2255 (threshold 0.35)
- smolvlm-500m-instruct / int8 reference-number recall observed 0.0000 (threshold 0.2)
- smolvlm-500m-instruct / int8 reference-term recall observed 0.1971 (threshold 0.35)

`qualified: false`, `defaults_changed: false`: no default, preset, bundle or supported-model claim follows from this run.
