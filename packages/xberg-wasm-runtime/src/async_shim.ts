/**
 * Single-flight enforcement for the wasm engine.
 *
 * **MECHANISM NOTE:** The engine holds `&self` across an `await` in its JSPI bridges.
 * This means overlapping async calls on one engine instance will race and corrupt state.
 * The engine API is NOT thread-safe across concurrent invocations on the same handle.
 *
 * **USAGE:** Frontends (browser UI, MCP server) must serialize calls to one engine instance.
 * Do not call engine.ingest() and engine.query() concurrently on the same engine handle.
 *
 * This module provides a guard to detect and report violations in development.
 * In production, the onus is on the caller to respect single-flight discipline.
 */

export class SingleFlightGuard {
	private active = false;
	private label: string;

	constructor(label: string) {
		this.label = label;
	}

	/**
	 * Run an async operation with single-flight enforcement.
	 * Throws if called concurrently.
	 */
	async run<T>(fn: () => Promise<T>): Promise<T> {
		if (this.active) {
			throw new Error(
				`[${this.label}] single-flight violation: concurrent call detected. ` +
					`The wasm engine holds &self across an await and is not re-entrant. ` +
					`Caller must serialize invocations on one engine handle.`,
			);
		}

		this.active = true;
		try {
			return await fn();
		} finally {
			this.active = false;
		}
	}
}

/**
 * Document the single-flight constraint in the injection descriptor.
 * Callers should review this when integrating the engine into their application.
 */
export const SINGLE_FLIGHT_CONSTRAINT = `
The XbergEngine injection descriptor holds a reference to the embedder and store
across async suspension points. This means:

1. The engine is NOT safe for concurrent calls on a single handle.
2. Overlapping calls to engine.ingest(), engine.query(), engine.ocr(), etc. on the
   same handle will race and may corrupt state or return incorrect results.
3. Callers MUST serialize all operations on a given engine instance.

Example (WRONG):
  const engine = new XbergEngine(config, injection);
  Promise.all([
    engine.ingest(doc, "col"),
    engine.query(q, "col", 10)  // RACE! both accessing store concurrently
  ]);

Example (CORRECT):
  const engine = new XbergEngine(config, injection);
  await engine.ingest(doc, "col");
  const results = await engine.query(q, "col", 10);  // Sequential

If your frontend needs concurrent extraction, create multiple engine instances (one per
logical task) and let the injection layer (store, embedder) handle synchronization.
`;
