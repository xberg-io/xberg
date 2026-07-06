import { describe, it, expect } from "vitest";
import { detectPii, mergeNerEntities, selectPiiScan, type NerEntity } from "../src/redaction/detect.js";

describe("mergeNerEntities", () => {
  it("returns sorted regex findings unchanged when entities list is empty", () => {
    const text = "Call me at 555-123-4567 or email me.";
    const regex = detectPii(text);
    const merged = mergeNerEntities(regex, [], text);
    expect(merged).toEqual(regex);
  });

  it("appends NER entity as new finding when no span overlap with regex", () => {
    const text = "Contact Alice Smith for details.";
    const regex = detectPii(text);
    const entities: NerEntity[] = [{ category: "person", text: "Alice Smith", confidence: 0.92, start: 8, end: 19 }];
    const merged = mergeNerEntities(regex, entities, text);
    const names = merged.filter((f) => f.category === "NAME");
    expect(names).toHaveLength(1);
    expect(names[0]?.original).toBe("Alice Smith");
    expect(names[0]?.confidence).toBe(0.92);
  });

  it("maps entity category 'person' to PII category NAME", () => {
    const text = "Bob Jones signed.";
    const entities: NerEntity[] = [{ category: "person", text: "Bob Jones", confidence: 0.88, start: 0, end: 9 }];
    const merged = mergeNerEntities([], entities, text);
    expect(merged[0]?.category).toBe("NAME");
  });

  it("maps 'organization' and 'location' to ORG and LOCATION categories", () => {
    const text = "Acme Corp in Berlin.";
    const entities: NerEntity[] = [
      { category: "organization", text: "Acme Corp", confidence: 0.9, start: 0, end: 9 },
      { category: "location", text: "Berlin", confidence: 0.87, start: 13, end: 19 },
    ];
    const merged = mergeNerEntities([], entities, text);
    expect(merged.find((f) => f.category === "ORG")?.original).toBe("Acme Corp");
    expect(merged.find((f) => f.category === "LOCATION")?.original).toBe("Berlin");
  });

  it("deduplicates when NER span overlaps regex finding, keeping higher-confidence result", () => {
    const text = "Sent from 555-123-4567.";
    const regex = detectPii(text);
    const phoneRegex = regex.find((f) => f.category === "PHONE");
    expect(phoneRegex).toBeDefined();

    const highConfidenceEntity: NerEntity[] = [
      { category: "phone", text: "555-123-4567", confidence: 0.99, start: phoneRegex!.start, end: phoneRegex!.end },
    ];
    const merged = mergeNerEntities(regex, highConfidenceEntity, text);
    const phones = merged.filter((f) => f.category === "PHONE");
    expect(phones).toHaveLength(1);
    expect(phones[0]?.confidence).toBe(0.99);
  });

  it("keeps lower-confidence regex finding when NER confidence is not higher on overlap", () => {
    const text = "My SSN is 123-45-6789.";
    const regex = detectPii(text);
    const ssn = regex.find((f) => f.category === "SSN");
    expect(ssn).toBeDefined();
    const originalConfidence = ssn!.confidence;

    const lowConfidenceEntity: NerEntity[] = [
      { category: "custom", text: "123-45-6789", confidence: 0.5, start: ssn!.start, end: ssn!.end },
    ];
    const merged = mergeNerEntities(regex, lowConfidenceEntity, text);
    const ssns = merged.filter((f) => f.category === "SSN");
    expect(ssns).toHaveLength(1);
    expect(ssns[0]?.confidence).toBe(originalConfidence);
  });

  it("assigns sequential token format [CATEGORY_N] to NER-added findings", () => {
    const text = "Hello from Alice and Bob.";
    const entities: NerEntity[] = [
      { category: "person", text: "Alice", confidence: 0.9, start: 11, end: 16 },
      { category: "person", text: "Bob", confidence: 0.88, start: 21, end: 24 },
    ];
    const merged = mergeNerEntities([], entities, text);
    const tokens = merged.map((f) => f.token);
    expect(tokens).toContain("[NAME_1]");
    expect(tokens).toContain("[NAME_2]");
  });
});

describe("selectPiiScan (ingest_folder's eu_patterns routing)", () => {
  it("routes through detectPii when euPatterns is false", () => {
    const result = selectPiiScan("He was diagnosed with cancer.", false);
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(false);
  });

  it("routes through detectPiiEu when euPatterns is true", () => {
    const result = selectPiiScan("He was diagnosed with cancer.", true);
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
  });

  it("still applies filterCategories when euPatterns is true", () => {
    const result = selectPiiScan("Email: bob@example.com. He was diagnosed with cancer.", true, ["EMAIL"]);
    expect(result.some((f) => f.category === "EMAIL")).toBe(true);
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(false);
  });
});
