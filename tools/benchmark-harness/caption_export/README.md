# Local caption graph exporter

Export a caller-selected, already verified local ViT/GPT-2 source. This tool does
not fetch a model or remote Python, install dependencies, publish an artifact, or
select a default model. The existing frozen Python environment must be prepared
before running it.

```bash
mise run caption:export -- export-local \
  --source /absolute/path/to/snapshot \
  --inventory /absolute/path/to/provenance/verified-inventory.json \
  --repository nlpconnect/vit-gpt2-image-captioning \
  --revision dc68f91c06a1ba6f15268e5b9c13ae7a7c514084 \
  --output /absolute/path/to/new-caption-bundle

mise run caption:export -- validate /absolute/path/to/new-caption-bundle
```

The input receipt must come from an independently verified source acquisition.
It contains `source_repository`, the full lowercase 40-character
`source_revision`, and a nonempty `source_files` array with `file`, `bytes` and
full lowercase `sha256` for every flat source file. Both explicit CLI identity
arguments must match it. Verification streams every file before loading weights;
missing, additional, corrupt, duplicate, symbolic-link or remote-code assets
fail. One `pytorch_model.bin` or `model.safetensors` is supported. Pickle-backed
weights load with `weights_only=True`; loading is local-only with remote code
disabled. This command does not independently authenticate the supplied receipt
against Hugging Face.

The destination must not exist and must be outside the source directory. Failures
can leave a new incomplete destination, which is never accepted as a completed
bundle; choose a fresh destination for retry. Existing destinations are untouched.

The fresh CPU FP32 eager graphs use opset 17, batch one, fixed image and encoder
context geometry, and dynamic token length only. Both actual decoder configs and
forward calls disable cache. Greedy generation metadata describes this new
full-prefix interface. Source files stay unchanged. The six output companions
retain the tokenizer vocabulary and resolve BOS/EOS/PAD names from the existing
configuration IDs. Scalar ViT sizes become explicit height/width; omitted ViT
`do_rescale=True` and `rescale_factor=1/255` defaults become explicit. Other
processor settings must already satisfy the native contract.

`model/` contains the six companions, two graphs and their `sha256.json` manifest.
The outer bundle preserves the source receipt, available README/license/NOTICE
files, package versions, producer code/recipe hashes and output hashes.
Validation checks source-receipt and notice consistency and the existing strict
graph/metadata contract, verifying graph integrity before graph loading.

Only seeded tiny checkpoints qualify the CLI tests. The selected public model
still requires real export and numerical parity, SDK end-to-end checks, artifact
and license review, and publication approval. Its Apache-2.0 model-card metadata
does not substitute for an independently reviewed redistribution package; no
caption-quality or default-inventory claim follows from export validation.


## Decoder first-divergence diagnostic

The separate diagnostic observes existing exported tensors. It does not re-export
weights, change tolerances, qualify a failed bundle or select additional data.
Freeze a plan before running it, using the project environment and this checkout's
`src` on `PYTHONPATH`:

```bash
python -m caption_export.first_divergence_plan \
  --original /absolute/path/to/frozen-original-parity-directory \
  --plan /absolute/path/to/new-diagnostic-plan.json

python -m caption_export.first_divergence_run \
  --plan /absolute/path/to/new-diagnostic-plan.json \
  --plan-sha256 FULL_SHA_PRINTED_BY_FREEZE \
  --output /absolute/path/to/new-diagnostic-output
```

The plan pins the actual checkout's complete Python package and the original
`plan.json`, `parity.py` and failed `result.json`. The runner reuses that pinned
probe's exact source/checkpoint, package-version, installed-source, loading and
24 legacy mask-buffer checks before loading weights. It verifies those inputs
again on completion. An imported package from another checkout fails. Output
must be new and outside the immutable source, bundle, receipts and package.
Failures retain an explicitly incomplete receipt; retry in a new directory.

This protocol is fixed to the existing zero-image development input and token
prefixes T=1 and T=2. Both sessions use CPU, sequential execution, one thread and
`ORT_DISABLE_ALL`; the original ALL-optimization qualification remains unchanged.
The original decoder must reproduce T1 passing and T2 failing at `atol=1e-5`,
`rtol=1e-4`. The fifteen taps are embedding output, twelve block outputs, final
normalization and logits. Norm scopes must be unique; a new protobuf exposes
existing values without changing graph nodes or initializers. Thirty comparisons
must contain exactly 183,027 finite FP32 elements. First literal-byte divergence
and first tolerance failure are reported separately in architecture order.

Two controls require instrumented and original ORT logits to be bit-identical;
two more require default HF and the export wrapper's explicit mask to agree
bit-for-bit. Poisoned-value and missing-stage controls must fail for their exact
intended reasons. The receipt preserves the original six failed prefixes and
`qualified=false`, even if all diagnostic comparisons agree. Successful command
execution means this bounded diagnostic completed, not numerical qualification,
caption quality, performance, SDK readiness or permission to publish weights.
