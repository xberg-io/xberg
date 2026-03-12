#!/usr/bin/env node
/**
 * Post-build script to fix WASI and env imports in wasm-bindgen generated JS.
 *
 * Problem: When Tesseract/Leptonica are compiled with WASI SDK and linked into
 * the wasm-bindgen output, the generated JS has:
 * 1. `import * as importN from "env"` / `import * as importN from "wasi_snapshot_preview1"`
 *    statements that can't be resolved (no such ES modules exist)
 * 2. Duplicate object keys in __wbg_get_imports() return value (JS last-key-wins
 *    means only the last import per namespace survives)
 *
 * Solution: Replace external module imports with inline stub implementations and
 * merge duplicate keys using Object.assign().
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const pkgDir = path.join(__dirname, "..", "pkg");
const jsFile = path.join(pkgDir, "kreuzberg_wasm.js");

if (!fs.existsSync(jsFile)) {
	console.log("No pkg/kreuzberg_wasm.js found, skipping WASI import fix.");
	process.exit(0);
}

let content = fs.readFileSync(jsFile, "utf-8");
const originalContent = content;

// Check if already patched (idempotent)
if (content.includes("__wasi_stubs__")) {
	console.log("WASI imports already patched, skipping.");
	process.exit(0);
}

// Check if there are any env/wasi imports to fix (ESM or CJS)
const hasEsmImports = content.includes('from "env"') || content.includes('from "wasi_snapshot_preview1"');
const hasCjsImports = content.includes('require("env")') || content.includes('require("wasi_snapshot_preview1")');
if (!hasEsmImports && !hasCjsImports) {
	console.log("No env/wasi_snapshot_preview1 imports found, skipping WASI import fix.");
	process.exit(0);
}

console.log("Fixing WASI and env imports in kreuzberg_wasm.js...\n");

// Step 1: Collect all importN identifiers and their source modules
// Support both ESM: import * as importN from "env"
// and CJS: const importN = require("env")
const esmPattern = /^import \* as (import\d+) from "(env|wasi_snapshot_preview1)";?$/gm;
const cjsPattern = /^const (import\d+) = require\("(env|wasi_snapshot_preview1)"\);?$/gm;
const envImports = [];
const wasiImports = [];

for (const match of content.matchAll(esmPattern)) {
	const [, varName, moduleName] = match;
	if (moduleName === "env") {
		envImports.push(varName);
	} else {
		wasiImports.push(varName);
	}
}

for (const match of content.matchAll(cjsPattern)) {
	const [, varName, moduleName] = match;
	if (moduleName === "env") {
		envImports.push(varName);
	} else {
		wasiImports.push(varName);
	}
}

console.log(`Found ${envImports.length} env imports: ${envImports.join(", ")}`);
console.log(`Found ${wasiImports.length} wasi_snapshot_preview1 imports: ${wasiImports.join(", ")}`);

// Step 2: Remove all import/require statements for env and wasi_snapshot_preview1
content = content.replace(/^import \* as import\d+ from "(env|wasi_snapshot_preview1)";?\n/gm, "");
content = content.replace(/^const import\d+ = require\("(env|wasi_snapshot_preview1)"\);?\n/gm, "");

// Step 3: Insert stub definitions at the same location (before __wbg_get_imports)
const stubCode = `// __wasi_stubs__ - WASI and env import stubs for in-memory OCR processing
// Lazy reference to WASM memory, populated after module instantiation.
// Stubs that write output values use this to access WASM linear memory.
let __wasi_mem_ref = { memory: null };
function __wasi_view() {
    if (!__wasi_mem_ref.memory) return null;
    return new DataView(__wasi_mem_ref.memory.buffer);
}

// env stubs: system() and mkstemp() are never called at runtime in WASM OCR.
// The Proxy catch-all handles any additional env imports (e.g. TessDeleteText and
// other Tesseract FFI symbols) that the linker may leave unresolved on some platforms.
const __env_stubs__ = new Proxy({
    system: () => -1,
    mkstemp: () => -1,
}, {
    get(target, prop) {
        if (prop in target) return target[prop];
        return () => {};
    }
});

// WASI stubs: minimal implementations for WASI preview1 syscalls.
// Functions that take output pointers write proper values to WASM memory.
const __wasi_stubs__ = {
    fd_close: () => 0,
    fd_read: (fd, iovs_ptr, iovs_len, nread_ptr) => {
        const v = __wasi_view();
        if (v && nread_ptr) v.setUint32(nread_ptr, 0, true);
        return 0;
    },
    fd_write: (fd, iovs_ptr, iovs_len, nwritten_ptr) => {
        const v = __wasi_view();
        if (v) {
            let total = 0;
            for (let i = 0; i < iovs_len; i++) {
                total += v.getUint32(iovs_ptr + i * 8 + 4, true);
            }
            if (nwritten_ptr) v.setUint32(nwritten_ptr, total, true);
        }
        return 0;
    },
    fd_seek: (fd, offset_lo, offset_hi, whence, newoffset_ptr) => {
        const v = __wasi_view();
        if (v && newoffset_ptr) {
            v.setUint32(newoffset_ptr, 0, true);
            v.setUint32(newoffset_ptr + 4, 0, true);
        }
        return 0;
    },
    fd_fdstat_get: (fd, fdstat_ptr) => {
        const v = __wasi_view();
        if (v && fdstat_ptr) {
            v.setUint8(fdstat_ptr, fd <= 2 ? 2 : 4);
            v.setUint16(fdstat_ptr + 2, 0, true);
            v.setBigUint64(fdstat_ptr + 8, 0xffffffffffffffffn, true);
            v.setBigUint64(fdstat_ptr + 16, 0xffffffffffffffffn, true);
        }
        return 0;
    },
    fd_fdstat_set_flags: (fd, flags) => 0,
    fd_prestat_get: (fd, prestat_ptr) => 8, // EBADF - no preopened dirs
    fd_prestat_dir_name: (fd, path_ptr, path_len) => 8, // EBADF
    environ_get: (environ_ptr, environ_buf_ptr) => 0,
    environ_sizes_get: (count_ptr, buf_size_ptr) => {
        const v = __wasi_view();
        if (v) {
            if (count_ptr) v.setUint32(count_ptr, 0, true);
            if (buf_size_ptr) v.setUint32(buf_size_ptr, 0, true);
        }
        return 0;
    },
    clock_time_get: (clock_id, precision, time_ptr) => {
        const v = __wasi_view();
        if (v && time_ptr) {
            v.setBigUint64(time_ptr, BigInt(Math.floor(Date.now() * 1e6)), true);
        }
        return 0;
    },
    path_create_directory: (fd, path_ptr, path_len) => 63, // ENOSYS
    path_filestat_get: (fd, flags, path_ptr, path_len, filestat_ptr) => 63,
    path_open: (dirfd, dirflags, path_ptr, path_len, oflags, fs_rights_base_lo, fs_rights_base_hi, fs_rights_inheriting_lo, fs_rights_inheriting_hi, fdflags, fd_ptr) => 63,
    path_remove_directory: (fd, path_ptr, path_len) => 63,
    path_unlink_file: (fd, path_ptr, path_len) => 63,
    proc_exit: (code) => { throw new Error(\`WASM proc_exit called with code \${code}\`); },
    sched_yield: () => 0,
};

`;

// Insert stubs before __wbg_get_imports function
const getImportsIdx = content.indexOf("function __wbg_get_imports()");
if (getImportsIdx === -1) {
	console.error("ERROR: Could not find __wbg_get_imports() function in kreuzberg_wasm.js");
	process.exit(1);
}
content = content.slice(0, getImportsIdx) + stubCode + content.slice(getImportsIdx);

// Step 4: Replace all importN references for env/wasi with the stub objects
for (const varName of envImports) {
	content = content.replaceAll(varName, "__env_stubs__");
}
for (const varName of wasiImports) {
	content = content.replaceAll(varName, "__wasi_stubs__");
}

// Step 5: Merge duplicate keys in the __wbg_get_imports return object
// The return block looks like:
//   return {
//       __proto__: null,
//       "./kreuzberg_wasm_bg.js": import0,
//       "env": __env_stubs__,
//       "env": __env_stubs__,
//       "wasi_snapshot_preview1": __wasi_stubs__,
//       "wasi_snapshot_preview1": __wasi_stubs__,
//       ...
//   };
// Since all env stubs point to the same object and all wasi stubs point to the same object,
// we just need to deduplicate the keys. Remove all duplicate "env" and "wasi_snapshot_preview1" lines.
const returnBlockStart = content.indexOf('"./kreuzberg_wasm_bg.js": import0,');
if (returnBlockStart !== -1) {
	const returnBlockEnd = content.indexOf("};", returnBlockStart);
	if (returnBlockEnd !== -1) {
		const returnBlock = content.slice(returnBlockStart, returnBlockEnd);

		// Remove duplicate "env" lines (keep first)
		let seenEnv = false;
		let seenWasi = false;
		const lines = returnBlock.split("\n");
		const dedupedLines = lines.filter((line) => {
			const trimmed = line.trim();
			if (trimmed.startsWith('"env"')) {
				if (seenEnv) return false;
				seenEnv = true;
				return true;
			}
			if (trimmed.startsWith('"wasi_snapshot_preview1"')) {
				if (seenWasi) return false;
				seenWasi = true;
				return true;
			}
			return true;
		});

		content = content.slice(0, returnBlockStart) + dedupedLines.join("\n") + content.slice(returnBlockEnd);
	}
}

// Step 6: Inject WASI memory reference after WASM instantiation
// The WASI stubs need access to WASM linear memory to write output values.
// Look for the wasm instantiation pattern and add memory ref after it.
const instantiatePatterns = [
	// CJS pattern: let wasm = new WebAssembly.Instance(...).exports;
	/^(let wasm = new WebAssembly\.Instance\(.*\)\.exports;)$/m,
	// ESM/async pattern: const instance = await WebAssembly.instantiate(...)
	/^(const \{ instance \} = await WebAssembly\.instantiate\(.*\);)$/m,
];
let memRefInjected = false;
for (const pattern of instantiatePatterns) {
	if (pattern.test(content)) {
		content = content.replace(
			pattern,
			"$1\n// Populate WASI memory reference for stubs that write output values\n__wasi_mem_ref.memory = wasm.memory || (typeof instance !== 'undefined' && instance.exports.memory);",
		);
		memRefInjected = true;
		break;
	}
}
if (!memRefInjected) {
	console.log("WARNING: Could not find WASM instantiation to inject memory reference.");
	console.log("WASI stubs that write to memory output pointers may not work correctly.");
}

if (content === originalContent) {
	console.log("No changes needed.");
} else {
	fs.writeFileSync(jsFile, content);
	const removedImports = envImports.length + wasiImports.length;
	console.log(`Replaced ${removedImports} external imports with inline stubs.`);
	console.log("Deduplicated import keys in __wbg_get_imports().");
	if (memRefInjected) console.log("Injected WASI memory reference after WASM instantiation.");
	console.log("Done.");
}
