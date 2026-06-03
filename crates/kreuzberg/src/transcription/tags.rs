//! Audio tag extraction via lofty.
//!
//! Reads ID3v1/v2 (MP3), MP4 atoms (AAC/M4A), Vorbis comments (OGG/FLAC/Opus),
//! RIFF INFO (WAV), and other container-specific tag formats. Returns best-effort
//! metadata — failures are logged at debug level and produce default values.

use lofty::prelude::{Accessor, AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use std::io::Cursor;

/// Tag and audio-property data extracted from an audio/video file.
// Fields are consumed by the inference path in the follow-up PR.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct AudioTags {
    pub title: Option<String>,
    /// Artist/performer — maps to `Metadata::authors` (wrapped in a Vec).
    pub artist: Option<String>,
    /// Release year as a four-digit string (e.g. "2023") for `Metadata::created_at`.
    pub year: Option<String>,
    /// ISO 639 language tag if present in the file (e.g. "eng", "deu").
    pub language: Option<String>,
    /// Duration in milliseconds from the file's audio properties.
    pub duration_ms: Option<u64>,
    /// Sample rate in Hz from the file's audio properties.
    pub sample_rate_hz: Option<u32>,
    /// Channel count from the file's audio properties.
    pub channels: Option<u16>,
    /// Overall bitrate in kbps from the file's audio properties.
    pub bitrate: Option<u32>,
    /// Container format string derived from lofty's `FileType`.
    pub container: Option<String>,
}

/// Attempt to read audio tags and properties from raw bytes.
///
/// Never panics or returns an error — failures produce `AudioTags::default()`.
pub fn read_audio_tags(bytes: &[u8]) -> AudioTags {
    let cursor = Cursor::new(bytes);
    // guess_file_type() → Result<_, io::Error>; read() → Result<_, LoftyError>.
    // Both error types differ, so chain via .ok() and fall back gracefully.
    let Some(tagged_file) = Probe::new(cursor).guess_file_type().ok().and_then(|p| {
        p.read()
            .map_err(|e| tracing::debug!("lofty read failed (non-fatal): {e}"))
            .ok()
    }) else {
        return AudioTags::default();
    };

    let primary = tagged_file.primary_tag();

    use lofty::tag::ItemKey;

    let title = primary.and_then(|t| t.title()).map(|s| s.into_owned());
    let artist = primary.and_then(|t| t.artist()).map(|s| s.into_owned());
    // `year()` was dropped from lofty 0.24's Accessor; use get_string with ItemKey::Year.
    let year = primary.and_then(|t| t.get_string(ItemKey::Year)).map(|s| s.to_string());
    let language = primary
        .and_then(|t| t.get_string(ItemKey::Language))
        .map(|s| s.to_string());

    let props = tagged_file.properties();
    let duration_ms = Some(props.duration().as_millis() as u64);
    let sample_rate_hz = props.sample_rate();
    let channels = props.channels().map(|c| c as u16);
    let bitrate = props.overall_bitrate();
    let container = Some(format!("{:?}", tagged_file.file_type()).to_lowercase());

    AudioTags {
        title,
        artist,
        year,
        language,
        duration_ms,
        sample_rate_hz,
        channels,
        bitrate,
        container,
    }
}
