// Register WASI imports for WASM modules in Node.js
// This file is loaded before vitest runs

const Module = require("module");
const path = require("path");

// Create mock WASI and env objects
const env = {};
const wasi_snapshot_preview1 = {
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

// Patch require to provide these modules
const originalRequire = Module.prototype.require;
Module.prototype.require = function (id) {
	if (id === "env") return env;
	if (id === "wasi_snapshot_preview1") return wasi_snapshot_preview1;
	return originalRequire.apply(this, arguments);
};

// For ES modules, we need a different approach - use globalThis
globalThis.env = env;
globalThis.wasi_snapshot_preview1 = wasi_snapshot_preview1;
