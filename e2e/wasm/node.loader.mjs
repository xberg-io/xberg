// Custom Node.js loader for WASM imports
import Module from "module";

const originalResolveFilename = Module.prototype._resolveFilename;

Module.prototype._resolveFilename = function (request, parent, isMain) {
	if (request === "env" || request === "wasi_snapshot_preview1") {
		// Return a fake module path that won't be resolved
		// Instead, we'll handle it in the import hook
		return request;
	}
	return originalResolveFilename.apply(this, arguments);
};

// ES module loader hook
export async function resolve(specifier, context, nextResolve) {
	if (specifier === "env" || specifier === "wasi_snapshot_preview1") {
		return {
			url: "node:vm",
			shortCircuit: true,
		};
	}
	return nextResolve(specifier);
}

export async function getFormat(url, context, nextGetFormat) {
	return nextGetFormat(url);
}

export async function getSource(url, context, nextGetSource) {
	return nextGetSource(url);
}

export async function load(url, context, nextLoad) {
	if (url === "node:vm") {
		return {
			format: "module",
			source: "export default {}; export const wasi_snapshot_preview1 = {};",
		};
	}
	return nextLoad(url);
}
