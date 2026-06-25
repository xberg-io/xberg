//! Audio decoding to the exact format Whisper expects: 16 kHz, mono, f32 PCM.
//!
//! Uses `symphonia` (behind the `transcription` feature). This module is
//! intentionally small and focused — the only job is "give me clean PCM or
//! a clear error".

use crate::Result;
use crate::XbergError;

/// The canonical PCM format that all transcription engines receive.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct PcmAudio {
    /// Interleaved (or mono) f32 PCM samples in [-1.0, 1.0].
    pub samples: Vec<f32>,
    /// Always 16000 after resampling/normalization in this decoder.
    pub sample_rate_hz: u32,
    /// Always 1 (mono) after our conversion.
    pub channels: u16,
    /// Duration of the decoded audio in milliseconds.
    pub duration_ms: u64,
}

/// Decode arbitrary audio bytes (mp3, wav, m4a, webm, etc.) into 16 kHz mono f32 PCM.
///
/// This is a blocking CPU-heavy operation — callers should use
/// `tokio::task::spawn_blocking` when on an async runtime.
#[cfg(feature = "transcription")]
#[cfg_attr(alef, alef(skip))]
pub fn decode_audio_to_pcm(bytes: &[u8], max_bytes: Option<u64>) -> Result<PcmAudio> {
    use std::io::Cursor;
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::formats::TrackType;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    if let Some(limit) = max_bytes
        && (bytes.len() as u64) > limit
    {
        return Err(XbergError::transcription(format!(
            "Audio input size {} bytes exceeds configured limit of {} bytes",
            bytes.len(),
            limit
        )));
    }

    // Wrap bytes in a Cursor — io::Cursor<&[u8]> implements MediaSource.
    let cursor = Cursor::new(bytes);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    // No extension hint — let symphonia detect format from container magic bytes.
    let hint = Hint::new();
    let fmt_opts: FormatOptions = Default::default();
    let meta_opts: MetadataOptions = Default::default();

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(|e| XbergError::transcription(format!("symphonia probe failed: {e}")))?;

    // Select the first decodable audio track.
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| XbergError::transcription("no audio track found in input"))?;

    let track_id = track.id;

    // Extract codec parameters before the mutable borrow of `format` below.
    let audio_codec_params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .cloned()
        .ok_or_else(|| XbergError::transcription("audio track has no decodable codec parameters"))?;

    let src_sample_rate = audio_codec_params.sample_rate.unwrap_or(44_100);
    let src_channels = audio_codec_params.channels.as_ref().map(|c| c.count()).unwrap_or(1);

    let dec_opts: AudioDecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio_codec_params, &dec_opts)
        .map_err(|e| XbergError::transcription(format!("unsupported audio codec: {e}")))?;

    // Decode all packets, collecting interleaved f32 samples.
    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(pkt)) => pkt,
            Ok(None) => break, // end of stream
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                // Chained OGG streams: reset and continue.
                decoder.reset();
                continue;
            }
            Err(e) => {
                return Err(XbergError::transcription(format!("error reading audio packet: {e}")));
            }
        };

        if packet.track_id != track_id {
            continue;
        }

        let audio_buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            // Soft errors: skip the packet.
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => {
                return Err(XbergError::transcription(format!("audio decode error: {e}")));
            }
        };

        let frame_count = audio_buf.frames();
        if frame_count == 0 {
            continue;
        }

        let total_samples = audio_buf.samples_interleaved();
        let mut chunk = vec![0.0f32; total_samples];
        audio_buf.copy_to_slice_interleaved(chunk.as_mut_slice());
        interleaved.extend_from_slice(&chunk);
    }

    // Down-mix to mono if the source has more than one channel.
    let mono = if src_channels <= 1 {
        interleaved
    } else {
        down_mix_to_mono(&interleaved, src_channels)
    };

    // Resample to 16 kHz.
    let samples = if src_sample_rate == 16_000 {
        mono
    } else {
        resample_linear_to_16k(&mono, src_sample_rate)
    };

    let duration_ms = samples.len() as u64 * 1000 / 16_000;

    Ok(PcmAudio {
        samples,
        sample_rate_hz: 16_000,
        channels: 1,
        duration_ms,
    })
}

/// Average `channels` interleaved planes down to mono.
#[cfg(feature = "transcription")]
fn down_mix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels == 0 {
        return Vec::new();
    }
    let frames = interleaved.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    let inv = 1.0_f32 / channels as f32;
    for frame in 0..frames {
        let mut sum = 0.0_f32;
        for ch in 0..channels {
            sum += interleaved[frame * channels + ch];
        }
        mono.push(sum * inv);
    }
    mono
}

/// Resample mono f32 PCM from `src_hz` to 16 000 Hz using linear interpolation.
///
/// Linear interpolation is sufficient for the Whisper pipeline at v1. If accuracy
/// issues emerge for very low-bitrate sources (e.g. 8 kHz telephone audio), the
/// caller can swap in a higher-quality resampler at the W2 inference layer.
#[cfg(feature = "transcription")]
fn resample_linear_to_16k(samples: &[f32], src_hz: u32) -> Vec<f32> {
    const TARGET_HZ: u32 = 16_000;

    if samples.is_empty() || src_hz == 0 {
        return Vec::new();
    }
    if src_hz == TARGET_HZ {
        return samples.to_vec();
    }

    let src_len = samples.len();
    // Number of output frames: ceil(src_len * TARGET_HZ / src_hz).
    let out_len = (src_len as u64 * TARGET_HZ as u64).div_ceil(src_hz as u64) as usize;

    let mut out = Vec::with_capacity(out_len);
    let ratio = src_hz as f64 / TARGET_HZ as f64;

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let lo = src_pos as usize;
        let hi = (lo + 1).min(src_len - 1);
        let frac = (src_pos - lo as f64) as f32;
        out.push(samples[lo] + (samples[hi] - samples[lo]) * frac);
    }

    out
}

/// Fallback no-op decode when the transcription feature is completely disabled
/// at compile time (should never be called in practice because the extractor
/// itself is also cfg-gated).
#[cfg(not(feature = "transcription"))]
#[cfg_attr(alef, alef(skip))]
pub fn decode_audio_to_pcm(_bytes: &[u8], _max_bytes: Option<u64>) -> Result<PcmAudio> {
    Err(XbergError::transcription(
        "Audio decoding requires the `transcription` Cargo feature (symphonia + ORT)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "transcription")]
    #[test]
    fn test_size_limit_enforced() {
        let result = decode_audio_to_pcm(&[0u8; 20], Some(10));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("exceed") || msg.contains("limit"), "unexpected: {msg}");
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn test_empty_bytes_returns_error() {
        // Empty input has no audio track — should return an error.
        let result = decode_audio_to_pcm(&[], None);
        assert!(result.is_err());
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn test_down_mix_to_mono_stereo() {
        // L=1.0, R=-1.0 → mono=0.0
        let stereo = vec![1.0f32, -1.0, 0.5, 0.5];
        let mono = down_mix_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0]).abs() < 1e-6);
        assert!((mono[1] - 0.5).abs() < 1e-6);
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn test_resample_linear_passthrough_when_same_rate() {
        let samples = vec![0.1f32, 0.2, 0.3];
        let out = resample_linear_to_16k(&samples, 16_000);
        assert_eq!(out, samples);
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn test_resample_linear_halves_rate() {
        // Source: 32 kHz, 2 samples → should produce 1 output sample at 16 kHz.
        let samples = vec![0.0f32, 1.0];
        let out = resample_linear_to_16k(&samples, 32_000);
        assert_eq!(out.len(), 1);
    }
}
