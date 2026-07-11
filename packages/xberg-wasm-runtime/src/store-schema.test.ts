import { describe, it, expect } from "vitest";
import { sanitizeTableName, SCHEMA_SQL, vecTableName, createVecTableSql } from "./store-schema";

describe("store-schema", () => {
	it("sanitizes a collection name with hyphens into a valid identifier", () => {
		expect(sanitizeTableName("test-docs")).toBe("test_docs");
	});
	it("sanitizes a collection name with spaces and special chars", () => {
		expect(sanitizeTableName("my collection!@#")).toBe("my_collection___");
	});
	it("produces a deterministic vec table name", () => {
		expect(vecTableName("test-docs")).toMatch(/^vec_test_docs_[0-9a-f]{8}$/);
	});
	it("does not map distinct collection names to the same vec table", () => {
		expect(vecTableName("test-docs")).not.toBe(vecTableName("test_docs"));
	});
	it("produces a valid CREATE VIRTUAL TABLE statement with the given dimension", () => {
		const sql = createVecTableSql("test-docs", 384);
		expect(sql).toContain(`CREATE VIRTUAL TABLE IF NOT EXISTS ${vecTableName("test-docs")}`);
		expect(sql).toContain("USING vec0");
		expect(sql).toContain("FLOAT[384]");
	});
	it.each([0, -1, 1.5, Number.NaN, 65_537])("rejects unsafe vector dimensions: %s", (dimension) => {
		expect(() => createVecTableSql("docs", dimension)).toThrow(/vector dimension/);
	});
	it("SCHEMA_SQL defines the collections, documents, chunks, and graph_edges tables", () => {
		expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS collections");
		expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS documents");
		expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS chunks");
		expect(SCHEMA_SQL).toContain("embedding_dim");
		expect(SCHEMA_SQL).toContain("full_text");
		expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS graph_edges");
		expect(SCHEMA_SQL).toContain("source");
		expect(SCHEMA_SQL).toContain("target");
		expect(SCHEMA_SQL).toContain("label");
	});
	it("SCHEMA_SQL defines an FTS5 external-content table synced to chunks", () => {
		expect(SCHEMA_SQL).toContain("CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5");
		expect(SCHEMA_SQL).toContain("content='chunks'");
		expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks");
		expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks");
		expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks");
	});
});
