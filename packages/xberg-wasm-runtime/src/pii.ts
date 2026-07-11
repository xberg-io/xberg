import type { Entity } from "./types.js";

export interface PiiFinding {
	token: string;
	category: string;
	original: string;
	start: number;
	end: number;
	confidence: number;
}

const PATTERNS: Array<{ category: string; pattern: RegExp; confidence: number }> = [
	{ category: "EMAIL", pattern: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g, confidence: 0.95 },
	{ category: "PHONE", pattern: /\b(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b/g, confidence: 0.85 },
	{ category: "SSN", pattern: /\b\d{3}-\d{2}-\d{4}\b/g, confidence: 0.9 },
	{ category: "CREDIT_CARD", pattern: /\b(?:\d{4}[-\s]?){3}\d{4}\b/g, confidence: 0.9 },
	{ category: "IP_ADDRESS", pattern: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g, confidence: 0.8 },
	{ category: "DATE_ISO", pattern: /\b\d{4}-\d{2}-\d{2}\b/g, confidence: 0.7 },
	{ category: "DATE_MDY", pattern: /\b\d{1,2}\/\d{1,2}\/\d{2,4}\b/g, confidence: 0.7 },
	{ category: "IBAN", pattern: /\b[A-Z]{2}\d{2}[A-Z0-9]{4,30}\b/g, confidence: 0.85 },
	{ category: "SWIFT_BIC", pattern: /\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b/g, confidence: 0.8 },
	{ category: "POSTAL_CODE_US", pattern: /\b\d{5}(?:-\d{4})?\b/g, confidence: 0.75 },
	{ category: "POSTAL_CODE_UK", pattern: /\b[A-Z]{1,2}\d[A-Z\d]?\s?\d[A-Z]{2}\b/g, confidence: 0.75 },
];

/**
 * Deterministic regex-based PII detection. Ported from
 * mcp-server/src/redaction/detect.ts (pure function, no Node-specific I/O —
 * runs identically in Node and the browser) so this package's structured PII
 * coverage (email/phone/SSN/etc.) does not depend on any NER model's label
 * set — deliberately kept as a duplicate rather than a shared package
 * (packages/xberg-wasm-runtime has no dependency on mcp-server today).
 */
export function detectPii(text: string, filterCategories?: string[]): PiiFinding[] {
	const findings: PiiFinding[] = [];
	const counters: Record<string, number> = {};

	for (const { category, pattern, confidence } of PATTERNS) {
		if (filterCategories && !filterCategories.includes(category)) continue;

		const regex = new RegExp(pattern.source, pattern.flags);
		let match: RegExpExecArray | null;
		while ((match = regex.exec(text)) !== null) {
			counters[category] = (counters[category] ?? 0) + 1;
			findings.push({
				token: `[${category}_${counters[category]}]`,
				category,
				original: match[0],
				start: match.index,
				end: match.index + match[0].length,
				confidence,
			});
		}
	}

	return findings.sort((a, b) => a.start - b.start);
}

const NER_LABEL_TO_PII: Record<string, string> = {
	// GLiNER2 / in-binary Candle privacy adapter labels (lowercased).
	person: "NAME",
	organization: "ORG",
	location: "LOCATION",
	email: "EMAIL",
	phone: "PHONE",
	date: "DATE",
	money: "MONEY",
	url: "URL",
	// transformers.js `token-classification` labels, e.g. Xenova/bert-base-NER
	// (CoNLL-2003: PER/ORG/LOC/MISC). mergeNerEntities lowercases labels before
	// lookup, so these keys must be the lowercased CoNLL forms.
	per: "NAME",
	org: "ORG",
	loc: "LOCATION",
	misc: "MISC",
};

function spansOverlap(a: PiiFinding, b: { start: number; end: number }): boolean {
	return a.start < b.end && b.start < a.end;
}

/**
 * Merge regex findings with NER entities (this package's `Entity` shape:
 * `label`/`score`, not mcp-server's `NerEntity` shape of `category`/
 * `confidence` — the field names differ between the two packages by design,
 * this function is the adapter).
 */
export function mergeNerEntities(
	regex: PiiFinding[],
	entities: Entity[],
	filterCategories?: string[],
): PiiFinding[] {
	const findings = [...regex];
	const counters: Record<string, number> = {};
	for (const f of findings) {
		counters[f.category] = Math.max(counters[f.category] ?? 0, Number(f.token.match(/_(\d+)\]$/)?.[1] ?? 0));
	}

	for (const entity of entities) {
		const category = NER_LABEL_TO_PII[entity.label.toLowerCase()] ?? `NER_${entity.label.toUpperCase()}`;
		if (filterCategories && !filterCategories.includes(category)) continue;
		const { text: entityText, start, end } = entity;
		const entityConfidence = entity.score ?? 0.8;

		const overlap = findings.find((f) => spansOverlap(f, { start, end }));
		if (overlap) {
			if (entityConfidence > overlap.confidence) {
				counters[category] = (counters[category] ?? 0) + 1;
				overlap.category = category;
				overlap.confidence = entityConfidence;
				overlap.original = entityText;
				overlap.start = start;
				overlap.end = end;
				overlap.token = `[${category}_${counters[category]}]`;
			}
			continue;
		}

		counters[category] = (counters[category] ?? 0) + 1;
		findings.push({
			token: `[${category}_${counters[category]}]`,
			category,
			original: entityText,
			start,
			end,
			confidence: entityConfidence,
		});
	}

	return findings.sort((a, b) => a.start - b.start);
}

export function groupByCategory(findings: PiiFinding[]): Record<string, number> {
	const grouped: Record<string, number> = {};
	for (const f of findings) {
		grouped[f.category] = (grouped[f.category] ?? 0) + 1;
	}
	return grouped;
}

/**
 * Runs regex PII detection and merges it with an already-computed NER
 * result (from either the injected JS NER path or, once wired, the
 * in-binary Candle fallback — this function does not care which produced
 * `nerResult`, or whether it's empty). Regex-only detection still functions
 * as a floor when `nerResult` is `[]`.
 */
export function detectPiiWithNer(text: string, nerResult: Entity[], filterCategories?: string[]): PiiFinding[] {
	const regexFindings = detectPii(text, filterCategories);
	return mergeNerEntities(regexFindings, nerResult, filterCategories);
}
