use crate::lh;
use four_cc::FourCC;

pub type ColorProfileType = FourCC;

pub mod color_profile_types {
    use super::{ColorProfileType, FourCC};

    /// Color profile is not present inside image
    pub const NOT_PRESENT: ColorProfileType = FourCC(0u32.to_le_bytes());
    /// NCLX color profile
    pub const NCLX: ColorProfileType = FourCC(*b"nclx");
    /// Restricted ICC color profile
    pub const R_ICC: ColorProfileType = FourCC(*b"rICC");
    /// Prof color profile
    pub const PROF: ColorProfileType = FourCC(*b"prof");
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum ColorPrimaries {
    Unspecified = lh::heif_color_primaries_heif_color_primaries_unspecified as _,
    ITU_R_BT_470_6_System_M = lh::heif_color_primaries_heif_color_primaries_ITU_R_BT_470_6_System_M as _,
    ITU_R_BT_470_6_System_B_G = lh::heif_color_primaries_heif_color_primaries_ITU_R_BT_470_6_System_B_G as _,
    ITU_R_BT_601_6 = lh::heif_color_primaries_heif_color_primaries_ITU_R_BT_601_6 as _,
    ITU_R_BT_709_5 = lh::heif_color_primaries_heif_color_primaries_ITU_R_BT_709_5 as _,
    ITU_R_BT_2020_2_and_2100_0 = lh::heif_color_primaries_heif_color_primaries_ITU_R_BT_2020_2_and_2100_0 as _,
    GenericFilm = lh::heif_color_primaries_heif_color_primaries_generic_film as _,
    SMPTE_240M = lh::heif_color_primaries_heif_color_primaries_SMPTE_240M as _,
    SMPTE_ST_428_1 = lh::heif_color_primaries_heif_color_primaries_SMPTE_ST_428_1 as _,
    SMPTE_RP_431_2 = lh::heif_color_primaries_heif_color_primaries_SMPTE_RP_431_2 as _,
    SMPTE_EG_432_1 = lh::heif_color_primaries_heif_color_primaries_SMPTE_EG_432_1 as _,
    EBU_Tech_3213_E = lh::heif_color_primaries_heif_color_primaries_EBU_Tech_3213_E as _,
    /// This value is used when library `libheif` returns unknown value of color primaries.
    Unknown,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum TransferCharacteristics {
    ITU_R_BT_709_5 = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_709_5 as _,
    Unspecified = lh::heif_transfer_characteristics_heif_transfer_characteristic_unspecified as _,
    ITU_R_BT_470_6_System_M =
        lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_470_6_System_M as _,
    ITU_R_BT_470_6_System_B_G =
        lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_470_6_System_B_G as _,
    ITU_R_BT_601_6 = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_601_6 as _,
    SMPTE_240M = lh::heif_transfer_characteristics_heif_transfer_characteristic_SMPTE_240M as _,
    Linear = lh::heif_transfer_characteristics_heif_transfer_characteristic_linear as _,
    Logarithmic100 = lh::heif_transfer_characteristics_heif_transfer_characteristic_logarithmic_100 as _,
    Logarithmic100Sqrt10 = lh::heif_transfer_characteristics_heif_transfer_characteristic_logarithmic_100_sqrt10 as _,
    IEC_61966_2_4 = lh::heif_transfer_characteristics_heif_transfer_characteristic_IEC_61966_2_4 as _,
    ITU_R_BT_1361 = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_1361 as _,
    IEC_61966_2_1 = lh::heif_transfer_characteristics_heif_transfer_characteristic_IEC_61966_2_1 as _,
    ITU_R_BT_2020_2_10bit = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_2020_2_10bit as _,
    ITU_R_BT_2020_2_12bit = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_2020_2_12bit as _,
    ITU_R_BT_2100_0_PQ = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_2100_0_PQ as _,
    SMPTE_ST_428_1 = lh::heif_transfer_characteristics_heif_transfer_characteristic_SMPTE_ST_428_1 as _,
    ITU_R_BT_2100_0_HLG = lh::heif_transfer_characteristics_heif_transfer_characteristic_ITU_R_BT_2100_0_HLG as _,
    /// This value is used when library `libheif` returns unknown value of transfer characteristics.
    Unknown,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, enumn::N)]
#[repr(C)]
pub enum MatrixCoefficients {
    RGB_GBR = lh::heif_matrix_coefficients_heif_matrix_coefficients_RGB_GBR as _,
    ITU_R_BT_709_5 = lh::heif_matrix_coefficients_heif_matrix_coefficients_ITU_R_BT_709_5 as _,
    Unspecified = lh::heif_matrix_coefficients_heif_matrix_coefficients_unspecified as _,
    US_FCC_T47 = lh::heif_matrix_coefficients_heif_matrix_coefficients_US_FCC_T47 as _,
    ITU_R_BT_470_6_System_B_G = lh::heif_matrix_coefficients_heif_matrix_coefficients_ITU_R_BT_470_6_System_B_G as _,
    ITU_R_BT_601_6 = lh::heif_matrix_coefficients_heif_matrix_coefficients_ITU_R_BT_601_6 as _,
    SMPTE_240M = lh::heif_matrix_coefficients_heif_matrix_coefficients_SMPTE_240M as _,
    YCgCo = lh::heif_matrix_coefficients_heif_matrix_coefficients_YCgCo as _,
    ITU_R_BT_2020_2_NonConstantLuminance =
        lh::heif_matrix_coefficients_heif_matrix_coefficients_ITU_R_BT_2020_2_non_constant_luminance as _,
    ITU_R_BT_2020_2_ConstantLuminance =
        lh::heif_matrix_coefficients_heif_matrix_coefficients_ITU_R_BT_2020_2_constant_luminance as _,
    SMPTE_ST_2085 = lh::heif_matrix_coefficients_heif_matrix_coefficients_SMPTE_ST_2085 as _,
    ChromaticityDerivedNonConstantLuminance =
        lh::heif_matrix_coefficients_heif_matrix_coefficients_chromaticity_derived_non_constant_luminance as _,
    ChromaticityDerivedConstantLuminance =
        lh::heif_matrix_coefficients_heif_matrix_coefficients_chromaticity_derived_constant_luminance as _,
    ICtCp = lh::heif_matrix_coefficients_heif_matrix_coefficients_ICtCp as _,
    /// This value is used when library `libheif` returns unknown value of matrix coefficients.
    Unknown,
}

pub trait ColorProfile {
    fn profile_type(&self) -> ColorProfileType;
}

pub struct ColorProfileRaw {
    pub(crate) typ: ColorProfileType,
    pub data: Vec<u8>,
}

impl ColorProfileRaw {
    pub fn new(typ: ColorProfileType, data: Vec<u8>) -> Self {
        Self { typ, data }
    }
}

impl ColorProfile for ColorProfileRaw {
    fn profile_type(&self) -> ColorProfileType {
        self.typ
    }
}

#[derive(Debug)]
pub struct ColorProfileNCLX {
    pub(crate) inner: *mut lh::heif_color_profile_nclx,
}

#[allow(unsafe_code)]
impl Drop for ColorProfileNCLX {
    fn drop(&mut self) {
        // SAFETY: `inner` is the profile this value allocated and owns; this free is the counterpart of
        // `heif_nclx_color_profile_alloc` and runs exactly once.
        unsafe {
            lh::heif_nclx_color_profile_free(self.inner);
        }
    }
}

impl ColorProfile for ColorProfileNCLX {
    fn profile_type(&self) -> ColorProfileType {
        color_profile_types::NCLX
    }
}

#[allow(unsafe_code)]
impl ColorProfileNCLX {
    /// `None` when libheif cannot allocate the profile or cannot be loaded (`dynamic` feature).
    pub fn new() -> Option<Self> {
        if !crate::libheif_available() {
            return None;
        }
        // SAFETY: The allocator takes no caller pointers; it returns an owned profile, and a null result
        // becomes `None` rather than a wrapper.
        let inner = unsafe { lh::heif_nclx_color_profile_alloc() };
        (!inner.is_null()).then_some(Self { inner })
    }

    #[inline(always)]
    #[allow(unsafe_code)]
    fn inner_ref(&self) -> &lh::heif_color_profile_nclx {
        // SAFETY: `inner` is non-null by construction (both constructors reject null) and owned by this
        // value, which keeps it alive for the shared borrow.
        unsafe { &(*self.inner) }
    }

    #[inline(always)]
    #[allow(unsafe_code)]
    fn inner_mut(&mut self) -> &mut lh::heif_color_profile_nclx {
        // SAFETY: `inner` is non-null by construction and owned by this value; `&mut self` grants exclusive
        // access for the returned borrow.
        unsafe { &mut (*self.inner) }
    }

    pub fn version(&self) -> u8 {
        self.inner_ref().version
    }

    pub fn color_primaries(&self) -> ColorPrimaries {
        ColorPrimaries::n(self.inner_ref().color_primaries).unwrap_or(ColorPrimaries::Unknown)
    }

    pub fn set_color_primaries(&mut self, v: ColorPrimaries) {
        if v != ColorPrimaries::Unknown {
            self.inner_mut().color_primaries = v as _;
        }
    }

    pub fn transfer_characteristics(&self) -> TransferCharacteristics {
        TransferCharacteristics::n(self.inner_ref().transfer_characteristics)
            .unwrap_or(TransferCharacteristics::Unknown)
    }

    pub fn matrix_coefficients(&self) -> MatrixCoefficients {
        MatrixCoefficients::n(self.inner_ref().matrix_coefficients).unwrap_or(MatrixCoefficients::Unknown)
    }

    pub fn full_range_flag(&self) -> u8 {
        self.inner_ref().full_range_flag
    }

    pub fn color_primary_red_x(&self) -> f32 {
        self.inner_ref().color_primary_red_x
    }

    pub fn color_primary_red_y(&self) -> f32 {
        self.inner_ref().color_primary_red_y
    }

    pub fn color_primary_green_x(&self) -> f32 {
        self.inner_ref().color_primary_green_x
    }

    pub fn color_primary_green_y(&self) -> f32 {
        self.inner_ref().color_primary_green_y
    }

    pub fn color_primary_blue_x(&self) -> f32 {
        self.inner_ref().color_primary_blue_x
    }

    pub fn color_primary_blue_y(&self) -> f32 {
        self.inner_ref().color_primary_blue_y
    }

    pub fn color_primary_white_x(&self) -> f32 {
        self.inner_ref().color_primary_white_x
    }

    pub fn color_primary_white_y(&self) -> f32 {
        self.inner_ref().color_primary_white_y
    }
}
