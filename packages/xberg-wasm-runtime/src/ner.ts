import { pipeline, env } from "@huggingface/transformers";
import type { CacheConfig, Entity, NerInterface } from "./types.js";
import type { TokenClassificationSingle } from "@huggingface/transformers";
import { selectModelBackend } from "./backend.js";
import { configureTransformersEnvironment } from "./runtime-env.js";

// Allow reading locally-cached transformers.js models in CI environments.
// Note: this permits local file loading but does NOT suppress remote downloads;
// remote model fetching is controlled by env.allowRemoteModels separately.
if (typeof process !== "undefined" && process.env.CI) {
	env.allowLocalModels = true;
}

// "Xenova/gliner2-small-onnx" (the model named in the original spec) does not
// exist on the Hub. GLiNER2 ONNX exports that do exist (e.g.
// SemplificaAI/gliner2-multi-v1-onnx) target a schema-driven "zero-shot"
// extraction API that is not the standard transformers.js
// `token-classification` pipeline used here. We default to
// "Xenova/bert-base-NER" — a real, canonically-cased NER model (ONNX export
// of dslim/bert-base-NER) explicitly documented as transformers.js
// v3-compatible — while still allowing callers to inject any other
// token-classification model id via `config.models.ner`.
const DEFAULT_NER_MODEL = "Xenova/bert-base-NER";

const BEGIN_PREFIX = "B-";
const INSIDE_PREFIX = "I-";
const OUTSIDE_LABEL = "O";

/**
 * Create a NER (named entity recognition) interface using transformers.js v3.
 * Returns null if NER is disabled or the model cannot be loaded.
 * Optional; if not injected into the engine, the engine falls back to in-binary Candle NER.
 */
export async function createNer(config?: CacheConfig): Promise<NerInterface | null> {
	try {
		const modelId = config?.models?.ner ?? DEFAULT_NER_MODEL;
		configureTransformersEnvironment(config);

		const backend = await selectModelBackend(config);
		console.debug(`[ner] device=${backend.device} dtype=${backend.dtype} model=${modelId}`);
		const tokenClassifier = await pipeline("token-classification", modelId, backend);

		/**
		 * Named entity recognition on the given text. Returns a list of named
		 * entities with their labels, text, and confidence scores.
		 *
		 * IMPORTANT: The currently-loaded model (Xenova/bert-base-NER) recognizes
		 * only a fixed label set: PER (person), ORG (organization), LOC (location),
		 * and MISC (miscellaneous). The `categories` parameter filters results to
		 * only entities matching those labels, but only works within this fixed
		 * set. Requesting categories outside this set (e.g., EMAIL, PHONE) will
		 * silently return no results with no error — packages/xberg-wasm-runtime's
		 * pii.ts regex layer exists specifically to cover that gap deterministically.
		 *
		 * `categories` is a plain positional array (not an options object) because
		 * this must match crates/xberg-wasm/src/bridge/ner.rs's
		 * call_injected_ner, which calls `ner(text, categories)` positionally —
		 * the Rust bridge is the fixed contract this signature exists to satisfy.
		 *
		 * @param text The input text to analyze
		 * @param categories Optional label filter
		 * @param threshold Optional minimum confidence score
		 * @returns Array of entities with label, text, position, and confidence score
		 */
		async function ner(text: string, categories?: string[], threshold?: number): Promise<Entity[]> {
			if (!text || text.length === 0) return [];

			try {
				const predictions = await tokenClassifier(text);
				const tokens = (
					Array.isArray(predictions) ? predictions : [predictions]
				) as TokenClassificationSingle[];

				return mergeEntities(tokens, text, categories, threshold);
			} catch (err) {
				console.error("[ner] classification failed:", err);
				return [];
			}
		}

		return { ner };
	} catch (err) {
		console.warn("[ner] model load failed, falling back to in-binary:", err);
		return null;
	}
}

const WORDPIECE_CONTINUATION_PREFIX = "##";

/**
 * Merge consecutive B-/I- prefixed token predictions of the same entity type
 * into single entity spans. transformers.js token-classification pipelines
 * return per-token predictions (BIO tagging scheme), not pre-merged spans, so
 * this grouping step is required to produce whole-entity `Entity` records.
 *
 * Many WordPiece-tokenizer NER models (e.g. bert-base-NER) do not populate
 * `start`/`end` character offsets on their token predictions at all — the
 * fields are typed optional in transformers.js precisely because not every
 * tokenizer computes them. When missing, offsets are recovered here by
 * locating each token's surface text in the original string with a
 * forward-scanning cursor (so repeated words resolve to distinct
 * occurrences in order).
 */
function mergeEntities(
	tokens: TokenClassificationSingle[],
	sourceText: string,
	categories?: string[],
	threshold?: number,
): Entity[] {
	const entities: Entity[] = [];
	let current: Entity | null = null;
	let searchCursor = 0;

	for (const token of tokens) {
		const rawLabel = token.entity;
		if (!rawLabel || rawLabel === OUTSIDE_LABEL) {
			current = null;
			continue;
		}

		const isBegin = rawLabel.startsWith(BEGIN_PREFIX);
		const isInside = rawLabel.startsWith(INSIDE_PREFIX);
		const label = isBegin || isInside ? rawLabel.slice(2) : rawLabel;
		const isContinuationPiece = token.word.startsWith(WORDPIECE_CONTINUATION_PREFIX);
		const surfaceWord = isContinuationPiece ? token.word.slice(WORDPIECE_CONTINUATION_PREFIX.length) : token.word;

		let start = token.start;
		let end = token.end;
		if (start === undefined || end === undefined) {
			const resolved = locateToken(sourceText, surfaceWord, searchCursor, {
				allowAdjacent: isContinuationPiece,
			});
			if (!resolved) {
				// Could not recover a position for this token; drop it rather than
				// fabricate a location the `Entity` contract promises is accurate.
				current = null;
				continue;
			}
			start = resolved.start;
			end = resolved.end;
		}
		searchCursor = end;

		const continuesCurrent = !isBegin && current !== null && current.label === label;

		if (continuesCurrent && current) {
			current.text = isContinuationPiece ? current.text + surfaceWord : `${current.text} ${surfaceWord}`;
			current.end = end;
			current.score = Math.min(current.score ?? 1, token.score);
		} else {
			current = {
				label,
				text: surfaceWord,
				start,
				end,
				score: token.score,
			};
			entities.push(current);
		}
	}

	return entities.filter((entity) => {
		if (threshold !== undefined && (entity.score ?? 0) < threshold) {
			return false;
		}
		if (categories && !categories.includes(entity.label)) {
			return false;
		}
		return true;
	});
}

/**
 * Find the next occurrence of `word` in `text` at or after `fromIndex`.
 * When `allowAdjacent` is set (WordPiece continuation tokens, e.g. "##ing"),
 * the search also accepts a match immediately abutting `fromIndex` with no
 * intervening whitespace, since continuation pieces are not separated by a
 * space from the token they extend.
 */
function locateToken(
	text: string,
	word: string,
	fromIndex: number,
	{ allowAdjacent }: { allowAdjacent: boolean },
): { start: number; end: number } | null {
	if (word.length === 0) return null;

	const searchFrom = allowAdjacent ? fromIndex : skipWhitespace(text, fromIndex);
	const lowerText = text.toLowerCase();
	const lowerWord = word.toLowerCase();
	const index = lowerText.indexOf(lowerWord, searchFrom);
	if (index === -1) return null;

	return { start: index, end: index + word.length };
}

function skipWhitespace(text: string, index: number): number {
	let i = index;
	while (i < text.length && /\s/.test(text[i] ?? "")) {
		i++;
	}
	return i;
}
