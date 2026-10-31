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
