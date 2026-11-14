use crate::error::{Result, TesseractError};
use std::os::raw::{c_int, c_void};
use std::sync::{Arc, Mutex};

pub struct TessMonitor {
    handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for TessMonitor {}
unsafe impl Sync for TessMonitor {}

impl TessMonitor {
    /// Creates a new instance of the TessMonitor.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the TessMonitor.
    pub fn new() -> Self {
        let handle = unsafe { TessMonitorCreate() };
        TessMonitor {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Sets the deadline for the monitor.
    ///
    /// # Arguments
    ///
    /// * `deadline` - Deadline in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns a `TesseractError::MutexLockError` if the mutex lock fails.
    pub fn set_deadline(&self, deadline: i32) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        unsafe { TessMonitorSetDeadlineMSecs(*handle, deadline) };
        Ok(())
    }

    /// Gets the progress of the monitor.
    ///
    /// # Returns
    ///
    /// Returns the progress as an `i32` if successful, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns a `TesseractError::MutexLockError` if the mutex lock fails.
    pub fn get_progress(&self) -> Result<i32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessMonitorGetProgress(*handle) })
    }

    pub(crate) fn with_handle<T>(&self, action: impl FnOnce(*mut c_void) -> T) -> Result<T> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        if handle.is_null() {
            return Err(TesseractError::OcrError);
        }
        Ok(action(*handle))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn with_cancel<F: Fn() -> bool>(
        &self,
        stop: &F,
        recognize: impl FnOnce(*mut c_void) -> Result<()>,
    ) -> Result<()> {
        unsafe extern "C" fn cancelled<F: Fn() -> bool>(data: *mut c_void, _: c_int) -> bool {
            // SAFETY: with_cancel borrows the callback for the synchronous recognition call.
            let stop = unsafe { &*data.cast::<F>() };
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(stop)).unwrap_or(true)
        }

        self.with_handle(|handle| {
            // SAFETY: this private monitor is locked for the call and the callback outlives it.
            unsafe {
                TessMonitorSetCancelThis(handle, std::ptr::from_ref(stop).cast_mut().cast());
                TessMonitorSetCancelFunc(handle, Some(cancelled::<F>));
            }
            let result = recognize(handle);
            // SAFETY: the native call returned and no callback can still be running.
            unsafe {
                TessMonitorSetCancelFunc(handle, None);
                TessMonitorSetCancelThis(handle, std::ptr::null_mut());
            }
            result
        })?
    }
}

impl Drop for TessMonitor {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            unsafe { TessMonitorDelete(*handle) };
        }
    }
}

ffi_extern! {
    pub fn TessMonitorCreate() -> *mut c_void;
    pub fn TessMonitorDelete(monitor: *mut c_void);
    pub fn TessMonitorSetDeadlineMSecs(monitor: *mut c_void, deadline: c_int);
    #[cfg(not(target_arch = "wasm32"))]
    fn TessMonitorSetCancelFunc(monitor: *mut c_void, callback: Option<unsafe extern "C" fn(*mut c_void, c_int) -> bool>);
    #[cfg(not(target_arch = "wasm32"))]
    fn TessMonitorSetCancelThis(monitor: *mut c_void, data: *mut c_void);
    pub fn TessMonitorGetProgress(monitor: *mut c_void) -> c_int;
}
