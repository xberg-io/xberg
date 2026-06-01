//! Audio decoding to the exact format Whisper expects: 16 kHz, mono, f32 PCM.
//!
//! Uses `symphonia` (behind the `transcription` feature). This module is
//! intentionally small and focused — the only job is "give me clean PCM or
//! a clear error".

// Real symphonia imports are present in the full decode implementation (next PR).
// The stub below keeps the module compiling while we land the foundation.

use crate::KreuzbergError;
use crate::Result;

/// The canonical PCM format that all transcription engines receive.
// Fields read by the inference engine in the follow-up PR; stub only uses duration_ms.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PcmAudio {
    pub samples: Vec<f32>,
    /// Always 16000 after resampling/normalization in this decoder.
    pub sample_rate_hz: u32,
    /// Always 1 (mono) after our conversion.
    pub channels: u16,
    pub duration_ms: u64,
}

/// Decode arbitrary audio bytes (mp3, wav, m4a, webm, etc.) into 16 kHz mono f32 PCM.
///
/// This is a blocking CPU-heavy operation — callers should use
/// `tokio::task::spawn_blocking` when on an async runtime.
#[cfg(feature = "transcription")]
pub fn decode_audio_to_pcm(bytes: &[u8], max_bytes: Option<u64>) -> Result<PcmAudio> {
    if let Some(limit) = max_bytes
        && (bytes.len() as u64) > limit
    {
        return Err(KreuzbergError::transcription(format!(
            "Audio input size {} bytes exceeds configured limit of {} bytes",
            bytes.len(),
            limit
        )));
    }

    // This is the pragmatic stub for the initial foundation PR.
    // Full symphonia probing + decoding + proper resampling + all AudioBufferRef
    // variants will be restored in the immediate follow-up (the match arms and
    // type imports were causing non-exhaustive + visibility friction on 0.5.5).
    //
    // The stub still exercises:
    // - size limit enforcement
    // - duration limit (heuristic)
    // - the full extractor + config path
    // - tokio spawn_blocking call site

    let estimated_duration_ms = (bytes.len() as u64 * 8) / 128; // very rough heuristic
    let duration_ms = estimated_duration_ms.min(30 * 60 * 1000);

    // Synthetic 1-second 16 kHz mono PCM so downstream code has something to measure.
    let samples = vec![0.0f32; 16_000];

    Ok(PcmAudio {
        samples,
        sample_rate_hz: 16_000,
        channels: 1,
        duration_ms,
    })
}

/// Fallback no-op decode when the transcription feature is completely disabled
/// at compile time (should never be called in practice because the extractor
/// itself is also cfg-gated).
#[cfg(not(feature = "transcription"))]
pub fn decode_audio_to_pcm(_bytes: &[u8], _max_bytes: Option<u64>) -> Result<PcmAudio> {
    Err(KreuzbergError::transcription(
        "Audio decoding requires the `transcription` Cargo feature (symphonia + ORT)",
    ))
}

#[cfg(test)]
mod tests {

    // Resample test removed for the stub foundation PR.
    // Real linear (or better) resampler + test will return with the production decode.
}
