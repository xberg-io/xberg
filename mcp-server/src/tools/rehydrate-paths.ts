import * as path from "path";

/**
 * Resolve a rehydration map's path from a caller-supplied `document_id`,
 * rejecting anything that isn't a plain filename component. `document_id`
 * is untrusted input reachable from any MCP client; without this check
 * `path.join(dir, \`${document_id}.map\`)` lets a value like
 * `../../../../etc/passwd` (or an absolute path, which `path.join` does
 * not confine either) escape `dir` entirely — a read primitive in
 * `find_pii_subject`/`rehydrate_document`, and an arbitrary-file-overwrite
 * primitive in `forget_pii_subject`, which writes back to the resolved path.
 * Deliberately rejects rather than sanitizing (e.g. via `path.basename`):
 * silently rewriting a traversal attempt to a different file is its own
 * footgun (the caller thinks it operated on the id it sent).
 *
 * Kept in its own module (no wasm-engine import) so it can be unit-tested
 * without needing the built `@xberg-io/xberg-wasm` binary — `rehydrate.ts`'s
 * other exports pull that in at module-load time via `getEngine`.
 */
export function resolveMapPath(dir: string, documentId: string): string {
	if (!/^[A-Za-z0-9._-]+$/.test(documentId) || documentId === "." || documentId === "..") {
		throw new Error(`invalid document_id "${documentId}": must contain only letters, digits, '.', '_', '-'`);
	}
	return path.join(dir, `${documentId}.map`);
}
