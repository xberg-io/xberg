import { describe, it, expect } from "vitest";
import { detectPii, groupByCategory, detectPiiWithNer } from "./pii";
import type { Entity } from "./types";

describe("detectPii", () => {
	it("detects email addresses", () => {
		const findings = detectPii("Contact us at info@example.com for details.");
		expect(findings).toHaveLength(1);
		expect(findings[0]?.category).toBe("EMAIL");
		expect(findings[0]?.original).toBe("info@example.com");
		expect(findings[0]?.token).toBe("[EMAIL_1]");
		expect(findings[0]?.confidence).toBeGreaterThan(0.9);
	});

	it("detects phone numbers", () => {
		const findings = detectPii("Call us at 555-867-5309.");
		expect(findings.some((f) => f.category === "PHONE")).toBe(true);
	});

	it("detects SSN", () => {
		const findings = detectPii("SSN: 123-45-6789");
		expect(findings.some((f) => f.category === "SSN")).toBe(true);
	});

	it("detects credit card numbers", () => {
		const findings = detectPii("Card: 4111 1111 1111 1111");
		expect(findings.some((f) => f.category === "CREDIT_CARD")).toBe(true);
	});

	it("filters by category", () => {
		const findings = detectPii("Email: test@test.com, SSN: 123-45-6789", ["EMAIL"]);
		expect(findings.every((f) => f.category === "EMAIL")).toBe(true);
	});

	it("returns empty array for clean text", () => {
		expect(detectPii("Hello, how are you today?")).toHaveLength(0);
	});
});

describe("groupByCategory", () => {
	it("counts findings per category", () => {
		const findings = detectPii("a@b.com c@d.com 555-123-4567");
		const groups = groupByCategory(findings);
		expect(groups["EMAIL"]).toBe(2);
		expect(groups["PHONE"]).toBe(1);
	});
});

describe("detectPiiWithNer", () => {
	it("merges regex findings with NER entities using the Entity (label/score) shape", () => {
		const nerResult: Entity[] = [{ label: "person", text: "Alice", start: 0, end: 5, score: 0.95 }];
		const findings = detectPiiWithNer("Alice's email is a@b.com", nerResult);
		expect(findings.some((f) => f.category === "EMAIL")).toBe(true);
		expect(findings.some((f) => f.category === "NAME" && f.original === "Alice")).toBe(true);
	});

	it("still detects regex PII when nerResult is empty", () => {
		const findings = detectPiiWithNer("Contact: a@b.com", []);
		expect(findings.some((f) => f.category === "EMAIL")).toBe(true);
	});

	it("maps CoNLL-style PER/ORG/LOC labels (e.g. Xenova/bert-base-NER) to PII categories", () => {
		const nerResult: Entity[] = [
			{ label: "PER", text: "Alice", start: 0, end: 5, score: 0.95 },
			{ label: "ORG", text: "Acme", start: 10, end: 14, score: 0.9 },
			{ label: "LOC", text: "Paris", start: 20, end: 25, score: 0.9 },
		];
		const findings = detectPiiWithNer("Alice works at Acme in Paris", nerResult);
		expect(findings.some((f) => f.category === "NAME" && f.original === "Alice")).toBe(true);
		expect(findings.some((f) => f.category === "ORG" && f.original === "Acme")).toBe(true);
		expect(findings.some((f) => f.category === "LOCATION" && f.original === "Paris")).toBe(true);
	});

	it("filters NER entities by filterCategories in detectPiiWithNer", () => {
		const nerResult: Entity[] = [
			{ label: "PER", text: "Alice", start: 0, end: 5, score: 0.95 },
			{ label: "ORG", text: "Acme", start: 10, end: 14, score: 0.9 },
		];
		const findings = detectPiiWithNer("Alice at Acme", nerResult, ["ORG"]);
		expect(findings.some((f) => f.category === "ORG")).toBe(true);
		expect(findings.some((f) => f.category === "NAME")).toBe(false);
	});

	it("regenerates the token to match the new category when a higher-confidence NER entity overrides a regex finding", () => {
		// "555-123-4567" overlaps a PHONE regex match, but the NER entity has higher
		// confidence and is actually a person's name — the resulting finding must carry
		// a NAME token, not a stale PHONE token.
		const nerResult: Entity[] = [{ label: "person", text: "555-123-4567", start: 0, end: 12, score: 0.99 }];
		const findings = detectPiiWithNer("555-123-4567 called", nerResult);

		expect(findings).toHaveLength(1);
		expect(findings[0]?.category).toBe("NAME");
		expect(findings[0]?.token).toBe("[NAME_1]");
	});
});
