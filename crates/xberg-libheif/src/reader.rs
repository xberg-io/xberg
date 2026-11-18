use std::os::raw::{c_int, c_void};
use std::{io, slice};

use libheif_sys as lh;

use crate::enums::ReaderGrowStatus;

pub trait Reader {
    /// Current position, in bytes, inside a source.
    fn position(&mut self) -> u64;

    /// Pull some bytes from a source into the specified buffer, returning
    /// how many bytes were read.
    #[deprecated(since = "2.4.0", note = "use 'read_exact' method instead.")]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Pull bytes from a source into the specified buffer.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        loop {
            #[allow(deprecated)]
            let size = self.read(buf)?;
            if size == buf.len() {
                return Ok(());
            }
            buf = &mut buf[size..];
        }
    }

    /// Seek to a position, in bytes, from the start of a source.
    fn seek(&mut self, position: u64) -> io::Result<u64>;

    /// Wait until a source will be ready to read bytes to
    /// the specified position.
    ///
    /// When calling this function, `libheif` wants to make sure that it can read the file
    /// up to `target_size`.
    /// This is useful when the file is currently downloaded and may
    /// grow with time.
    /// You may, for example, extract the image sizes even before the actual
    /// compressed image data has been completely downloaded.
    ///
    /// Even if your input files do not grow, you will have to implement at least
    /// detection whether the `target_size` is above the (fixed) file length
    /// (in this case, return 'ReaderGrowStatus::SizeBeyondEof').
    fn wait_for_file_size(&mut self, target_size: u64) -> ReaderGrowStatus;
}

#[derive(Debug)]
pub struct StreamReader<T>
where
    T: io::Read + io::Seek,
{
    stream: T,
    total_size: u64,
}

impl<T> StreamReader<T>
where
    T: io::Read + io::Seek,
{
    #[allow(unsafe_code)]
    pub fn new(stream: T, total_size: u64) -> StreamReader<T> {
        StreamReader { stream, total_size }
    }
}

impl<T> Reader for StreamReader<T>
where
    T: io::Read + io::Seek,
{
    #[allow(unsafe_code)]
    fn position(&mut self) -> u64 {
        self.stream.stream_position().unwrap_or(self.total_size)
    }

    #[allow(unsafe_code)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read_exact(buf).map(|_| buf.len())
    }

    #[allow(unsafe_code)]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.stream.read_exact(buf)
    }

    #[allow(unsafe_code)]
    fn seek(&mut self, position: u64) -> io::Result<u64> {
        self.stream.seek(io::SeekFrom::Start(position as _))
    }

    #[allow(unsafe_code)]
    fn wait_for_file_size(&mut self, target_size: u64) -> ReaderGrowStatus {
        if self.stream.stream_position().is_err() {
            ReaderGrowStatus::Timeout
        } else if target_size > self.total_size {
            ReaderGrowStatus::SizeBeyondEof
        } else {
            ReaderGrowStatus::SizeReached
        }
    }
}

/// # Safety
///
/// The native callback must receive the live reader pointer registered by
/// `read_reader` and must have exclusive access to that reader.
#[allow(unsafe_code)]
unsafe extern "C" fn get_position(user_data: *mut c_void) -> i64 {
    // read_reader registers the heap-held Box as userdata. ~keep
    // FIXME(#3229): This requires the box to be live and the callbacks to have exclusive reader access.
    let reader = unsafe { &mut *(user_data as *mut Box<dyn Reader>) };
    reader.position() as _
}

#[allow(unsafe_code)]
unsafe extern "C" fn read(data: *mut c_void, size: usize, user_data: *mut c_void) -> c_int {
    if data.is_null() || size == 0 {
        return 0;
    }
    // read_reader registers the heap-held Box as userdata. ~keep
    // FIXME(#3229): This requires the box to be live and the callbacks to have exclusive reader access.
    let reader = unsafe { &mut *(user_data as *mut Box<dyn Reader>) };
    // Native item payload reads resize their vector before passing its initialized bytes here. ~keep
    // FIXME(#3229): The callback relies on every native caller providing initialized exclusive storage.
    let buf = unsafe { slice::from_raw_parts_mut(data as *mut u8, size) };
    if reader.read_exact(buf).is_ok() { 0 } else { 1 }
}

#[allow(unsafe_code)]
unsafe extern "C" fn seek(position: i64, user_data: *mut c_void) -> c_int {
    // read_reader registers the heap-held Box as userdata. ~keep
    // FIXME(#3229): This requires the box to be live and the callbacks to have exclusive reader access.
    let reader = unsafe { &mut *(user_data as *mut Box<dyn Reader>) };
    match reader.seek(position as _) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

/// # Safety
///
/// The native callback must receive the live reader pointer registered by
/// `read_reader` and must have exclusive access to that reader.
#[allow(unsafe_code)]
unsafe extern "C" fn wait_for_file_size(target_size: i64, user_data: *mut c_void) -> lh::heif_reader_grow_status {
    // read_reader registers the heap-held Box as userdata. ~keep
    // FIXME(#3229): This requires the box to be live and the callbacks to have exclusive reader access.
    let reader = unsafe { &mut *(user_data as *mut Box<dyn Reader>) };
    let target_size = target_size as u64;
    reader.wait_for_file_size(target_size) as _
}

#[cfg(not(feature = "v1_19"))]
pub(crate) static HEIF_READER: lh::heif_reader = lh::heif_reader {
    reader_api_version: 1,
    get_position: Some(get_position),
    read: Some(read),
    seek: Some(seek),
    wait_for_file_size: Some(wait_for_file_size),
};

#[cfg(feature = "v1_19")]
pub(crate) static HEIF_READER: lh::heif_reader = lh::heif_reader {
    reader_api_version: 1,
    get_position: Some(get_position),
    read: Some(read),
    seek: Some(seek),
    wait_for_file_size: Some(wait_for_file_size),
    request_range: None,
    preload_range_hint: None,
    release_file_range: None,
    release_error_msg: None,
};
