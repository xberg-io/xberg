use crate::lh;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Chroma {
    C420,
    C422,
    C444,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RgbChroma {
    C444,
    Rgb,
    Rgba,
    HdrRgbBe,
    HdrRgbaBe,
    HdrRgbLe,
    HdrRgbaLe,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Undefined,
    YCbCr(Chroma),
    Rgb(RgbChroma),
    Monochrome,
    /// Indicates that this image has no visual channels.
    #[cfg(feature = "v1_19")]
    NonVisual,
}

impl ColorSpace {
    pub(crate) fn from_libheif(color_space: lh::heif_colorspace, chroma: lh::heif_chroma) -> Option<Self> {
        match color_space {
            lh::heif_colorspace_heif_colorspace_undefined => Some(ColorSpace::Undefined),
            lh::heif_colorspace_heif_colorspace_monochrome => Some(ColorSpace::Monochrome),
            lh::heif_colorspace_heif_colorspace_YCbCr => match chroma {
                lh::heif_chroma_heif_chroma_420 => Some(ColorSpace::YCbCr(Chroma::C420)),
                lh::heif_chroma_heif_chroma_422 => Some(ColorSpace::YCbCr(Chroma::C422)),
                lh::heif_chroma_heif_chroma_444 => Some(ColorSpace::YCbCr(Chroma::C444)),
                _ => None,
            },
            lh::heif_colorspace_heif_colorspace_RGB => match chroma {
                lh::heif_chroma_heif_chroma_444 => Some(ColorSpace::Rgb(RgbChroma::C444)),
                lh::heif_chroma_heif_chroma_interleaved_RGB => Some(ColorSpace::Rgb(RgbChroma::Rgb)),
                lh::heif_chroma_heif_chroma_interleaved_RGBA => Some(ColorSpace::Rgb(RgbChroma::Rgba)),
                lh::heif_chroma_heif_chroma_interleaved_RRGGBB_BE => Some(ColorSpace::Rgb(RgbChroma::HdrRgbBe)),
                lh::heif_chroma_heif_chroma_interleaved_RRGGBB_LE => Some(ColorSpace::Rgb(RgbChroma::HdrRgbLe)),
                lh::heif_chroma_heif_chroma_interleaved_RRGGBBAA_BE => Some(ColorSpace::Rgb(RgbChroma::HdrRgbaBe)),
                lh::heif_chroma_heif_chroma_interleaved_RRGGBBAA_LE => Some(ColorSpace::Rgb(RgbChroma::HdrRgbaLe)),
                _ => None,
            },
            #[cfg(feature = "v1_19")]
            lh::heif_colorspace_heif_colorspace_nonvisual => Some(ColorSpace::NonVisual),
            _ => None,
        }
    }

    pub(crate) fn heif_color_space(self) -> lh::heif_colorspace {
        match self {
            ColorSpace::YCbCr(_) => lh::heif_colorspace_heif_colorspace_YCbCr,
            ColorSpace::Rgb(_) => lh::heif_colorspace_heif_colorspace_RGB,
            ColorSpace::Monochrome => lh::heif_colorspace_heif_colorspace_monochrome,
            ColorSpace::Undefined => lh::heif_colorspace_heif_colorspace_undefined,
            #[cfg(feature = "v1_19")]
            ColorSpace::NonVisual => lh::heif_colorspace_heif_colorspace_nonvisual,
        }
    }

    pub(crate) fn heif_chroma(self) -> lh::heif_chroma {
        match self {
            ColorSpace::YCbCr(chroma) => match chroma {
                Chroma::C420 => lh::heif_chroma_heif_chroma_420,
                Chroma::C422 => lh::heif_chroma_heif_chroma_422,
                Chroma::C444 => lh::heif_chroma_heif_chroma_444,
            },
            ColorSpace::Rgb(chroma) => match chroma {
                RgbChroma::C444 => lh::heif_chroma_heif_chroma_444,
                RgbChroma::Rgb => lh::heif_chroma_heif_chroma_interleaved_RGB,
                RgbChroma::Rgba => lh::heif_chroma_heif_chroma_interleaved_RGBA,
                RgbChroma::HdrRgbBe => lh::heif_chroma_heif_chroma_interleaved_RRGGBB_BE,
                RgbChroma::HdrRgbLe => lh::heif_chroma_heif_chroma_interleaved_RRGGBB_LE,
                RgbChroma::HdrRgbaBe => lh::heif_chroma_heif_chroma_interleaved_RRGGBBAA_BE,
                RgbChroma::HdrRgbaLe => lh::heif_chroma_heif_chroma_interleaved_RRGGBBAA_LE,
            },
            ColorSpace::Undefined => lh::heif_chroma_heif_chroma_undefined,
            ColorSpace::Monochrome => lh::heif_chroma_heif_chroma_monochrome,
            #[cfg(feature = "v1_19")]
            ColorSpace::NonVisual => lh::heif_chroma_heif_chroma_undefined,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum Channel {
    Y = lh::heif_channel_heif_channel_Y as _,
    Cb = lh::heif_channel_heif_channel_Cb as _,
    Cr = lh::heif_channel_heif_channel_Cr as _,
    R = lh::heif_channel_heif_channel_R as _,
    G = lh::heif_channel_heif_channel_G as _,
    B = lh::heif_channel_heif_channel_B as _,
    Alpha = lh::heif_channel_heif_channel_Alpha as _,
    Interleaved = lh::heif_channel_heif_channel_interleaved as _,
    #[cfg(feature = "v1_19")]
    FilterArray = lh::heif_channel_heif_channel_filter_array as _,
    #[cfg(feature = "v1_19")]
    Depth = lh::heif_channel_heif_channel_depth as _,
    #[cfg(feature = "v1_19")]
    Disparity = lh::heif_channel_heif_channel_disparity as _,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ReaderGrowStatus {
    SizeReached = lh::heif_reader_grow_status_heif_reader_grow_status_size_reached as _,
    Timeout = lh::heif_reader_grow_status_heif_reader_grow_status_timeout as _,
    SizeBeyondEof = lh::heif_reader_grow_status_heif_reader_grow_status_size_beyond_eof as _,
    #[cfg(feature = "v1_19")]
    Error = lh::heif_reader_grow_status_heif_reader_grow_status_error as _,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum FileTypeResult {
    No = lh::heif_filetype_result_heif_filetype_no as _,
    /// It is HEIF and can be read by libheif
    Supported = lh::heif_filetype_result_heif_filetype_yes_supported as _,
    /// It is HEIF, but cannot be read by libheif
    Unsupported = lh::heif_filetype_result_heif_filetype_yes_unsupported as _,
    /// Not sure whether it is an HEIF, try detection with more input data
    MayBe = lh::heif_filetype_result_heif_filetype_maybe as _,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum ImageOrientation {
    Normal = lh::heif_orientation_heif_orientation_normal as _,
    FlipHorizontally = lh::heif_orientation_heif_orientation_flip_horizontally as _,
    Rotate180 = lh::heif_orientation_heif_orientation_rotate_180 as _,
    FlipVertically = lh::heif_orientation_heif_orientation_flip_vertically as _,
    Rotate90CwThenFlipHorizontally = lh::heif_orientation_heif_orientation_rotate_90_cw_then_flip_horizontally as _,
    Rotate90Cw = lh::heif_orientation_heif_orientation_rotate_90_cw as _,
    Rotate90CwThenFlipVertically = lh::heif_orientation_heif_orientation_rotate_90_cw_then_flip_vertically as _,
    Rotate270Cw = lh::heif_orientation_heif_orientation_rotate_270_cw as _,
    /// This value is used when library `libheif` returns unknown value of image orientation.
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[non_exhaustive]
#[repr(C)]
pub enum ChromaDownsamplingAlgorithm {
    NearestNeighbor = lh::heif_chroma_downsampling_algorithm_heif_chroma_downsampling_nearest_neighbor as _,
    Average = lh::heif_chroma_downsampling_algorithm_heif_chroma_downsampling_average as _,
    /// Combine with `ChromaUpsamplingAlgorithm::Bilinear` for best quality.
    /// Makes edges look sharper when using YUV 420 with bilinear chroma upsampling.
    SharpYuv = lh::heif_chroma_downsampling_algorithm_heif_chroma_downsampling_sharp_yuv as _,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[non_exhaustive]
#[repr(C)]
pub enum ChromaUpsamplingAlgorithm {
    NearestNeighbor = lh::heif_chroma_upsampling_algorithm_heif_chroma_upsampling_nearest_neighbor as _,
    Bilinear = lh::heif_chroma_upsampling_algorithm_heif_chroma_upsampling_bilinear as _,
}

#[cfg(feature = "v1_20")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AlphaCompositionMode {
    #[default]
    None,
    SolidColor {
        background_rgb: [u16; 3],
    },
    Checkerboard {
        background_rgb: [u16; 3],
        secondary_background_rgb: [u16; 3],
        square_size: u16,
    },
}

#[cfg(feature = "v1_20")]
#[allow(unsafe_code)]
impl AlphaCompositionMode {
    pub(crate) fn from_libheif(cc_options_ext: *const lh::heif_color_conversion_options_ext) -> Self {
        // SAFETY: `cc_options_ext` is null or points at the color-conversion options struct the caller
        // keeps alive for this call, so the shared reference cannot outlive its allocation.
        let cc_options_ext = unsafe { cc_options_ext.as_ref() };
        let Some(cc_options_ext) = cc_options_ext else {
            return Self::None;
        };
        match cc_options_ext.alpha_composition_mode {
            lh::heif_alpha_composition_mode_heif_alpha_composition_mode_none => AlphaCompositionMode::None,
            lh::heif_alpha_composition_mode_heif_alpha_composition_mode_solid_color => {
                AlphaCompositionMode::SolidColor {
                    background_rgb: [
                        cc_options_ext.background_red,
                        cc_options_ext.background_green,
                        cc_options_ext.background_blue,
                    ],
                }
            }
            lh::heif_alpha_composition_mode_heif_alpha_composition_mode_checkerboard => {
                AlphaCompositionMode::Checkerboard {
                    background_rgb: [
                        cc_options_ext.background_red,
                        cc_options_ext.background_green,
                        cc_options_ext.background_blue,
                    ],
                    secondary_background_rgb: [
                        cc_options_ext.secondary_background_red,
                        cc_options_ext.secondary_background_green,
                        cc_options_ext.secondary_background_blue,
                    ],
                    square_size: cc_options_ext.checkerboard_square_size,
                }
            }
            _ => AlphaCompositionMode::None,
        }
    }

    pub(crate) fn fill_libheif_cc_options_ext(&self, cc_options_ext_ptr: *mut lh::heif_color_conversion_options_ext) {
        // SAFETY: `cc_options_ext_ptr` is null or points at writable, initialized options storage the caller
        // owns for the duration of the call.
        if let Some(cc_options_ext) = unsafe { cc_options_ext_ptr.as_mut() } {
            match self {
                AlphaCompositionMode::None => {
                    cc_options_ext.alpha_composition_mode =
                        lh::heif_alpha_composition_mode_heif_alpha_composition_mode_none;
                }
                AlphaCompositionMode::SolidColor { background_rgb } => {
                    cc_options_ext.alpha_composition_mode =
                        lh::heif_alpha_composition_mode_heif_alpha_composition_mode_solid_color;
                    cc_options_ext.background_red = background_rgb[0];
                    cc_options_ext.background_green = background_rgb[1];
                    cc_options_ext.background_blue = background_rgb[2];
                }
                AlphaCompositionMode::Checkerboard {
                    background_rgb,
                    secondary_background_rgb,
                    square_size,
                } => {
                    cc_options_ext.alpha_composition_mode =
                        lh::heif_alpha_composition_mode_heif_alpha_composition_mode_checkerboard;
                    cc_options_ext.background_red = background_rgb[0];
                    cc_options_ext.background_green = background_rgb[1];
                    cc_options_ext.background_blue = background_rgb[2];
                    cc_options_ext.secondary_background_red = secondary_background_rgb[0];
                    cc_options_ext.secondary_background_green = secondary_background_rgb[1];
                    cc_options_ext.secondary_background_blue = secondary_background_rgb[2];
                    cc_options_ext.checkerboard_square_size = *square_size;
                }
            }
        }
    }
}
