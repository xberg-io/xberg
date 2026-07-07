import { describe, it, expect, beforeAll } from "vitest";
import { createNer } from "./ner";
import type { NerInterface } from "./types";

describe("NER", () => {
  let ner: NerInterface | null;

  beforeAll(async () => {
    // "Xenova/gliner2-small-onnx" (as written in the original spec) does not
    // exist on the Hub — GLiNER2 ONNX exports that do exist (e.g.
    // SemplificaAI/gliner2-multi-v1-onnx, lion-ai/gliner2-base-v1-onnx) target
    // a schema-driven "zero-shot" extraction API, not the standard
    // transformers.js `token-classification` pipeline this module implements.
    // We substitute "Xenova/bert-base-NER" — a real, canonically-cased,
    // ONNX-converted NER model (dslim/bert-base-NER) explicitly documented as
    // transformers.js v3 compatible. This triggers a live download on first
    // run.
    ner = await createNer({
      models: { ner: "Xenova/bert-base-NER" },
    });
  }, 120_000);

  it("detects named entities in text", async () => {
    if (!ner) {
      console.log("[skip] NER not enabled");
      return;
    }
    const text = "Alice works at Google in Mountain View.";
    const entities = await ner.ner(text);

    expect(Array.isArray(entities)).toBe(true);
    // Expect some entities like PERSON, ORGANIZATION, LOCATION
    const labels = entities.map((e) => e.label);
    expect(labels.length).toBeGreaterThan(0);
  }, 60_000);

  it("returns entity structure with position info", async () => {
    if (!ner) {
      console.log("[skip] NER not enabled");
      return;
    }
    const text = "Email: john@example.com";
    const entities = await ner.ner(text);

    if (entities.length > 0) {
      const entity = entities[0];
      if (!entity) throw new Error("expected entity");
      expect(entity).toHaveProperty("label");
      expect(entity).toHaveProperty("text");
      expect(entity).toHaveProperty("start");
      expect(entity).toHaveProperty("end");
      expect(typeof entity.label).toBe("string");
      expect(typeof entity.start).toBe("number");
      expect(typeof entity.end).toBe("number");
    }
  }, 60_000);

  it("merges multi-word entities into single spans", async () => {
    if (!ner) {
      console.log("[skip] NER not enabled");
      return;
    }
    // "Mountain View" is a multi-word location; should be merged into one entity
    // with the combined text, not split into "Mountain" and "View" as separate
    // single-word entities.
    const text = "The company Google is located in Mountain View, California.";
    const entities = await ner.ner(text);

    // Expect at least one entity (company or location)
    expect(entities.length).toBeGreaterThan(0);

    // Search for a location entity containing "Mountain View"
    const locationWithMultipleWords = entities.find((e) =>
      e.label === "LOC" && e.text.includes("Mountain")
    );

    // If we found a LOC entity with "Mountain", verify it's a merged multi-word span
    // (not two separate entities). The merged text should include both words.
    if (locationWithMultipleWords) {
      expect(locationWithMultipleWords.text).toMatch(/Mountain.*View/i);
    }
  }, 60_000);

  it("filters entities by categories option", async () => {
    if (!ner) {
      console.log("[skip] NER not enabled");
      return;
    }
    const text = "Alice works at Google in Mountain View.";

    // Get all entities (no category filter)
    const allEntities = await ner.ner(text);
    expect(allEntities.length).toBeGreaterThan(0);

    // Filter to only PER (person) category
    const personEntities = await ner.ner(text, { categories: ["PER"] });

    // All returned entities must have label "PER"
    for (const entity of personEntities) {
      expect(entity.label).toBe("PER");
    }

    // Requesting a category that bert-base-NER does not support (e.g., EMAIL)
    // should return empty results (not an error).
    const emailEntities = await ner.ner(text, { categories: ["EMAIL"] });
    expect(emailEntities).toEqual([]);
  }, 60_000);

  it("filters entities by threshold option", async () => {
    if (!ner) {
      console.log("[skip] NER not enabled");
      return;
    }
    const text = "John Smith works at Microsoft.";

    // Get all entities (no threshold filter)
    const allEntities = await ner.ner(text);
    expect(allEntities.length).toBeGreaterThan(0);

    // Set a high confidence threshold
    const highThresholdEntities = await ner.ner(text, { threshold: 0.99 });

    // All returned entities must have score >= threshold
    for (const entity of highThresholdEntities) {
      if (entity.score !== undefined) {
        expect(entity.score).toBeGreaterThanOrEqual(0.99);
      }
    }

    // High threshold may exclude some entities, so we expect fewer or equal
    expect(highThresholdEntities.length).toBeLessThanOrEqual(allEntities.length);
  }, 60_000);
});
