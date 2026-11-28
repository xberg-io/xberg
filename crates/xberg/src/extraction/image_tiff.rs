//! Bounded walk over the image file directories (IFDs) of a TIFF, and isolation of one frame.
//!
//! The walk reads only the IFD chain structure (entry counts and next-frame links), never pixel
//! data, so it holds no decoder call: a frame that cannot be decoded (unsupported compression,
//! damaged strips) is still listed and fails later, when that frame alone is read.

use crate::error::{Result, XbergError};

/// Frames read from a TIFF when `security_limits.max_pages` is unset.
///
/// PDF extraction has no default page ceiling, but a TIFF frame costs one full OCR pass and a
/// crafted file can hold tens of thousands of ~100-byte IFDs that share strip data, so the
/// frame count is bounded by default. 1000 frames is far above scanned-document batches in
/// practice; raise `security_limits.max_pages` for a longer legitimate file.
pub(crate) const DEFAULT_MAX_TIFF_FRAMES: usize = 1_000;

/// Why the walk ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TiffWalkEnd {
    /// The last directory has no next-frame link.
    Complete,
    /// More frames follow the `max_frames` read.
    CapReached,
    /// The directory of the frame after the last listed one could not be read.
    Broken(&'static str),
}

/// The frames found by [`walk_tiff_ifds`], in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TiffIfdWalk {
    /// Byte offset of each readable frame directory.
    pub offsets: Vec<u64>,
    pub end: TiffWalkEnd,
}

#[derive(Clone, Copy)]
struct Layout {
    big_endian: bool,
    bigtiff: bool,
}

impl Layout {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let big_endian = match bytes.get(..2)? {
            b"II" => false,
            b"MM" => true,
            _ => return None,
        };
        let mut layout = Self {
            big_endian,
            bigtiff: false,
        };
        layout.bigtiff = match layout.read(bytes, 2, 2)? {
            42 => false,
            43 => true,
            _ => return None,
        };
        Some(layout)
    }

    fn read(self, bytes: &[u8], at: usize, len: usize) -> Option<u64> {
        let raw = bytes.get(at..at.checked_add(len)?)?;
        Some(raw.iter().enumerate().fold(0u64, |value, (index, byte)| {
            let shift = if self.big_endian { len - 1 - index } else { index } * 8;
            value | (u64::from(*byte) << shift)
        }))
    }

    fn write(self, bytes: &mut [u8], at: usize, len: usize, value: u64) -> Option<()> {
        let target = bytes.get_mut(at..at.checked_add(len)?)?;
        for (index, slot) in target.iter_mut().enumerate() {
            let shift = if self.big_endian { len - 1 - index } else { index } * 8;
            *slot = (value >> shift) as u8;
        }
        Some(())
    }

    /// Width of an offset or entry count: 4 and 2 bytes, or 8 and 8 for BigTIFF.
    fn pointer_width(self) -> usize {
        if self.bigtiff { 8 } else { 4 }
    }

    fn count_width(self) -> usize {
        if self.bigtiff { 8 } else { 2 }
    }

    fn entry_size(self) -> usize {
        if self.bigtiff { 20 } else { 12 }
    }

    fn header_pointer_at(self) -> usize {
        if self.bigtiff { 8 } else { 4 }
    }

    /// Where the next-frame link of the directory at `ifd` sits, or why it cannot be read.
    fn next_link_at(self, bytes: &[u8], ifd: usize) -> std::result::Result<usize, &'static str> {
        let entries = self
            .read(bytes, ifd, self.count_width())
            .ok_or("directory offset is past the end of the file")?;
        let entries = usize::try_from(entries).map_err(|_| "directory entry count is too large")?;
        let link = entries
            .checked_mul(self.entry_size())
            .and_then(|size| size.checked_add(ifd)?.checked_add(self.count_width()))
            .ok_or("directory entry count is too large")?;
        let end = link
            .checked_add(self.pointer_width())
            .ok_or("directory entry count is too large")?;
        if end > bytes.len() {
            return Err("directory is cut off by the end of the file");
        }
        Ok(link)
    }
}

/// List the frame directories of a TIFF, stopping after `max_frames` of them.
///
/// Returns `None` when `bytes` has no TIFF header. A cycle or a link past the end of the file
/// ends the walk as [`TiffWalkEnd::Broken`] with the frames read so far.
pub(crate) fn walk_tiff_ifds(bytes: &[u8], max_frames: usize) -> Option<TiffIfdWalk> {
    let layout = Layout::parse(bytes)?;
    let mut next = layout.read(bytes, layout.header_pointer_at(), layout.pointer_width())?;
    let mut offsets: Vec<u64> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let end = loop {
        if next == 0 {
            break if offsets.is_empty() {
                TiffWalkEnd::Broken("the file has no image directory")
            } else {
                TiffWalkEnd::Complete
            };
        }
        if offsets.len() >= max_frames {
            break TiffWalkEnd::CapReached;
        }
        if !seen.insert(next) {
            break TiffWalkEnd::Broken("directory links form a cycle");
        }
        let Ok(ifd) = usize::try_from(next) else {
            break TiffWalkEnd::Broken("directory offset is not addressable");
        };
        match layout.next_link_at(bytes, ifd) {
            Ok(link) => {
                offsets.push(next);
                next = layout.read(bytes, link, layout.pointer_width()).unwrap_or(0);
            }
            Err(reason) => break TiffWalkEnd::Broken(reason),
        }
    };
    Some(TiffIfdWalk { offsets, end })
}

/// Number of frames of a TIFF, at least 1, bounded by [`DEFAULT_MAX_TIFF_FRAMES`].
pub(crate) fn frame_count(bytes: &[u8]) -> Result<usize> {
    let walk = walk_tiff_ifds(bytes, DEFAULT_MAX_TIFF_FRAMES)
        .ok_or_else(|| XbergError::parsing("TIFF decode: not a TIFF header".to_string()))?;
    Ok(walk.offsets.len().max(1))
}

/// A copy of a TIFF whose only frame is the one at `ifd_offset`: the header points at that
/// directory and the directory's next-frame link is cleared. The strip and tile data stay where
/// they are, so every colour type and compression the decoder reads is kept unchanged.
///
/// Returns `None` for a header or directory that does not parse.
pub(crate) fn single_frame_tiff(bytes: &[u8], ifd_offset: u64) -> Option<Vec<u8>> {
    let layout = Layout::parse(bytes)?;
    let link = layout.next_link_at(bytes, usize::try_from(ifd_offset).ok()?).ok()?;
    let mut patched = bytes.to_vec();
    layout.write(
        &mut patched,
        layout.header_pointer_at(),
        layout.pointer_width(),
        ifd_offset,
    )?;
    layout.write(&mut patched, link, layout.pointer_width(), 0)?;
    Some(patched)
}

#[cfg(test)]
#[path = "image_tiff_tests.rs"]
mod tests;
