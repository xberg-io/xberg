use std::ptr;
use std::rc::Rc;

use four_cc::FourCC;
use libheif_sys as lh;

use crate::context::ContextOwner;
use crate::decoder::get_decoding_options_ptr;
use crate::{ColorSpace, DecodingOptions, HeifError, Image, Result};

pub type TrackType = FourCC;

pub mod track_types {
    use super::{FourCC, TrackType};

    pub const VIDEO: TrackType = FourCC(*b"vide");
    pub const IMAGE_SEQUENCE: TrackType = FourCC(*b"pict");
    pub const AUXILIARY: TrackType = FourCC(*b"auxv");
    pub const METADATA: TrackType = FourCC(*b"meta");
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct ImageResolution {
    pub width: u16,
    pub height: u16,
}

pub struct Track<'a> {
    pub(crate) inner: *mut lh::heif_track,
    _context: Rc<ContextOwner<'a>>,
}

#[allow(unsafe_code)]
impl<'a> Track<'a> {
    #[inline]
    pub(crate) fn from_heif_track(track: *mut lh::heif_track, context: &Rc<ContextOwner<'a>>) -> Track<'a> {
        Track {
            inner: track,
            _context: Rc::clone(context),
        }
    }

    /// Get the ID of the track.
    ///
    /// The track ID will never be 0.
    #[allow(unsafe_code)]
    pub fn id(&self) -> u32 {
        unsafe { lh::heif_track_get_id(self.inner) }
    }

    /// Get the four-cc track handler type.
    ///
    /// Typical codes are "vide" for video sequences, "pict" for image sequences,
    /// "meta" for metadata tracks.
    /// These are defined in [`track_types`] module, but files may also contain other types.
    #[allow(unsafe_code)]
    pub fn handler_type(&self) -> TrackType {
        let c_track_type = unsafe { lh::heif_track_get_track_handler_type(self.inner) };
        TrackType::from(c_track_type as u32)
    }

    #[cfg(feature = "v1_21")]
    #[allow(unsafe_code)]
    pub fn has_alpha_channel(&self) -> bool {
        unsafe { lh::heif_track_has_alpha_channel(self.inner) != 0 }
    }

    /// Get the timescale (clock ticks per second) for this track.
    ///
    /// Note that this can be different from the timescale used at sequence level.
    #[allow(unsafe_code)]
    pub fn timescale(&self) -> u32 {
        unsafe { lh::heif_track_get_timescale(self.inner) }
    }

    /// Get the image resolution of the track.
    ///
    /// If the track is no visual track, an error is returned.
    #[allow(unsafe_code)]
    pub fn image_resolution(&self) -> Result<ImageResolution> {
        let mut res = ImageResolution::default();

        let err = unsafe { lh::heif_track_get_image_resolution(self.inner, &mut res.width, &mut res.height) };

        HeifError::from_heif_error(err)?;
        Ok(res)
    }

    /// Decode the next image in the sequence track.
    ///
    /// If there is no more image in the sequence,
    /// error with code [HeifErrorCode::EndOfSequence](crate::HeifErrorCode::EndOfSequence)
    /// will be returned.
    /// The parameters `color_space` and `decoding_options` are similar to
    /// [LibHeif::decode()](crate::LibHeif::decode).
    /// If you want to let `libheif` decide the output colorspace and chroma,
    /// set `color_space` parameter to [ColorSpace::Undefined].
    /// Usually, `libheif` will return the image in the input colorspace,
    /// but it may also modify it for example when it has to rotate the image.
    /// If you want to get the image in a specific colorspace/chroma format,
    /// you can specify this and `libheif` will convert the image to match this format.
    #[allow(unsafe_code)]
    pub fn decode_next_image(
        &self,
        color_space: ColorSpace,
        decoding_options: Option<DecodingOptions>,
    ) -> Result<Image> {
        let mut c_image: *mut lh::heif_image = ptr::null_mut();
        let err = unsafe {
            lh::heif_track_decode_next_image(
                self.inner,
                &mut c_image,
                color_space.heif_color_space(),
                color_space.heif_chroma(),
                get_decoding_options_ptr(&decoding_options),
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Image::from_heif_image(c_image))
    }
}

#[allow(unsafe_code)]
impl Drop for Track<'_> {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `inner` is the track this value owns and releases exactly once, before the
        // context share in `_context` is dropped, so the native context is still live.
        unsafe { lh::heif_track_release(self.inner) };
    }
}
