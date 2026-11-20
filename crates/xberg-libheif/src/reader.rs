use std::os::raw::{c_int, c_void};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::{io, slice};

use libheif_sys as lh;

use crate::enums::ReaderGrowStatus;

/// A byte source libheif pulls HEIF data from.
///
/// `Send` is required because libheif may invoke the callbacks from its own decoding
/// worker threads, not only from the thread that owns the context. A source that is not
/// `Send` is rejected at compile time:
///
/// ```compile_fail
/// use std::io::{self, Cursor, Read, Seek, SeekFrom};
/// use std::rc::Rc;
/// use xberg_libheif::{HeifContext, StreamReader};
///
/// struct NotSend(Cursor<Vec<u8>>, Rc<()>);
///
/// impl Read for NotSend {
///     fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
///         self.0.read(buf)
///     }
/// }
///
/// impl Seek for NotSend {
///     fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
///         self.0.seek(pos)
///     }
/// }
///
/// let stream = NotSend(Cursor::new(Vec::new()), Rc::new(()));
/// let _ = HeifContext::read_from_reader(Box::new(StreamReader::new(stream, 0)));
/// ```
pub trait Reader: Send {
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
    T: io::Read + io::Seek + Send,
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

/// The reader as stored by `HeifContext` and registered with libheif as callback user data.
///
/// The mutex serializes callbacks, so a reader that is only `Send` is never entered twice
/// at once even when libheif calls from several worker threads.
pub(crate) type SharedReader<'a> = Mutex<Box<dyn Reader + 'a>>;

/// # Safety
///
/// `user_data` must be the address of the `SharedReader` that `HeifContext::read_reader`
/// registered. That value sits in a stable heap allocation owned by the context's backing
/// store, which is freed only after the native context (and every handle sharing it) is gone,
/// so it is live for every callback libheif can make.
#[allow(unsafe_code)]
unsafe fn locked_reader<'r>(user_data: *mut c_void) -> MutexGuard<'r, Box<dyn Reader + 'r>> {
    // SAFETY: Per this function's contract the pointer addresses a live `SharedReader`; only a
    // shared reference is formed and exclusive reader access comes from the mutex. A poisoned
    // lock only means an earlier callback panicked, and the reader state is still memory-safe.
    let reader = unsafe { &*(user_data as *const SharedReader<'r>) };
    reader.lock().unwrap_or_else(PoisonError::into_inner)
}

/// # Safety
///
/// `user_data` must satisfy the contract of [`locked_reader`].
#[allow(unsafe_code)]
unsafe extern "C" fn get_position(user_data: *mut c_void) -> i64 {
    // SAFETY: libheif passes back the user data registered by `read_reader`.
    let mut reader = unsafe { locked_reader(user_data) };
    reader.position() as _
}

/// # Safety
///
/// `user_data` must satisfy the contract of [`locked_reader`]; a non-null `data` must point to
/// `size` initialized, writable bytes that no other thread accesses during the call.
#[allow(unsafe_code)]
unsafe extern "C" fn read(data: *mut c_void, size: usize, user_data: *mut c_void) -> c_int {
    if data.is_null() || size == 0 {
        return 0;
    }
    // SAFETY: libheif passes back the user data registered by `read_reader`.
    let mut reader = unsafe { locked_reader(user_data) };
    // SAFETY: libheif's read contract hands over `size` bytes of its own exclusive, initialized
    // destination buffer (it resizes its vector before the call); null and zero size are
    // rejected above.
    let buf = unsafe { slice::from_raw_parts_mut(data as *mut u8, size) };
    if reader.read_exact(buf).is_ok() { 0 } else { 1 }
}

/// # Safety
///
/// `user_data` must satisfy the contract of [`locked_reader`].
#[allow(unsafe_code)]
unsafe extern "C" fn seek(position: i64, user_data: *mut c_void) -> c_int {
    // SAFETY: libheif passes back the user data registered by `read_reader`.
    let mut reader = unsafe { locked_reader(user_data) };
    match reader.seek(position as _) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

/// # Safety
///
/// `user_data` must satisfy the contract of [`locked_reader`].
#[allow(unsafe_code)]
unsafe extern "C" fn wait_for_file_size(target_size: i64, user_data: *mut c_void) -> lh::heif_reader_grow_status {
    // SAFETY: libheif passes back the user data registered by `read_reader`.
    let mut reader = unsafe { locked_reader(user_data) };
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
