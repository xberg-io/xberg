// WASM loader that provides WASI imports
import Module from "module";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Mock WASI environment
const mockEnv = {
  // Standard C library functions
  malloc: () => 0,
  free: () => {},
  memcpy: () => 0,
  memset: () => 0,
  strlen: () => 0,
};

const mockWasi = {
  proc_exit: () => {},
  environ_get: () => 0,
  environ_sizes_get: () => 0,
  fd_write: () => 0,
  fd_read: () => 0,
  fd_seek: () => 0,
  fd_close: () => 0,
  fd_prestat: () => 8,
  fd_prestat_dir_name: () => 0,
  path_open: () => 8,
  path_create_directory: () => 0,
  path_remove_directory: () => 0,
  path_unlink_file: () => 0,
  path_filestat_get: () => 0,
  path_rename: () => 0,
  sys_info: () => 0,
  clock_time_get: () => 0,
  random_get: (buf, buflen) => 0,
  thread_spawn: () => 0,
  args_get: () => 0,
  args_sizes_get: () => 0,
};

// Register globals
globalThis.env = mockEnv;
globalThis.wasi_snapshot_preview1 = mockWasi;
