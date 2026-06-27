# Audio and Video Transcription

Turn audio and video into searchable, model-ready transcripts. Whisper-based speech-to-text with automatic language detection and multi-language support, automatic sample-rate and channel handling, chunking for files over 30 seconds, and configurable model sizes from tiny (10 MB) to large.

See the [TranscriptionConfig reference](../reference/configuration.md#transcriptionconfig) for all configuration options.

## Setup

Enable the `transcription` Cargo feature and set a `TranscriptionConfig` block in your `ExtractionConfig` to extract transcripts from audio and video files.

## Supported MIME types

| MIME type    | Extensions     | Container                    |
|--------------|----------------|------------------------------|
| `audio/mpeg` | `.mp3`, `.mpga` | MP3                          |
| `audio/mp4`  | `.m4a`         | M4A / AAC in MP4             |
| `audio/wav`  | `.wav`         | WAV / RIFF                   |
| `audio/webm` | `.webm`        | WebM audio                   |
| `video/mp4`  | `.mp4`, `.mpeg` | MP4 video (audio track only) |
| `video/webm` | `.webm`        | WebM video (audio track only) |

## Model sizes

| Variant   | Cache footprint | RAM at inference | Mel bins |
|-----------|-----------------|------------------|----------|
| `Tiny`    | Smallest        | Lowest memory    | 80       |
| `Base`    | Small           | Low memory       | 80       |
| `Small`   | Medium          | Medium memory    | 80       |
| `Medium`  | Large           | High memory      | 80       |
| `LargeV3` | Largest         | Highest memory   | 128      |

Models are downloaded from HuggingFace Hub on first use — `Tiny`, `Base`, and
`Small` from `onnx-community/whisper-{size}`, and `Medium` and `LargeV3` from
`Xenova/whisper-{size}` — and cached under `{XBERG_CACHE_DIR}/whisper/{size}/` when
`XBERG_CACHE_DIR` is set, or under the platform cache directory such as
`~/.cache/xberg/whisper/{size}/` on Linux.

## Configuration knobs

| Field              | Type              | Default  | Description |
|--------------------|-------------------|----------|-------------|
| `enabled`          | `bool`            | `true`   | The extractor activates only when the `transcription` block is present and `enabled` is true. |
| `model`            | `WhisperModel`    | `Tiny`   | Size variant to use. |
| `language`         | `Option<String>`  | `None`   | ISO-639-1 code (e.g. `"en"`, `"de"`). The current engine falls back to English when unset; set this explicitly for deterministic output. |
| `timestamps`       | `bool`            | `false`  | Accepted for forward-compatibility; segment timestamps are not yet emitted. |
| `max_bytes`        | `Option<u64>`     | `512 MiB` | Reject input larger than this many bytes before decoding. |
| `max_duration_ms`  | `Option<u64>`     | `30 min` | Reject audio longer than this many milliseconds after decode. |
| `timeout_ms`       | `Option<u64>`     | `10 min` | Reserved wall-clock timeout for the full inference call. The current extractor does not enforce it yet. |
| `model_cache_dir`  | `Option<PathBuf>` | `None`   | Override the default cache location. |
| `allow_network`    | `bool`            | `true`   | Set to `false` to disable automatic downloads; returns `ModelMissing` if the model is not already cached. |
| `verify_hash`      | `bool`            | `true`   | Hash verification is reserved for a future work item; currently a no-op with a warning. |

## First-run download

On the first call with `allow_network = true`, the extractor downloads the
required ONNX files and tokenizer from HuggingFace Hub. The download is
serialised per process via a cross-process advisory file lock so concurrent
first-time callers do not race. Subsequent calls use the local cache.

Set `allow_network = false` and pre-populate the cache directory if you need
air-gapped deployments. When the model is absent and `allow_network = false`,
extraction returns a `XbergError::Transcription` with the message
`"network access disabled and model not cached"`.

## Usage

Add the feature to `Cargo.toml`:

```toml
xberg = { version = "5", features = ["transcription"] }
```

```rust
use xberg::core::config::transcription::{TranscriptionConfig, WhisperModel};
use xberg::{extract, ExtractInput, ExtractionConfig};

let config = ExtractionConfig {
    transcription: Some(TranscriptionConfig {
        enabled: true,
        model: WhisperModel::Tiny,
        language: Some("en".to_string()),
        ..Default::default()
    }),
    ..Default::default()
};

let bytes = std::fs::read("recording.wav")?;
let output = extract(
    ExtractInput::from_bytes(bytes, "audio/wav", Some("recording.wav".to_string())),
    &config,
).await?;
println!("{}", output.results[0].content); // transcript
```

## Notes

- Audio longer than 30 seconds is split into 30-second chunks; each chunk is
  transcribed independently and the results are joined with a space.
- The extractor always resamples to 16 kHz mono before inference; source sample
  rate and channel layout are handled automatically.
- Engine instances are cached per process keyed by model paths, so the ONNX
  sessions are loaded once and reused across calls.
- Async inference calls are bounded by a semaphore sized to
  `resolve_thread_budget`, matching the same limit used by the embedding and
  reranking pipelines.
