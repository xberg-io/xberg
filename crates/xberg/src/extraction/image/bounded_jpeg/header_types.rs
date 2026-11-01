pub(super) const COMPONENTS: usize = 3;

#[derive(Clone, Copy)]
pub(super) struct Header {
    pub width: u32,
    pub height: u32,
    pub horizontal_sampling: u8,
    pub vertical_sampling: u8,
    pub metadata_bytes: u64,
    pub metadata_segments: usize,
    pub(super) component_ids: [u8; COMPONENTS],
}
