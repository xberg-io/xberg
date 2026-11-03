# Setup PaddleOCR Models Cache

This composite action downloads the PaddleOCR ONNX artifacts used by Xberg into the standard Hugging Face Hub cache. It does not create a second Xberg-owned copy.

The repository is pinned to commit `bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8`. Every requested artifact is SHA-256 verified after both warm-cache resolution and download.

## Usage

```yaml
- uses: ./.github/actions/setup-paddle-ocr-models
  id: paddle-models

- run: cargo test -p xberg --features paddle-ocr
  env:
    HF_HUB_CACHE: ${{ steps.paddle-models.outputs.cache-dir }}
```

To request a subset:

```yaml
- uses: ./.github/actions/setup-paddle-ocr-models
  with:
    models: "det,rec"
```

## Inputs

| Name | Default | Description |
| --- | --- | --- |
| `cache-enabled` | `true` | Cache the standard HF Hub directory with `actions/cache`. |
| `models` | `det,cls,rec` | Runtime model groups to resolve. |
| `cache-key-suffix` | `paddle-ocr-v5-onnx` | Prefix for the Actions cache key. |

## Runtime artifacts

| Group | Pinned repository path | Purpose |
| --- | --- | --- |
| `det` | `v2/det/server.onnx` | Server-tier text detection. |
| `cls` | `v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx` | Text-line orientation. |
| `rec` | `v2/rec/unified_server/model.onnx`, `dict.txt` | Unified server recognition and dictionary. |

## Outputs

| Name | Description |
| --- | --- |
| `cache-hit` | Whether Actions restored a matching HF cache. |
| `cache-dir` | The HF Hub cache root (`~/.cache/huggingface/hub` by default). |
| `models-available` | Comma-separated verified groups. |

The action exports `HF_HUB_CACHE` for downstream steps. Xberg also honors the normal Hugging Face conventions (`HF_HUB_CACHE`, `HF_HOME`, then the platform default). `XBERG_CACHE_DIR` is unrelated to model artifacts and is not set by this action.
