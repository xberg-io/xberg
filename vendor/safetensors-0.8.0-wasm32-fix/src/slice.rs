//! Module handling lazy loading via iterating on slices on the original buffer.
use crate::lib::Vec;
use crate::tensor::{Dtype, TensorView};
use core::fmt::Display;
use core::num::NonZeroUsize;
use core::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

/// Error representing invalid slicing attempt
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum InvalidSlice {
    /// When the client asked for more slices than the tensors has dimensions
    TooManySlices,
    /// When the client asked for a slice that exceeds the allowed bounds
    SliceOutOfRange {
        /// The rank of the dimension that has the out of bounds
        dim_index: usize,
        /// The problematic value
        asked: usize,
        /// The dimension size we shouldn't go over.
        dim_size: usize,
    },
    /// For smaller than 1 byte dtypes, some slices will happen outside of the byte boundary, some special care has to be taken
    /// and standard functions will fail
    MisalignedSlice,
}

impl Display for InvalidSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            InvalidSlice::TooManySlices => {
                write!(f, "more slicing indexes than dimensions in tensor")
            }
            InvalidSlice::SliceOutOfRange {
                dim_index,
                asked,
                dim_size,
            } => {
                write!(f, "index {asked} out of bounds for tensor dimension #{dim_index} of size {dim_size}")
            }
            InvalidSlice::MisalignedSlice => {
                write!(f, "The slice is slicing for subbytes dtypes, and the slice does not end up at a byte boundary, this is invalid.")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidSlice {}

#[cfg(not(feature = "std"))]
impl core::error::Error for InvalidSlice {}

#[derive(Debug, Clone)]
/// Generic structure used to index a slice of the tensor
pub enum TensorIndexer {
    /// This is selecting an entire dimension
    Select(usize),
    /// A slice `start:stop:step`. `step` is always >= 1; a contiguous slice
    /// has `step == 1`.
    Narrow(Bound<usize>, Bound<usize>, NonZeroUsize),
}

fn display_bound(bound: &Bound<usize>) -> &dyn Display {
    match bound {
        Bound::Unbounded => &"",
        Bound::Excluded(n) => n,
        Bound::Included(n) => n,
    }
}

/// Intended for Python users mostly or at least for its conventions
impl Display for TensorIndexer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TensorIndexer::Select(n) => {
                write!(f, "{n}")
            }
            TensorIndexer::Narrow(left, right, step) => {
                if step.get() == 1 {
                    write!(f, "{}:{}", display_bound(left), display_bound(right))
                } else {
                    write!(f, "{}:{}:{step}", display_bound(left), display_bound(right))
                }
            }
        }
    }
}

impl From<usize> for TensorIndexer {
    fn from(index: usize) -> Self {
        TensorIndexer::Select(index)
    }
}

// impl From<&[usize]> for TensorIndexer {
//     fn from(index: &[usize]) -> Self {
//         let tensor = index.into();
//         TensorIndexer::IndexSelect(tensor)
//     }
// }
//
// impl From<Vec<usize>> for TensorIndexer {
//     fn from(index: Vec<usize>) -> Self {
//         let tensor = Tensor::of_slice(&index);
//         TensorIndexer::IndexSelect(tensor)
//     }
// }

macro_rules! impl_from_range {
    ($range_type:ty) => {
        impl From<$range_type> for TensorIndexer {
            fn from(range: $range_type) -> Self {
                use core::ops::Bound::*;

                let start = match range.start_bound() {
                    Included(idx) => Included(*idx),
                    Excluded(idx) => Excluded(*idx),
                    Unbounded => Unbounded,
                };

                let end = match range.end_bound() {
                    Included(idx) => Included(*idx),
                    Excluded(idx) => Excluded(*idx),
                    Unbounded => Unbounded,
                };

                TensorIndexer::Narrow(start, end, NonZeroUsize::MIN)
            }
        }
    };
}

impl_from_range!(Range<usize>);
impl_from_range!(RangeFrom<usize>);
impl_from_range!(RangeFull);
impl_from_range!(RangeInclusive<usize>);
impl_from_range!(RangeTo<usize>);
impl_from_range!(RangeToInclusive<usize>);

/// Trait used to implement multiple signatures for ease of use of the slicing
/// of a tensor
pub trait IndexOp<'data, T> {
    /// Returns a slicing iterator which are the chunks of data necessary to
    /// reconstruct the desired tensor.
    fn slice(&'data self, index: T) -> Result<SliceIterator<'data>, InvalidSlice>;
}

impl<'data, A> IndexOp<'data, A> for TensorView<'data>
where
    A: Into<TensorIndexer>,
{
    fn slice(&'data self, index: A) -> Result<SliceIterator<'data>, InvalidSlice> {
        self.sliced_data(&[index.into()])
    }
}

impl<'data, A> IndexOp<'data, (A,)> for TensorView<'data>
where
    A: Into<TensorIndexer>,
{
    fn slice(&'data self, index: (A,)) -> Result<SliceIterator<'data>, InvalidSlice> {
        let idx_a = index.0.into();
        self.sliced_data(&[idx_a])
    }
}

impl<'data, A, B> IndexOp<'data, (A, B)> for TensorView<'data>
where
    A: Into<TensorIndexer>,
    B: Into<TensorIndexer>,
{
    fn slice(&'data self, index: (A, B)) -> Result<SliceIterator<'data>, InvalidSlice> {
        let idx_a = index.0.into();
        let idx_b = index.1.into();
        self.sliced_data(&[idx_a, idx_b])
    }
}

impl<'data, A, B, C> IndexOp<'data, (A, B, C)> for TensorView<'data>
where
    A: Into<TensorIndexer>,
    B: Into<TensorIndexer>,
    C: Into<TensorIndexer>,
{
    fn slice(&'data self, index: (A, B, C)) -> Result<SliceIterator<'data>, InvalidSlice> {
        let idx_a = index.0.into();
        let idx_b = index.1.into();
        let idx_c = index.2.into();
        self.sliced_data(&[idx_a, idx_b, idx_c])
    }
}

// impl<A, B, C, D> IndexOp<(A, B, C, D)> for TensorView<'data>
// where
//     A: Into<TensorIndexer>,
//     B: Into<TensorIndexer>,
//     C: Into<TensorIndexer>,
//     D: Into<TensorIndexer>,
// {
//     fn slice(&self, index: (A, B, C, D)) -> TensorView<'data> {
//         let idx_a = index.0.into();
//         let idx_b = index.1.into();
//         let idx_c = index.2.into();
//         let idx_d = index.3.into();
//         self.sliced_data(&[idx_a, idx_b, idx_c, idx_d])
//     }
// }
//
// impl<A, B, C, D, E> IndexOp<(A, B, C, D, E)> for TensorView<'data>
// where
//     A: Into<TensorIndexer>,
//     B: Into<TensorIndexer>,
//     C: Into<TensorIndexer>,
//     D: Into<TensorIndexer>,
//     E: Into<TensorIndexer>,
// {
//     fn slice(&self, index: (A, B, C, D, E)) -> TensorView<'data> {
//         let idx_a = index.0.into();
//         let idx_b = index.1.into();
//         let idx_c = index.2.into();
//         let idx_d = index.3.into();
//         let idx_e = index.4.into();
//         self.sliced_data(&[idx_a, idx_b, idx_c, idx_d, idx_e])
//     }
// }
//
// impl<A, B, C, D, E, F> IndexOp<(A, B, C, D, E, F)> for TensorView<'data>
// where
//     A: Into<TensorIndexer>,
//     B: Into<TensorIndexer>,
//     C: Into<TensorIndexer>,
//     D: Into<TensorIndexer>,
//     E: Into<TensorIndexer>,
//     F: Into<TensorIndexer>,
// {
//     fn slice(&self, index: (A, B, C, D, E, F)) -> TensorView<'data> {
//         let idx_a = index.0.into();
//         let idx_b = index.1.into();
//         let idx_c = index.2.into();
//         let idx_d = index.3.into();
//         let idx_e = index.4.into();
//         let idx_f = index.5.into();
//         self.sliced_data(&[idx_a, idx_b, idx_c, idx_d, idx_e, idx_f])
//     }
// }
//
// impl<A, B, C, D, E, F, G> IndexOp<(A, B, C, D, E, F, G)> for TensorView<'data>
// where
//     A: Into<TensorIndexer>,
//     B: Into<TensorIndexer>,
//     C: Into<TensorIndexer>,
//     D: Into<TensorIndexer>,
//     E: Into<TensorIndexer>,
//     F: Into<TensorIndexer>,
//     G: Into<TensorIndexer>,
// {
//     fn slice(&self, index: (A, B, C, D, E, F, G)) -> TensorView<'data> {
//         let idx_a = index.0.into();
//         let idx_b = index.1.into();
//         let idx_c = index.2.into();
//         let idx_d = index.3.into();
//         let idx_e = index.4.into();
//         let idx_f = index.5.into();
//         let idx_g = index.6.into();
//         self.sliced_data(&[idx_a, idx_b, idx_c, idx_d, idx_e, idx_f, idx_g])
//     }
// }

/// Iterator used to return the bits of the overall tensor buffer
/// when client asks for a slice of the original tensor.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct SliceIterator<'data> {
    view: &'data TensorView<'data>,
    indices: Vec<(usize, usize)>,
    newshape: Vec<usize>,
}

impl<'data> SliceIterator<'data> {
    pub(crate) fn new(
        view: &'data TensorView<'data>,
        slices: &[TensorIndexer],
    ) -> Result<Self, InvalidSlice> {
        let (indices, newshape) = slice_byte_ranges(view.dtype(), view.shape(), slices)?;
        // Reversing so we can pop faster while iterating on the slice
        let indices = indices.into_iter().rev().collect();
        Ok(Self {
            view,
            indices,
            newshape,
        })
    }

    /// Gives back the amount of bytes still being in the iterator
    pub fn remaining_byte_len(&self) -> usize {
        self.indices.iter().map(|(start, stop)| stop - start).sum()
    }

    /// Gives back the amount of bytes still being in the iterator
    pub fn newshape(&self) -> Vec<usize> {
        self.newshape.clone()
    }
}

/// Byte ranges into a tensor's data section in iteration order;
/// concatenating them yields the dense destination layout.
pub type SliceByteRanges = Vec<(usize, usize)>;

/// Post-slice tensor shape (element counts per dim).
pub type SlicedShape = Vec<usize>;

/// Resolve a `(start, stop)` half-open element range from slice bounds,
/// defaulting unbounded ends to `0` and `dim`.
fn narrow_bounds(left: &Bound<usize>, right: &Bound<usize>, dim: usize) -> (usize, usize) {
    let start = match left {
        Bound::Unbounded => 0,
        Bound::Included(s) => *s,
        Bound::Excluded(s) => *s + 1,
    };
    let stop = match right {
        Bound::Unbounded => dim,
        Bound::Included(s) => *s + 1,
        Bound::Excluded(s) => *s,
    };
    (start, stop)
}

/// Compute the byte ranges and post-slice shape for a slicing operation
/// without requiring the underlying data buffer.
///
/// The returned [`SliceByteRanges`] is in source iteration order; callers
/// may reverse it for pop-based iteration.
pub fn slice_byte_ranges(
    dtype: Dtype,
    shape: &[usize],
    slices: &[TensorIndexer],
) -> Result<(SliceByteRanges, SlicedShape), InvalidSlice> {
    let n_slice = slices.len();
    let n_shape = shape.len();
    if n_slice > n_shape {
        return Err(InvalidSlice::TooManySlices);
    }
    let mut newshape = Vec::with_capacity(n_shape);

    // Minimum span is the span of 1 item;
    let mut span = dtype.bitsize();
    let mut indices: Vec<(usize, usize)> = vec![];
    // Everything is row major.
    for (i, &dim) in shape.iter().enumerate().rev() {
        if i >= slices.len() {
            // We are not slicing yet, just increase the local span
            newshape.push(dim);
        } else {
            let slice = &slices[i];
            let (start, stop, step) = match slice {
                TensorIndexer::Select(s) => (*s, *s + 1, 1),
                TensorIndexer::Narrow(left, right, step) => {
                    let (start, stop) = narrow_bounds(left, right, dim);
                    (start, stop, step.get())
                }
            };
            if start >= dim || stop > dim {
                let asked = if start >= dim {
                    start
                } else {
                    stop.saturating_sub(1)
                };
                return Err(InvalidSlice::SliceOutOfRange {
                    dim_index: i,
                    asked,
                    dim_size: dim,
                });
            }
            if !matches!(slice, TensorIndexer::Select(_)) {
                newshape.push((stop - start).div_ceil(step));
            }
            if indices.is_empty() {
                if step == 1 && start == 0 && stop == dim {
                    // Full range, nothing sliced yet; just grow the span.
                } else if step == 1 {
                    if start * span % 8 != 0 {
                        return Err(InvalidSlice::MisalignedSlice);
                    }
                    let offset = (start * span) / 8;
                    if stop * span % 8 != 0 {
                        return Err(InvalidSlice::MisalignedSlice);
                    }
                    let small_span = (stop * span) / 8 - offset;
                    indices.push((offset, offset + small_span));
                } else {
                    // Strided innermost dim: each kept element is its own run.
                    for n in (start..stop).step_by(step) {
                        if n * span % 8 != 0 || (n + 1) * span % 8 != 0 {
                            return Err(InvalidSlice::MisalignedSlice);
                        }
                        indices.push(((n * span) / 8, ((n + 1) * span) / 8));
                    }
                }
            } else {
                let capacity = (stop - start).div_ceil(step) * indices.len();
                let mut newindices = Vec::with_capacity(capacity);
                for n in (start..stop).step_by(step) {
                    if n * span % 8 != 0 {
                        return Err(InvalidSlice::MisalignedSlice);
                    }
                    let offset = (n * span) / 8;
                    for (old_start, old_stop) in &indices {
                        newindices.push((old_start + offset, old_stop + offset));
                    }
                }
                indices = newindices;
            }
        }
        span *= dim;
    }
    if indices.is_empty() {
        // Empty `slices` (or all unbounded full-range slices): no slicing
        // happened, the whole tensor is the result. `span` ended as
        // bitsize * product(shape).
        let total_bits = span;
        if total_bits % 8 != 0 {
            return Err(InvalidSlice::MisalignedSlice);
        }
        indices.push((0, total_bits / 8));
    }
    let newshape = newshape.into_iter().rev().collect();
    Ok((indices, newshape))
}

impl<'data> Iterator for SliceIterator<'data> {
    type Item = &'data [u8];

    fn next(&mut self) -> Option<Self::Item> {
        // TODO We might want to move the logic from `new`
        // here actually to remove the need to get all the indices
        // upfront.
        let (start, stop) = self.indices.pop()?;
        Some(&self.view.data()[start..stop])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{Dtype, TensorView};

    #[test]
    fn test_helpers() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![1, 2, 3], &data).unwrap();

        let iterator = SliceIterator::new(
            &attn_0,
            &[TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN)],
        )
        .unwrap();
        assert_eq!(iterator.remaining_byte_len(), 24);
        assert_eq!(iterator.newshape(), vec![1, 2, 3]);

        let iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.remaining_byte_len(), 12);
        assert_eq!(iterator.newshape(), vec![1, 1, 3]);
    }

    #[test]
    fn test_fp4_simple() {
        let data: Vec<u8> = vec![0u8, 1u8];

        let attn_0 = TensorView::new(Dtype::F4, vec![1, 2, 2], &data).unwrap();

        let iterator = SliceIterator::new(
            &attn_0,
            &[TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN)],
        )
        .unwrap();
        assert_eq!(iterator.remaining_byte_len(), 2);
        assert_eq!(iterator.newshape(), vec![1, 2, 2]);

        let iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.remaining_byte_len(), 1);
        assert_eq!(iterator.newshape(), vec![1, 1, 2]);
    }

    #[test]
    fn test_fp4_misaligned() {
        let data: Vec<u8> = vec![0u8];

        let attn_0 = TensorView::new(Dtype::F4, vec![1, 2], &data).unwrap();

        let iterator = SliceIterator::new(
            &attn_0,
            &[TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN)],
        )
        .unwrap();
        assert_eq!(iterator.remaining_byte_len(), 1);
        assert_eq!(iterator.newshape(), vec![1, 2]);

        let iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        );

        assert_eq!(iterator, Err(InvalidSlice::MisalignedSlice));
    }

    #[test]
    fn test_dummy() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![1, 2, 3], &data).unwrap();

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN)],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..24]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..24]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..24]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..24]));
        assert_eq!(iterator.next(), None);

        assert!(SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
            ],
        )
        .is_err(),);
    }

    #[test]
    fn test_slice_variety() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![1, 2, 3], &data).unwrap();

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[TensorIndexer::Narrow(
                Bound::Included(0),
                Bound::Excluded(1),
                NonZeroUsize::MIN,
            )],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..24]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..12]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[0..4]));
        assert_eq!(iterator.next(), Some(&data[12..16]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(2), NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(0), Bound::Excluded(1), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[12..16]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_slice_variety2() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![2, 3], &data).unwrap();

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Unbounded, Bound::Unbounded, NonZeroUsize::MIN),
                TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(3), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[4..12]));
        assert_eq!(iterator.next(), Some(&data[16..24]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_slice_select() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![2, 3], &data).unwrap();

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Select(1),
                TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(3), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[16..24]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Select(0),
                TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(3), NonZeroUsize::MIN),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[4..12]));
        assert_eq!(iterator.next(), None);

        let mut iterator = SliceIterator::new(
            &attn_0,
            &[
                TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(2), NonZeroUsize::MIN),
                TensorIndexer::Select(0),
            ],
        )
        .unwrap();
        assert_eq!(iterator.next(), Some(&data[12..16]));
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn test_invalid_range() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let attn_0 = TensorView::new(Dtype::F32, vec![2, 3], &data).unwrap();

        assert_eq!(
            SliceIterator::new(
                &attn_0,
                &[
                    TensorIndexer::Select(1),
                    TensorIndexer::Narrow(Bound::Included(1), Bound::Excluded(4), NonZeroUsize::MIN),
                ],
            ),
            Err(InvalidSlice::SliceOutOfRange {
                asked: 3,
                dim_index: 1,
                dim_size: 3,
            })
        );
        assert_eq!(
            SliceIterator::new(
                &attn_0,
                &[
                    TensorIndexer::Select(1),
                    TensorIndexer::Narrow(Bound::Included(3), Bound::Excluded(2), NonZeroUsize::MIN),
                ],
            ),
            Err(InvalidSlice::SliceOutOfRange {
                asked: 3,
                dim_index: 1,
                dim_size: 3,
            })
        );
        assert_eq!(
            SliceIterator::new(
                &attn_0,
                &[
                    TensorIndexer::Select(1),
                    TensorIndexer::Select(1),
                    TensorIndexer::Select(1),
                ],
            ),
            Err(InvalidSlice::TooManySlices)
        );
    }
}
