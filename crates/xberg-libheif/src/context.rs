#[cfg(feature = "v1_18")]
use std::num::NonZeroU16;
use std::os::raw::c_void;
use std::{ffi, ptr};

use four_cc::FourCC;
use libheif_sys as lh;

#[cfg(feature = "v1_19")]
use crate::SecurityLimits;
#[cfg(feature = "v1_20")]
use crate::Track;
use crate::encoder::get_encoding_options_ptr;
use crate::reader::{HEIF_READER, Reader};
use crate::utils::str_to_cstring;
use crate::{Encoder, EncodingOptions, HeifError, HeifErrorCode, HeifErrorSubCode, Image, ImageHandle, ItemId, Result};

#[allow(dead_code)]
enum Source<'a> {
    None,
    File,
    Memory(&'a [u8]),
    Reader(Box<Box<dyn Reader + 'a>>),
}

pub struct HeifContext<'a> {
    pub(crate) inner: *mut lh::heif_context,
    source: Source<'a>,
}

#[allow(unsafe_code)]
impl HeifContext<'static> {
    /// Create a new empty context.
    pub fn new() -> Result<HeifContext<'static>> {
        // SAFETY: Allocation takes no caller-owned pointers. A non-null result is owned by this context.
        let ctx = unsafe { lh::heif_context_alloc() };
        if ctx.is_null() {
            Err(HeifError {
                code: HeifErrorCode::ContextCreateFailed,
                sub_code: HeifErrorSubCode::Unspecified,
                message: String::from(""),
            })
        } else {
            Ok(HeifContext {
                inner: ctx,
                source: Source::None,
            })
        }
    }

    /// Create a new context from a file.
    pub fn read_from_file(name: &str) -> Result<HeifContext<'static>> {
        let mut context = HeifContext::new()?;
        context.read_file(name)?;
        Ok(context)
    }

    /// # Safety
    ///
    /// The pointer must be non-null, live, and independently releasable with
    /// `heif_context_free`. The caller must keep any backing Rust reader or
    /// borrowed input alive for the native context since this wrapper stores no source.
    #[cfg(feature = "v1_18")]
    #[allow(unsafe_code)]
    pub(crate) unsafe fn from_ptr(ctx: *mut lh::heif_context) -> HeifContext<'static> {
        HeifContext {
            inner: ctx,
            source: Source::None,
        }
    }
}

#[allow(unsafe_code)]
impl<'a> HeifContext<'a> {
    /// Read a HEIF file from a named disk file.
    #[allow(unsafe_code)]
    pub fn read_file(&mut self, name: &str) -> Result<()> {
        self.source = Source::File;
        let c_name = ffi::CString::new(name).unwrap();
        // SAFETY: The context is exclusively borrowed and the NUL-terminated filename lives
        // through this call. Null requests default read options.
        let err = unsafe { lh::heif_context_read_from_file(self.inner, c_name.as_ptr(), ptr::null()) };
        HeifError::from_heif_error(err)?;
        Ok(())
    }

    /// Read a HEIF file from the reader.
    #[allow(unsafe_code)]
    pub fn read_reader(&mut self, reader: Box<dyn Reader + 'a>) -> Result<()> {
        let mut reader_box = Box::new(reader);
        let user_data = reader_box.as_mut() as *mut _ as *mut c_void;
        // SAFETY: The callback table is static and user_data points to the inner Box held in
        // a stable allocation. The outer Box is retained in source after a successful read;
        // the failure path assumes libheif will no longer call this reader.
        let err = unsafe { lh::heif_context_read_from_reader(self.inner, &HEIF_READER, user_data, ptr::null()) };
        HeifError::from_heif_error(err)?;
        self.source = Source::Reader(reader_box);
        Ok(())
    }

    /// Create a new context from the reader.
    pub fn read_from_reader(reader: Box<dyn Reader + 'a>) -> Result<HeifContext<'a>> {
        let mut context = HeifContext::new()?;
        context.read_reader(reader)?;
        Ok(context)
    }

    /// Create a new context from bytes.
    ///
    /// The provided memory buffer is not copied.
    /// That means you will have to keep the memory buffer alive as
    /// long as you use the context.
    pub fn read_from_bytes(bytes: &[u8]) -> Result<HeifContext<'_>> {
        let mut context = HeifContext::new()?;
        context.read_bytes(bytes)?;
        Ok(context)
    }

    /// Read a HEIF file from bytes.
    ///
    /// The provided memory buffer is not copied.
    /// That means you will have to keep the memory buffer alive as
    /// long as you use the context.
    #[allow(unsafe_code)]
    pub fn read_bytes<'b: 'a>(&mut self, bytes: &'b [u8]) -> Result<()> {
        self.source = Source::Memory(bytes);
        // SAFETY: The slice supplies bytes.len() readable bytes and Source::Memory keeps it
        // borrowed while registered here. Native handles that outlive this source still need
        // the backing bytes to remain valid.
        let err = unsafe {
            lh::heif_context_read_from_memory_without_copy(self.inner, bytes.as_ptr() as _, bytes.len(), ptr::null())
        };
        HeifError::from_heif_error(err)?;
        Ok(())
    }

    #[allow(unsafe_code)]
    unsafe extern "C" fn vector_writer(
        _ctx: *mut lh::heif_context,
        data: *const c_void,
        size: usize,
        user_data: *mut c_void,
    ) -> lh::heif_error {
        // SAFETY: write_to_bytes passes its exclusively borrowed Vec as user_data for the
        // synchronous writer callback.
        let vec: &mut Vec<u8> = unsafe { &mut *(user_data as *mut Vec<u8>) };
        vec.reserve(size);
        // SAFETY: The native callback must supply a non-null buffer readable for size bytes,
        // even for an empty copy. Reserve makes size bytes available in the separate Vec.
        unsafe { ptr::copy_nonoverlapping::<u8>(data as _, vec.as_mut_ptr(), size) };
        // SAFETY: The copy initialized every byte in 0..size within the reserved capacity.
        unsafe { vec.set_len(size) };

        lh::heif_error {
            code: lh::heif_error_code_heif_error_Ok,
            subcode: lh::heif_suberror_code_heif_suberror_Unspecified,
            message: c"".as_ptr() as _,
        }
    }

    #[allow(unsafe_code)]
    pub fn write_to_bytes(&self) -> Result<Vec<u8>> {
        let mut res = Vec::<u8>::new();
        let pointer_to_res = &mut res as *mut _ as *mut c_void;

        let mut writer = lh::heif_writer {
            writer_api_version: 1,
            write: Some(Self::vector_writer),
        };

        // SAFETY: The context is live. The C writer invokes its callback during this call;
        // the versioned table and the exclusively borrowed result remain live until it returns.
        let err = unsafe { lh::heif_context_write(self.inner, &mut writer, pointer_to_res) };
        HeifError::from_heif_error(err)?;
        Ok(res)
    }

    #[allow(unsafe_code)]
    pub fn write_to_file(&self, name: &str) -> Result<()> {
        let c_name = ffi::CString::new(name).unwrap();
        // SAFETY: The context stays live and c_name supplies a NUL-terminated path during the call.
        let err = unsafe { lh::heif_context_write_to_file(self.inner, c_name.as_ptr()) };
        HeifError::from_heif_error(err)
    }

    /// Number of top-level images in the HEIF file.
    ///
    /// This does not include the thumbnails or the tile images that
    /// are composed to an image grid.
    /// You can get access to the thumbnails via the main image handle.
    #[deprecated(since = "2.7.0", note = "use 'image_ids' method instead.")]
    #[allow(unsafe_code)]
    pub fn number_of_top_level_images(&self) -> usize {
        // SAFETY: This query reads the live context without writing through Rust pointers.
        unsafe { lh::heif_context_get_number_of_top_level_images(self.inner) as _ }
    }

    /// Fills in image IDs into the user-supplied preallocated array 'ID_array'.
    ///
    /// Method returns the total number of IDs filled into the array.
    #[deprecated(since = "2.7.0", note = "use 'image_ids' method instead.")]
    #[allow(unsafe_code)]
    pub fn top_level_image_ids(&self, item_ids: &mut [ItemId]) -> usize {
        if item_ids.is_empty() {
            0
        } else {
            // SAFETY: The exclusive slice supplies storage for the advertised entry count.
            unsafe {
                lh::heif_context_get_list_of_top_level_image_IDs(self.inner, item_ids.as_mut_ptr(), item_ids.len() as _)
                    as usize
            }
        }
    }

    /// Returns a vector with top level image IDs.
    #[allow(unsafe_code)]
    pub fn image_ids(&self) -> Vec<ItemId> {
        // SAFETY: The count query takes only the live context.
        let count = unsafe { lh::heif_context_get_number_of_top_level_images(self.inner) as usize };
        let mut item_ids = vec![0; count];
        // SAFETY: The Vec has count writable entries, the bound passed to libheif.
        let real_count = unsafe {
            lh::heif_context_get_list_of_top_level_image_IDs(self.inner, item_ids.as_mut_ptr(), count as _) as usize
        };
        item_ids.truncate(real_count);
        item_ids
    }

    /// Get the image handle for a known image ID.
    #[allow(unsafe_code)]
    pub fn image_handle(&self, item_id: ItemId) -> Result<ImageHandle> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: The context stays live and handle is a writable out slot. On success the
        // ImageHandle wrapper owns the returned native handle.
        let err = unsafe { lh::heif_context_get_image_handle(self.inner, item_id, &mut handle) };
        HeifError::from_heif_error(err)?;
        Ok(ImageHandle::new(handle))
    }

    /// Get a handle to the primary image of the HEIF file.
    ///
    /// This is the image that should be displayed primarily
    /// when there are several images in the file.
    #[allow(unsafe_code)]
    pub fn primary_image_handle(&self) -> Result<ImageHandle> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: The context stays live and handle is a writable out slot. On success the
        // ImageHandle wrapper owns the returned native handle.
        let err = unsafe { lh::heif_context_get_primary_image_handle(self.inner, &mut handle) };
        HeifError::from_heif_error(err)?;
        Ok(ImageHandle::new(handle))
    }

    /// Returns a vector with top level image handles.
    pub fn top_level_image_handles(&self) -> Vec<ImageHandle> {
        let item_ids = self.image_ids();
        let mut handles = Vec::with_capacity(item_ids.len());
        for item_id in item_ids {
            if let Ok(handle) = self.image_handle(item_id) {
                handles.push(handle);
            }
        }
        handles
    }

    /// Compress the input image.
    ///
    /// The first image added to the context is also automatically set as the primary image, but
    /// you can change the primary image later with [`HeifContext::set_primary_image`] method.
    #[allow(unsafe_code)]
    pub fn encode_image(
        &mut self,
        image: &Image,
        encoder: &mut Encoder,
        encoding_options: Option<EncodingOptions>,
    ) -> Result<ImageHandle> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: The context, image, encoder, and optional settings live through encoding.
        // The writable handle slot receives an owned image handle on success.
        unsafe {
            let err = lh::heif_context_encode_image(
                self.inner,
                image.inner,
                encoder.inner,
                get_encoding_options_ptr(&encoding_options),
                &mut handle,
            );
            HeifError::from_heif_error(err)?;
        }
        Ok(ImageHandle::new(handle))
    }

    /// Encode the `image` as a scaled down thumbnail image.
    ///
    /// The image is scaled down to fit into a square area of width `bbox_size`.
    /// If the input image is already so small that it fits into this bounding
    /// box, no thumbnail image is encoded and `Ok(None)` is returned.
    /// No error is returned in this case.
    ///
    /// The encoded thumbnail is automatically assigned to the
    /// `master_image_handle`. Hence, you do not have to call
    /// [`HeifContext::assign_thumbnail()`] method.
    #[allow(unsafe_code)]
    pub fn encode_thumbnail(
        &mut self,
        image: &Image,
        master_image_handle: &ImageHandle,
        bbox_size: u32,
        encoder: &mut Encoder,
        encoding_options: Option<EncodingOptions>,
    ) -> Result<Option<ImageHandle>> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: Both source handles, the encoder and optional settings remain live for
        // the call. The writable slot receives libheif's nullable thumbnail result.
        unsafe {
            let err = lh::heif_context_encode_thumbnail(
                self.inner,
                image.inner,
                master_image_handle.inner,
                encoder.inner,
                get_encoding_options_ptr(&encoding_options),
                bbox_size.min(i32::MAX as _) as _,
                &mut handle,
            );
            HeifError::from_heif_error(err)?;
        }
        Ok(Some(ImageHandle::new(handle)))
    }

    /// Encodes an array of images into a grid.
    ///
    /// # Arguments
    ///
    /// * `tiles` - User allocated array of images that will form the grid.
    /// * `rows` - The number of rows in the grid. The number of columns will
    ///   be calculated from the size of `tiles`.
    /// * `encoder` - Defines the encoder to use.
    ///   See [LibHeif::encoder_for_format()](crate::LibHeif::encoder_for_format).
    /// * `encoding_options` - Optional, may be None.
    ///
    /// Returns an error if `tiles` slice is empty.
    #[cfg(feature = "v1_18")]
    #[allow(unsafe_code)]
    pub fn encode_grid(
        &mut self,
        tiles: &[Image],
        rows: NonZeroU16,
        encoder: &mut Encoder,
        encoding_options: Option<EncodingOptions>,
    ) -> Result<Option<ImageHandle>> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        let mut tiles_inners: Vec<*mut lh::heif_image> = tiles.iter().map(|img| img.inner).collect();
        let rows = rows.get();
        let columns = (tiles_inners.len() as u32 / rows as u32).min(u16::MAX as _) as u16;
        // SAFETY: rows * columns fits the pointer array; libheif rejects a zero dimension.
        // The tiles, encoder and optional settings stay live and handle is a writable out slot.
        unsafe {
            let err = lh::heif_context_encode_grid(
                self.inner,
                tiles_inners.as_mut_ptr(),
                rows,
                columns,
                encoder.inner,
                get_encoding_options_ptr(&encoding_options),
                &mut handle,
            );
            HeifError::from_heif_error(err)?;
        }
        Ok(Some(ImageHandle::new(handle)))
    }

    /// Assign `master_image_handle` as the thumbnail image of `thumbnail_image_handle`.
    #[allow(unsafe_code)]
    pub fn assign_thumbnail(
        &mut self,
        master_image_handle: &ImageHandle,
        thumbnail_image_handle: &ImageHandle,
    ) -> Result<()> {
        // SAFETY: The referenced handles stay live during this context mutation.
        unsafe {
            let err =
                lh::heif_context_assign_thumbnail(self.inner, master_image_handle.inner, thumbnail_image_handle.inner);
            HeifError::from_heif_error(err)
        }
    }

    #[allow(unsafe_code)]
    pub fn set_primary_image(&mut self, image_handle: &mut ImageHandle) -> Result<()> {
        // SAFETY: The context is exclusively borrowed and the image handle remains live.
        unsafe {
            let err = lh::heif_context_set_primary_image(self.inner, image_handle.inner);
            HeifError::from_heif_error(err)
        }
    }

    /// Add generic, proprietary metadata to an image.
    ///
    /// You have to specify an `item_type` that will identify your metadata.
    /// `content_type` can be an additional type.
    ///
    /// For example, this function can be used to add IPTC metadata
    /// (IIM stream, not XMP) to an image. Although not standard, we propose
    /// to store IPTC data with `item_type=b"iptc"` and `content_type=None`.
    #[allow(unsafe_code)]
    pub fn add_generic_metadata<T>(
        &mut self,
        image_handle: &ImageHandle,
        data: &[u8],
        item_type: T,
        content_type: Option<&str>,
    ) -> Result<()>
    where
        T: Into<FourCC>,
    {
        let c_item_type = str_to_cstring(&item_type.into().to_string(), "item_type")?;
        let c_content_type = match content_type {
            Some(s) => Some(str_to_cstring(s, "content_type")?),
            None => None,
        };
        let c_content_type_ptr = c_content_type.map(|s| s.as_ptr()).unwrap_or(ptr::null());
        // SAFETY requirement: libheif reads both C strings during the call. The item type
        // and data are live, but the consuming map drops a present content-type CString
        // before this call. The byte count also needs to fit libheif's signed int.
        let error = unsafe {
            lh::heif_context_add_generic_metadata(
                self.inner,
                image_handle.inner,
                data.as_ptr() as _,
                data.len() as _,
                c_item_type.as_ptr(),
                c_content_type_ptr,
            )
        };
        HeifError::from_heif_error(error)
    }

    /// Add EXIF metadata to an image.
    #[allow(unsafe_code)]
    pub fn add_exif_metadata(&mut self, master_image: &ImageHandle, data: &[u8]) -> Result<()> {
        // SAFETY requirement: The context and image handle are live and data is borrowed
        // for the copy. The conversion assumes its byte count fits libheif's signed int.
        let error = unsafe {
            lh::heif_context_add_exif_metadata(self.inner, master_image.inner, data.as_ptr() as _, data.len() as _)
        };
        HeifError::from_heif_error(error)
    }

    /// Add XMP metadata to an image.
    #[allow(unsafe_code)]
    pub fn add_xmp_metadata(&mut self, master_image: &ImageHandle, data: &[u8]) -> Result<()> {
        // SAFETY requirement: The context and image handle are live and data is borrowed
        // for the copy. The conversion assumes its byte count fits libheif's signed int.
        let error = unsafe {
            lh::heif_context_add_XMP_metadata(self.inner, master_image.inner, data.as_ptr() as _, data.len() as _)
        };
        HeifError::from_heif_error(error)
    }

    /// If the maximum threads number is set to 0, the image tiles are
    /// decoded in the main thread. This is different from setting it to 1,
    /// which will generate a single background thread to decode the tiles.
    ///
    /// Note that this setting only affects `libheif` itself. The codecs itself
    /// may still use multi-threaded decoding. You can use it, for example,
    /// in cases where you are decoding several images in parallel anyway you
    /// thus want to minimize parallelism in each decoder.
    #[allow(unsafe_code)]
    pub fn set_max_decoding_threads(&mut self, max_threads: u32) {
        let max_threads = max_threads.min(libc::c_int::MAX as u32) as libc::c_int;
        // SAFETY: The context is exclusively borrowed and the count fits the C int parameter.
        unsafe { lh::heif_context_set_max_decoding_threads(self.inner, max_threads) };
    }

    /// Returns the security limits for a context.
    ///
    /// By default, the limits are set to the global limits,
    /// but you can change them with the help of [`HeifContext::set_security_limits()`] method.
    #[cfg(feature = "v1_19")]
    #[allow(unsafe_code)]
    pub fn security_limits(&self) -> SecurityLimits {
        // SAFETY: The context stays live while its borrowed limits are copied by value.
        let inner_ptr = unsafe { lh::heif_context_get_security_limits(self.inner) };
        let inner = ptr::NonNull::new(inner_ptr).unwrap();
        SecurityLimits::from_inner(inner)
    }

    /// Overwrites the security limits of a context.
    #[cfg(feature = "v1_19")]
    #[allow(unsafe_code)]
    pub fn set_security_limits(&mut self, limits: &SecurityLimits) -> Result<()> {
        // SAFETY: The context is exclusively borrowed and libheif copies the live limits value.
        let err = unsafe { lh::heif_context_set_security_limits(self.inner, limits.as_inner()) };
        HeifError::from_heif_error(err)
    }

    /// Check whether there is an image sequence in the HEIF file.
    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn has_sequence(&self) -> bool {
        // SAFETY: This query only borrows the live context.
        unsafe { lh::heif_context_has_sequence(self.inner) != 0 }
    }

    /// Get the timescale (clock ticks per second) for timing values in the sequence.
    ///
    /// Each track may have its independent timescale.
    ///
    /// Returns 0 if there is no sequence in the file.
    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn sequence_timescale(&self) -> u32 {
        // SAFETY: This query only borrows the live context.
        unsafe { lh::heif_context_get_sequence_timescale(self.inner) }
    }

    /// Get the total duration of the sequence in timescale clock ticks.
    ///
    /// Use [sequence_timescale] method to get the clock ticks per second.
    ///
    /// Returns 0 if there is no sequence in the file.
    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn sequence_duration(&self) -> u64 {
        // SAFETY: This query only borrows the live context.
        unsafe { lh::heif_context_get_sequence_duration(self.inner) }
    }

    /// Returns a vector with IDs for each of the tracks stored in the HEIF file.
    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn track_ids(&self) -> Vec<u32> {
        // SAFETY: The count query takes only the live context.
        let num_tracks = unsafe { lh::heif_context_number_of_sequence_tracks(self.inner) as usize };
        let mut track_ids = vec![0u32; num_tracks];

        // SAFETY: The Vec fits the count queried above. The C fill has no capacity parameter,
        // so the track list must remain unchanged until it returns.
        unsafe {
            lh::heif_context_get_track_ids(self.inner, track_ids.as_mut_ptr() as *mut [u32; 0]);
        }

        track_ids
    }

    /// Get the [Track] object for the given track ID.
    ///
    /// If you pass id=0, the first visual track will be returned.
    /// If there is no track with the given ID or if 0 is passed and
    /// there is no visual track, `None` will be returned.
    ///
    /// Tracks never have a zero ID.
    /// This is why we can use this as a special value to find the first visual track.
    #[cfg(feature = "v1_20")]
    #[allow(unsafe_code)]
    pub fn track(&self, id: u32) -> Option<Track> {
        // SAFETY: The context stays live for lookup. Null means no track. A non-null result
        // is transferred to Track, which frees the native track.
        unsafe {
            let heif_track = lh::heif_context_get_track(self.inner, id);
            if heif_track.is_null() {
                None
            } else {
                Some(Track::from_heif_track(heif_track))
            }
        }
    }
}

#[allow(unsafe_code)]
impl Drop for HeifContext<'_> {
    fn drop(&mut self) {
        // SAFETY: This wrapper owns one releasable context pointer. Its registered source is
        // still alive during the native free because Rust drops the fields after Drop returns.
        unsafe { lh::heif_context_free(self.inner) };
    }
}

#[allow(unsafe_code)]
// SAFETY requirement: moving the context also moves its backing source. Source::Reader stores
// a dyn Reader without a Send bound, so the current type does not uphold this for all sources.
unsafe impl Send for HeifContext<'_> {}
