// Minimal WASI preview1 stubs for browser context.
// The WASM binary was compiled with WASI syscalls (via tesseract's C layer)
// but none are invoked during in-browser document extraction. These stubs
// allow the module to instantiate without a full WASI runtime.
//
// errno values per WASI spec: 0=success, 8=BADF, 52=NOSYS
const ERRNO_SUCCESS = 0;
const ERRNO_BADF    = 8;
const ERRNO_NOSYS   = 52;

// Environment variables — return empty set
export function environ_sizes_get() { return ERRNO_SUCCESS; }
export function environ_get()       { return ERRNO_SUCCESS; }

// Clock — not needed for extraction timing paths
export function clock_time_get()    { return ERRNO_NOSYS; }

// File descriptors — no preopened fds in browser
export function fd_close()            { return ERRNO_BADF; }
export function fd_fdstat_get()       { return ERRNO_BADF; }
export function fd_fdstat_set_flags() { return ERRNO_BADF; }
export function fd_prestat_get()      { return ERRNO_BADF; }
export function fd_prestat_dir_name() { return ERRNO_BADF; }
export function fd_read()             { return ERRNO_BADF; }
export function fd_seek()             { return ERRNO_BADF; }
export function fd_write()            { return ERRNO_BADF; }

// Path operations — no filesystem in browser
export function path_create_directory() { return ERRNO_NOSYS; }
export function path_filestat_get()     { return ERRNO_NOSYS; }
export function path_open()             { return ERRNO_NOSYS; }
export function path_remove_directory() { return ERRNO_NOSYS; }
export function path_unlink_file()      { return ERRNO_NOSYS; }

// proc_exit — throw so Rust panics surface as JS errors
export function proc_exit(code) { throw new Error(`WASI: proc_exit(${code})`); }
