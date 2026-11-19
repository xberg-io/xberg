#[cfg(feature = "v1_18")]
use std::num::NonZeroU16;
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::Mutex;
use std::{ffi, mem, ptr};

use four_cc::FourCC;
use libheif_sys as lh;

#[cfg(feature = "v1_19")]
use crate::SecurityLimits;
#[cfg(feature = "v1_20")]
use crate::Track;
use crate::encoder::get_encoding_options_ptr;
use crate::reader::{HEIF_READER, Reader, SharedReader};
use crate::utils::str_to_cstring;
use crate::{Encoder, EncodingOptions, HeifError, HeifErrorCode, HeifErrorSubCode, Image, ImageHandle, ItemId, Result};

#[allow(dead_code)]
enum Source<'a> {
    None,
    File,
    Memory(&'a [u8]),
    Reader(Box<SharedReader<'a>>),
}

/// The native context together with the input backing its native state.
///
/// Shared by the `HeifContext` and by every handle or track derived from it, so the native
/// context is freed, and the backing released, only after the last of them is dropped.
pub(crate) struct ContextOwner<'a> {
    ptr: *mut lh::heif_context,
    source: Source<'a>,
}

impl ContextOwner<'_> {
    #[cfg_attr(not(feature = "v1_18"), allow(dead_code))]
    pub(crate) fn ptr(&self) -> *mut lh::heif_context {
        self.ptr
    }
}

#[allow(unsafe_code)]
impl Drop for ContextOwner<'_> {
    fn drop(&mut self) {
        // SAFETY: `ptr` is the context allocated by `HeifContext::new` and owned only by this
        // value; nothing else frees it. The native context is released before the fields drop,
        // so the source (reader or borrowed input) outlives it. Every wrapper that can keep
        // native context state alive (`HeifContext`, `ImageHandle`, `Track`, `RegionItem`)
        // holds an `Rc` of this owner, so the release happens only after the last of them.
        unsafe { lh::heif_context_free(self.ptr) };
    }
}

/// A HEIF container.
///
/// The lifetime `'a` is that of the borrowed input (see [`HeifContext::read_from_bytes`]).
/// Image handles and tracks created from a context share ownership of its native state and
/// input backing, so they stay valid after the context itself is dropped, but cannot outlive
/// the borrowed input:
///
/// ```no_run
/// use xberg_libheif::HeifContext;
///
/// let bytes = std::fs::read("image.heic").unwrap();
/// let handle = {
///     let context = HeifContext::read_from_bytes(&bytes).unwrap();
///     context.primary_image_handle().unwrap()
/// };
/// println!("{}", handle.width());
/// ```
///
/// ```compile_fail
/// use xberg_libheif::HeifContext;
///
/// let handle = {
///     let bytes = std::fs::read("image.heic").unwrap();
///     let context = HeifContext::read_from_bytes(&bytes).unwrap();
///     context.primary_image_handle().unwrap()
/// };
/// println!("{}", handle.width());
/// ```
///
/// A context is not `Send`: its handles are not either, and both address the same
/// unsynchronized native state.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<xberg_libheif::HeifContext<'static>>();
/// ```
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<xberg_libheif::ImageHandle<'static>>();
/// ```
pub struct HeifContext<'a> {
    pub(crate) inner: *mut lh::heif_context,
    owner: Rc<ContextOwner<'a>>,
}

#[allow(unsafe_code)]
impl HeifContext<'static> {
    /// Create a new context from a file.
    pub fn read_from_file(name: &str) -> Result<HeifContext<'static>> {
        let mut context = HeifContext::new()?;
        context.read_file(name)?;
        Ok(context)
    }
}

#[allow(unsafe_code)]
impl<'a> HeifContext<'a> {
    /// Create a new empty context.
    ///
    /// The lifetime is chosen by the caller: the context is invariant over it, and a
    /// reader or input slice later installed must outlive it.
    pub fn new() -> Result<HeifContext<'a>> {
        // SAFETY: Allocation takes no caller-owned pointers. A non-null result is owned by the
        // `ContextOwner` created below, which frees it exactly once.
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
                owner: Rc::new(ContextOwner {
                    ptr: ctx,
                    source: Source::None,
                }),
            })
        }
    }

    /// The source slot, available only while no handle, track or region item shares this context.
    ///
    /// Replacing the input under a live handle would leave the handle's native state reading
    /// from a different source than the one it was parsed from.
    fn source_mut(&mut self) -> Result<&mut Source<'a>> {
        Rc::get_mut(&mut self.owner)
            .map(|owner| &mut owner.source)
            .ok_or_else(|| HeifError {
                code: HeifErrorCode::UsageError,
                sub_code: HeifErrorSubCode::Unspecified,
                message: String::from("the context cannot be read again while image handles or tracks are alive"),
            })
    }

    fn check_owns(&self, handle: &ImageHandle<'_>) -> Result<()> {
        // Compared as addresses: the owners' lifetime parameters are invariant and need not match.
        if ptr::eq(
            Rc::as_ptr(&self.owner).cast::<()>(),
            Rc::as_ptr(&handle.context).cast::<()>(),
        ) {
            Ok(())
        } else {
            Err(HeifError {
                code: HeifErrorCode::UsageError,
                sub_code: HeifErrorSubCode::InvalidParameterValue,
                message: String::from("the image handle belongs to a different context"),
            })
        }
    }

    /// Read a HEIF file from a named disk file.
    #[allow(unsafe_code)]
    pub fn read_file(&mut self, name: &str) -> Result<()> {
        let inner = self.inner;
        let c_name = str_to_cstring(name, "name")?;
        *self.source_mut()? = Source::File;
        // SAFETY: The context is exclusively borrowed and the NUL-terminated filename lives
        // through this call. Null requests default read options.
        let err = unsafe { lh::heif_context_read_from_file(inner, c_name.as_ptr(), ptr::null()) };
        HeifError::from_heif_error(err)?;
        Ok(())
    }

    /// Read a HEIF file from the reader.
    ///
    /// The context takes ownership of the reader and keeps it until the context and every
    /// handle derived from it are dropped, including after an error: libheif keeps its
    /// adapter over the reader even when parsing fails.
    #[allow(unsafe_code)]
    pub fn read_reader(&mut self, reader: Box<dyn Reader + 'a>) -> Result<()> {
        let inner = self.inner;
        let source = self.source_mut()?;
        let previous_source = mem::replace(source, Source::Reader(Box::new(Mutex::new(reader))));
        let Source::Reader(shared) = source else {
            unreachable!("reader source just installed");
        };
        let user_data = &**shared as *const SharedReader<'a> as *mut c_void;
        // SAFETY: `HEIF_READER` is a static callback table. `user_data` addresses the
        // `SharedReader` heap allocation just stored in the context's backing, which the native
        // context (freed in `ContextOwner::drop` before the source) and its handles never
        // outlive, on success and on error alike. The callbacks only take a shared reference
        // and lock the mutex. The context is exclusively borrowed, so no handle exists.
        let err = unsafe { lh::heif_context_read_from_reader(inner, &HEIF_READER, user_data, ptr::null()) };
        // Copy the context-borrowed message before the previous reader can run its destructor.
        let result = HeifError::from_heif_error(err);
        drop(previous_source);
        result
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
        let inner = self.inner;
        *self.source_mut()? = Source::Memory(bytes);
        // SAFETY: libheif keeps reading `bytes` without copying, for as long as the native
        // context lives. The slice outlives `'a`, and every handle and track carries `'a` and
        // shares the owner, which frees the native context before `'a` can end.
        let err = unsafe {
            lh::heif_context_read_from_memory_without_copy(inner, bytes.as_ptr() as _, bytes.len(), ptr::null())
        };
        HeifError::from_heif_error(err)?;
        Ok(())
    }

    /// # Safety
    ///
    /// The native writer supplies a readable byte buffer and calls synchronously with
    /// the exclusively borrowed output vector that `write_to_bytes` passed as userdata.
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
        // SAFETY: heif_context_write supplies the serialized HEIF box bytes from its own
        // live StreamWriter_memory buffer. Reserve supplies separate writable Rust storage.
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
    pub fn image_handle(&self, item_id: ItemId) -> Result<ImageHandle<'a>> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: The context stays live and handle is a writable out slot. On success the
        // ImageHandle wrapper owns the returned native handle.
        let err = unsafe { lh::heif_context_get_image_handle(self.inner, item_id, &mut handle) };
        HeifError::from_heif_error(err)?;
        Ok(ImageHandle::new(handle, &self.owner))
    }

    /// Get a handle to the primary image of the HEIF file.
    ///
    /// This is the image that should be displayed primarily
    /// when there are several images in the file.
    #[allow(unsafe_code)]
    pub fn primary_image_handle(&self) -> Result<ImageHandle<'a>> {
        let mut handle: *mut lh::heif_image_handle = ptr::null_mut();
        // SAFETY: The context stays live and handle is a writable out slot. On success the
        // ImageHandle wrapper owns the returned native handle.
        let err = unsafe { lh::heif_context_get_primary_image_handle(self.inner, &mut handle) };
        HeifError::from_heif_error(err)?;
        Ok(ImageHandle::new(handle, &self.owner))
    }

    /// Returns a vector with top level image handles.
    pub fn top_level_image_handles(&self) -> Vec<ImageHandle<'a>> {
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
    ) -> Result<ImageHandle<'a>> {
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
        Ok(ImageHandle::new(handle, &self.owner))
    }

    /// Encode the `image` as a scaled down thumbnail image.
    ///
    /// The image is scaled down to fit into a square area of width `bbox_size`.
    /// Returns `Ok(None)` when libheif succeeds without returning a thumbnail handle.
    /// Errors reported by libheif are returned to the caller.
    ///
    /// The encoded thumbnail is automatically assigned to the
    /// `master_image_handle`. Hence, you do not have to call
    /// [`HeifContext::assign_thumbnail()`] method.
    #[allow(unsafe_code)]
    pub fn encode_thumbnail(
        &mut self,
        image: &Image,
        master_image_handle: &ImageHandle<'_>,
        bbox_size: u32,
        encoder: &mut Encoder,
        encoding_options: Option<EncodingOptions>,
    ) -> Result<Option<ImageHandle<'a>>> {
        self.check_owns(master_image_handle)?;
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
        Ok(thumbnail_handle(handle, &self.owner))
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
    ) -> Result<Option<ImageHandle<'a>>> {
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
        Ok(Some(ImageHandle::new(handle, &self.owner)))
    }

    /// Assign `master_image_handle` as the thumbnail image of `thumbnail_image_handle`.
    #[allow(unsafe_code)]
    pub fn assign_thumbnail(
        &mut self,
        master_image_handle: &ImageHandle<'_>,
        thumbnail_image_handle: &ImageHandle<'_>,
    ) -> Result<()> {
        self.check_owns(master_image_handle)?;
        self.check_owns(thumbnail_image_handle)?;
        // SAFETY: The referenced handles stay live during this context mutation.
        unsafe {
            let err =
                lh::heif_context_assign_thumbnail(self.inner, master_image_handle.inner, thumbnail_image_handle.inner);
            HeifError::from_heif_error(err)
        }
    }

    #[allow(unsafe_code)]
    pub fn set_primary_image(&mut self, image_handle: &mut ImageHandle<'_>) -> Result<()> {
        self.check_owns(image_handle)?;
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
        image_handle: &ImageHandle<'_>,
        data: &[u8],
        item_type: T,
        content_type: Option<&str>,
    ) -> Result<()>
    where
        T: Into<FourCC>,
    {
        self.check_owns(image_handle)?;
        let c_item_type = str_to_cstring(&item_type.into().to_string(), "item_type")?;
        let c_content_type = match content_type {
            Some(s) => Some(str_to_cstring(s, "content_type")?),
            None => None,
        };
        let c_content_type_ptr = c_content_type.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null());
        // Both C strings and the data remain live through the call. ~keep
        // TODO(#3172): Check that the byte count fits libheif's signed int.
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
    pub fn add_exif_metadata(&mut self, master_image: &ImageHandle<'_>, data: &[u8]) -> Result<()> {
        self.check_owns(master_image)?;
        // SAFETY requirement: The context and image handle are live and data is borrowed
        // for the copy. The conversion assumes its byte count fits libheif's signed int.
        let error = unsafe {
            lh::heif_context_add_exif_metadata(self.inner, master_image.inner, data.as_ptr() as _, data.len() as _)
        };
        HeifError::from_heif_error(error)
    }

    /// Add XMP metadata to an image.
    #[allow(unsafe_code)]
    pub fn add_xmp_metadata(&mut self, master_image: &ImageHandle<'_>, data: &[u8]) -> Result<()> {
        self.check_owns(master_image)?;
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
    pub fn track(&self, id: u32) -> Option<Track<'a>> {
        // SAFETY: The context stays live for lookup. Null means no track. A non-null result
        // is transferred to Track, which frees the native track and shares this context's owner.
        unsafe {
            let heif_track = lh::heif_context_get_track(self.inner, id);
            if heif_track.is_null() {
                None
            } else {
                Some(Track::from_heif_track(heif_track, &self.owner))
            }
        }
    }
}

fn thumbnail_handle<'a>(handle: *mut lh::heif_image_handle, owner: &Rc<ContextOwner<'a>>) -> Option<ImageHandle<'a>> {
    ptr::NonNull::new(handle).map(|handle| ImageHandle::new(handle.as_ptr(), owner))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::ReaderGrowStatus;

    struct DropCounting(Arc<AtomicUsize>);

    impl Drop for DropCounting {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Reader for DropCounting {
        fn position(&mut self) -> u64 {
            0
        }

        #[allow(deprecated)]
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::UnexpectedEof.into())
        }

        fn seek(&mut self, position: u64) -> io::Result<u64> {
            Ok(position)
        }

        fn wait_for_file_size(&mut self, _target_size: u64) -> ReaderGrowStatus {
            ReaderGrowStatus::SizeBeyondEof
        }
    }

    #[test]
    fn a_successful_thumbnail_without_a_native_handle_is_none() {
        let context = HeifContext::new().expect("allocate HEIF context");
        assert!(thumbnail_handle(ptr::null_mut(), &context.owner).is_none());
    }

    #[test]
    fn read_reader_retains_its_source_after_a_native_error() {
        let mut context = HeifContext::new().expect("allocate HEIF context");
        let reader = crate::StreamReader::new(std::io::Cursor::new([0_u8; 0]), 0);
        assert!(context.read_reader(Box::new(reader)).is_err());
        assert!(matches!(&context.owner.source, Source::Reader(_)));
    }

    #[test]
    fn a_reader_is_released_exactly_once_after_a_failed_read() {
        let drops = Arc::new(AtomicUsize::new(0));
        let result = HeifContext::read_from_reader(Box::new(DropCounting(Arc::clone(&drops))));
        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failed_read_keeps_the_reader_until_the_context_is_dropped() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut context = HeifContext::new().expect("allocate HEIF context");
        assert!(context.read_reader(Box::new(DropCounting(Arc::clone(&drops)))).is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(context);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_replaced_reader_is_released_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut context = HeifContext::new().expect("allocate HEIF context");
        assert!(context.read_reader(Box::new(DropCounting(Arc::clone(&drops)))).is_err());
        assert!(context.read_reader(Box::new(DropCounting(Arc::clone(&drops)))).is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(context);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn the_source_is_locked_while_another_owner_shares_the_context() {
        let mut context = HeifContext::new().expect("allocate HEIF context");
        let other_owner = Rc::clone(&context.owner);
        let error = context
            .read_bytes(b"x")
            .expect_err("shared context must refuse a new source");
        assert!(matches!(error.code, HeifErrorCode::UsageError));
        drop(other_owner);
        assert!(context.read_bytes(b"x").is_err());
        assert_eq!(Rc::strong_count(&context.owner), 1);
    }
}
