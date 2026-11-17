use std::ptr::NonNull;

use libheif_sys as lh;

/// Security limits.
///
/// If you set a limit to 0, the limit is disabled.
#[derive(Clone, Copy)]
pub struct SecurityLimits {
    inner: lh::heif_security_limits,
}

#[allow(unsafe_code)]
impl Default for SecurityLimits {
    #[allow(unsafe_code)]
    fn default() -> Self {
        // SAFETY: This takes no Rust pointers and returns libheif's immutable global limits.
        let inner_ptr = unsafe { lh::heif_get_global_security_limits() };
        if inner_ptr.is_null() {
            panic!("heif_get_global_security_limits returned the null pointer");
        }
        Self {
            // SAFETY: The checked non-null pointer refers to the library's initialized
            // global value. Copy it without retaining a pointer into libheif.
            inner: unsafe { *inner_ptr },
        }
    }
}

#[allow(unsafe_code)]
impl SecurityLimits {
    /// Returns a new instance with default values of limits.
    #[allow(unsafe_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn from_inner(inner_ptr: NonNull<lh::heif_security_limits>) -> Self {
        Self {
            // SAFETY: The sole caller obtains this pointer from an initialized context
            // that is still borrowed, and we retain only a copy of its limits.
            inner: unsafe { *inner_ptr.as_ref() },
        }
    }

    pub(crate) fn as_inner(&self) -> &lh::heif_security_limits {
        &self.inner
    }

    /// Limit on the maximum image size to avoid allocating too much memory.
    /// For example, setting this to 32768^2 pixels = 1 Gigapixels results
    /// in 1.5 GB memory need for YUV-4:2:0 or 4 GB for RGB32.
    pub fn max_image_size_pixels(&self) -> u64 {
        self.inner.max_image_size_pixels
    }

    pub fn set_max_image_size_pixels(&mut self, v: u64) {
        self.inner.max_image_size_pixels = v
    }

    pub fn max_number_of_tiles(&self) -> u64 {
        self.inner.max_number_of_tiles
    }

    pub fn set_max_number_of_tiles(&mut self, v: u64) {
        self.inner.max_number_of_tiles = v
    }

    pub fn max_bayer_pattern_pixels(&self) -> u32 {
        self.inner.max_bayer_pattern_pixels
    }

    pub fn set_max_bayer_pattern_pixels(&mut self, v: u32) {
        self.inner.max_bayer_pattern_pixels = v
    }

    pub fn max_items(&self) -> u32 {
        self.inner.max_items
    }

    pub fn set_max_items(&mut self, v: u32) {
        self.inner.max_items = v
    }

    pub fn max_color_profile_size(&self) -> u32 {
        self.inner.max_color_profile_size
    }

    pub fn set_max_color_profile_size(&mut self, v: u32) {
        self.inner.max_color_profile_size = v
    }

    pub fn max_memory_block_size(&self) -> u64 {
        self.inner.max_memory_block_size
    }

    pub fn set_max_memory_block_size(&mut self, v: u64) {
        self.inner.max_memory_block_size = v
    }

    pub fn max_components(&self) -> u32 {
        self.inner.max_components
    }

    pub fn set_max_components(&mut self, v: u32) {
        self.inner.max_components = v
    }

    pub fn max_iloc_extents_per_item(&self) -> u32 {
        self.inner.max_iloc_extents_per_item
    }

    pub fn set_max_iloc_extents_per_item(&mut self, v: u32) {
        self.inner.max_iloc_extents_per_item = v
    }

    pub fn max_size_entity_group(&self) -> u32 {
        self.inner.max_size_entity_group
    }

    pub fn set_max_size_entity_group(&mut self, v: u32) {
        self.inner.max_size_entity_group = v
    }

    /// For all boxes that are not covered by other limits.
    pub fn max_children_per_box(&self) -> u32 {
        self.inner.max_children_per_box
    }

    pub fn set_max_children_per_box(&mut self, v: u32) {
        self.inner.max_children_per_box = v
    }

    #[cfg(feature = "v1_20")]
    pub fn max_total_memory(&self) -> u64 {
        self.inner.max_total_memory
    }

    #[cfg(feature = "v1_20")]
    pub fn set_max_total_memory(&mut self, v: u64) {
        self.inner.max_total_memory = v
    }

    #[cfg(feature = "v1_20")]
    pub fn max_sample_description_box_entries(&self) -> u32 {
        self.inner.max_sample_description_box_entries
    }

    #[cfg(feature = "v1_20")]
    pub fn set_max_sample_description_box_entries(&mut self, v: u32) {
        self.inner.max_sample_description_box_entries = v
    }

    #[cfg(feature = "v1_20")]
    pub fn max_sample_group_description_box_entries(&self) -> u32 {
        self.inner.max_sample_group_description_box_entries
    }

    #[cfg(feature = "v1_20")]
    pub fn set_max_sample_group_description_box_entries(&mut self, v: u32) {
        self.inner.max_sample_group_description_box_entries = v
    }

    #[cfg(feature = "v1_21")]
    pub fn max_sequence_frames(&self) -> u32 {
        self.inner.max_sequence_frames
    }

    #[cfg(feature = "v1_21")]
    pub fn set_max_sequence_frames(&mut self, v: u32) {
        self.inner.max_sequence_frames = v
    }

    #[cfg(feature = "v1_21")]
    pub fn max_number_of_file_brands(&self) -> u32 {
        self.inner.max_number_of_file_brands
    }

    #[cfg(feature = "v1_21")]
    pub fn set_max_number_of_file_brands(&mut self, v: u32) {
        self.inner.max_number_of_file_brands = v
    }
}
