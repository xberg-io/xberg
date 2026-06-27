#!/usr/bin/env node
// Generates docs/demo-dev.html from docs/demo.html with CDN URLs replaced
// by the local asset server so no manual editing of demo.html is ever needed.
//
// CDN pattern replaced:
//   https://cdn.jsdelivr.net/npm/@xberg-io/xberg-wasm@*/...
//   → http://localhost:9000/...
//
// Also patches pkg/web/xberg_wasm.js (gitignored, wasm-pack generated) to
// replace bare specifier imports ("env", "wasi_snapshot_preview1") with inline
// browser shims.  The local 5.x WASM binary is compiled with WASI syscalls via
// tesseract's C layer; the importmap approach does not propagate into Workers
// loading cross-origin modules, so we shim the generated JS directly.
//
// The output file is gitignored and regenerated on every `task demo:dev`.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const src = join(root, "docs", "demo.html");
const dest = join(root, "docs", "demo-dev.html");
const ASSET_PORT = process.env.ASSET_PORT ?? "9000";

const cdnRe = /https:\/\/cdn\.jsdelivr\.net\/npm\/@xberg\/wasm@[^/'"]+/g;

const patched = readFileSync(src, "utf8")
	.replace(cdnRe, `http://localhost:${ASSET_PORT}`)
	.replace(/<title>(.*?)<\/title>/, "<title>$1 [local dev]</title>")
	.replace(
		"</body>",
		`  <div style="position:fixed;bottom:12px;right:12px;background:#1a172a;border:1px solid #58FBDA55;color:#58FBDA;font-family:monospace;font-size:11px;padding:6px 10px;border-radius:6px;z-index:9999">
    local dev · assets: localhost:${ASSET_PORT}
  </div>\n</body>`,
	);

writeFileSync(dest, patched, "utf8");
console.log(`patch-demo-dev: docs/demo-dev.html → http://localhost:8001/demo-dev.html`);
console.log(`  assets served from http://localhost:${ASSET_PORT}`);

// Patch pkg/web/xberg_wasm.js — strip bare "env" / "wasi_snapshot_preview1"
// import lines and replace with inline browser shims so the module loads in a
// Worker without an importmap (importmap inheritance in Workers is unreliable
// for bare specifiers in transitive cross-origin dynamic imports).
const wasmJs = join(root, "crates", "xberg-wasm", "pkg", "web", "xberg_wasm.js");
if (!existsSync(wasmJs)) {
	console.warn(`patch-demo-dev: ${wasmJs} not found — skipping WASI shim patch`);
} else {
	const bareImportRe = /^import \* as (import\d+) from "(env|wasi_snapshot_preview1)"\s*$/gm;
	const original = readFileSync(wasmJs, "utf8");

	const envAliases = [];
	const wasiAliases = [];
	let m;
	while ((m = bareImportRe.exec(original)) !== null) {
		if (m[2] === "env") envAliases.push(m[1]);
		else wasiAliases.push(m[1]);
	}

	if (envAliases.length === 0 && wasiAliases.length === 0) {
		console.log("patch-demo-dev: xberg_wasm.js already patched, skipping");
	} else {
		const stripped = original.replace(/^import \* as import\d+ from "(env|wasi_snapshot_preview1)"\s*\n/gm, "");

		const envShim = `const __env_shim = { system: () => -1, mkstemp: () => -1 };`;
		const envConsts = envAliases.map((a) => `const ${a} = __env_shim;`).join("\n");

		const wasiShim = [
			`const __wasi_shim = {`,
			`  environ_sizes_get: () => 0, environ_get: () => 0,`,
			`  clock_time_get: () => 52,`,
			`  fd_close: () => 8, fd_fdstat_get: () => 8, fd_fdstat_set_flags: () => 8,`,
			`  fd_prestat_get: () => 8, fd_prestat_dir_name: () => 8,`,
			`  fd_read: () => 8, fd_seek: () => 8, fd_write: () => 8,`,
			`  path_create_directory: () => 52, path_filestat_get: () => 52,`,
			`  path_open: () => 52, path_remove_directory: () => 52, path_unlink_file: () => 52,`,
			`  proc_exit: (code) => { throw new Error("WASI: proc_exit(" + code + ")"); },`,
			`};`,
		].join("\n");
		const wasiConsts = wasiAliases.map((a) => `const ${a} = __wasi_shim;`).join("\n");

		const shims = [envShim, envConsts, wasiShim, wasiConsts].filter(Boolean).join("\n") + "\n";
		const patchedWasmJs = stripped.replace(/^(\/\* @ts-self-types[^\n]*\n)/m, `$1${shims}`);

		writeFileSync(wasmJs, patchedWasmJs, "utf8");
		console.log(
			`patch-demo-dev: patched xberg_wasm.js` +
				` (${envAliases.length} env alias(es), ${wasiAliases.length} wasi alias(es))`,
		);
	}
}
