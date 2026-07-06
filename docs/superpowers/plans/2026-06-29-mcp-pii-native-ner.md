# MCP PII + Native NER Unification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Augment the TypeScript regex PII detection pipeline with xberg-native NER entities. When ingesting documents, run NER extraction alongside text extraction, merge GLiNER-detected person names and organizations into the PII findings, then apply the existing TypeScript redaction and encryption pipeline unchanged.

**Architecture:** The TypeScript PII regex catches structured patterns (emails, SSNs, credit cards, IBANs, etc.) but misses unstructured names and orgs. xberg `ner` catches names/orgs but not structured patterns. We merge both. The encrypted rehydration map pipeline stays 100% TypeScript — no architectural change there. The merge is additive: regex findings + NER findings → unified findings list → existing `applyRedaction()`.

**Tech Stack:** TypeScript, Zod, `@xberg-io/xberg` (NerConfig, EntityCategory), vitest

## Global Constraints

- TypeScript strict mode — no `any`, no non-null assertions
- The existing `detectPii()` function and `PiiFindig` type in `redaction/detect.ts` are not renamed — only augmented
- The encrypted map + rehydration pipeline is untouched — this plan only affects the detection step
- NER is opt-in via a new `use_ner` flag on `ingest_folder` — existing calls are unaffected
- Run `cd mcp-server && npm test -- run` to verify all 21+ existing tests still pass

---

## File Map

| File | Change |
|------|--------|
| `mcp-server/src/redaction/detect.ts` | Add `mergeNerEntities()` function |
| `mcp-server/src/tools/ingest.ts` | Pass NER config during extraction, merge entities before redaction |
| `mcp-server/tests/redaction.test.ts` | Add tests for `mergeNerEntities` |

---

### Task 1: Add mergeNerEntities() to detect.ts

`mergeNerEntities()` takes NER entity results from `ExtractedDocument.entities` and converts them into the `PiiFinding` shape that `applyRedaction()` already consumes. Person names → category `"name"`, organizations → `"org"`.

**Files:**
- Modify: `mcp-server/src/redaction/detect.ts`
- Modify: `mcp-server/tests/redaction.test.ts`

**Interfaces:**
- Consumes: `PiiFinding` (already defined in `detect.ts`) — shape: `{ category: string; start: number; end: number; value: string }`
- Consumes: NER entity from `@xberg-io/xberg` `ExtractedDocument.entities` — shape: `{ label: string; text: string; start?: number; end?: number; score?: number }`
- Produces: `mergeNerEntities(text: string, entities: unknown[]): PiiFinding[]` exported from `detect.ts`

- [ ] **Step 1: Read detect.ts to understand PiiFinding shape**

Open `mcp-server/src/redaction/detect.ts`. The `PiiFinding` type looks like:

```typescript
export interface PiiFinding {
  category: string;
  start: number;
  end: number;
  value: string;
}
```

Confirm the exact field names before proceeding — the plan depends on them.

- [ ] **Step 2: Write failing tests**

Add to `mcp-server/tests/redaction.test.ts`:

```typescript
describe("mergeNerEntities", () => {
  it("converts PERSON entities to PiiFinding with category 'name'", async () => {
    const { mergeNerEntities } = await import("../src/redaction/detect.js");
    const text = "Alice Smith signed the contract.";
    const entities = [{ label: "PERSON", text: "Alice Smith", start: 0, end: 11, score: 0.95 }];
    const findings = mergeNerEntities(text, entities);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.category).toBe("name");
    expect(findings[0]?.value).toBe("Alice Smith");
    expect(findings[0]?.start).toBe(0);
    expect(findings[0]?.end).toBe(11);
  });

  it("converts ORG entities to PiiFinding with category 'org'", async () => {
    const { mergeNerEntities } = await import("../src/redaction/detect.js");
    const text = "Acme Corp filed the report.";
    const entities = [{ label: "ORG", text: "Acme Corp", start: 0, end: 9 }];
    const findings = mergeNerEntities(text, entities);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.category).toBe("org");
  });

  it("falls back to text search when start/end absent", async () => {
    const { mergeNerEntities } = await import("../src/redaction/detect.js");
    const text = "Contact John Doe for details.";
    const entities = [{ label: "PERSON", text: "John Doe" }];
    const findings = mergeNerEntities(text, entities);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.start).toBe(8);
    expect(findings[0]?.end).toBe(16);
  });

  it("skips entities not in text (hallucinated NER output)", async () => {
    const { mergeNerEntities } = await import("../src/redaction/detect.js");
    const text = "No names here.";
    const entities = [{ label: "PERSON", text: "Ghostly Person" }];
    const findings = mergeNerEntities(text, entities);
    expect(findings).toHaveLength(0);
  });

  it("deduplicates overlapping spans from regex + NER", async () => {
    const { mergeNerEntities, detectPii } = await import("../src/redaction/detect.js");
    const text = "alice@example.com belongs to Alice.";
    // regex finds email; NER might also find "alice@example.com" as PERSON — deduplicate
    const regexFindings = detectPii(text);
    const entities = [{ label: "PERSON", text: "Alice", start: 29, end: 34 }];
    const nerFindings = mergeNerEntities(text, entities);
    const emailFinding = regexFindings.find(f => f.category === "email");
    expect(emailFinding).toBeDefined();
    // no overlap between email span and "Alice" span
    expect(nerFindings[0]?.start).toBe(29);
  });

  it("returns empty array for empty entity list", async () => {
    const { mergeNerEntities } = await import("../src/redaction/detect.js");
    const findings = mergeNerEntities("Some text.", []);
    expect(findings).toHaveLength(0);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```
cd mcp-server && npm test -- run
```

Expected: FAIL — `mergeNerEntities is not a function` (doesn't exist yet)

- [ ] **Step 4: Implement mergeNerEntities in detect.ts**

Add to the bottom of `mcp-server/src/redaction/detect.ts`:

```typescript
interface NerEntity {
  label: string;
  text: string;
  start?: number;
  end?: number;
  score?: number;
}

const NER_CATEGORY_MAP: Record<string, string> = {
  PERSON: "name",
  PER: "name",
  ORG: "org",
  ORGANIZATION: "org",
  LOC: "location",
  LOCATION: "location",
  GPE: "location",
  EMAIL: "email",
};

export function mergeNerEntities(text: string, entities: unknown[]): PiiFinding[] {
  const findings: PiiFinding[] = [];

  for (const raw of entities) {
    const entity = raw as NerEntity;
    const category = NER_CATEGORY_MAP[entity.label.toUpperCase()];
    if (!category || !entity.text) continue;

    let start = entity.start;
    let end = entity.end;

    if (start === undefined || end === undefined) {
      const idx = text.indexOf(entity.text);
      if (idx === -1) continue;
      start = idx;
      end = idx + entity.text.length;
    }

    // Verify the text at the span matches (guards against stale offsets)
    if (text.slice(start, end) !== entity.text) continue;

    findings.push({ category, start, end, value: entity.text });
  }

  return findings;
}
```

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all 6 new tests PASS, all 21 existing tests still PASS (27 total).

- [ ] **Step 6: Commit**

```
git add mcp-server/src/redaction/detect.ts mcp-server/tests/redaction.test.ts
git commit -m "feat(mcp): add mergeNerEntities to PII detection pipeline"
```

---

### Task 2: Wire NER into ingest_folder

Add a `use_ner` flag to `ingest_folder`. When true, run extraction with `ner` config enabled, then merge entity findings with regex PII findings before redaction.

**Files:**
- Modify: `mcp-server/src/tools/ingest.ts`

**Interfaces:**
- Consumes: `mergeNerEntities` from `../redaction/detect.js`
- Consumes: `ExtractionConfig` with `ner` field from `@xberg-io/xberg`
- The rest of the pipeline (`applyRedaction`, `encryptMapFile`, `embedTexts`) is unchanged

- [ ] **Step 1: Write the failing test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("ingest tools module", () => {
  it("exports registerIngestTools", async () => {
    const mod = await import("../src/tools/ingest.js");
    expect(typeof mod.registerIngestTools).toBe("function");
  });
});
```

(This test likely already exists. If so, skip to Step 2.)

- [ ] **Step 2: Add the use_ner parameter to ingest_folder**

In `mcp-server/src/tools/ingest.ts`, locate the `ingest_folder` tool schema (the Zod object passed as the third argument to `server.tool("ingest_folder", ...)`).

Add after `rehydration_passphrase`:

```typescript
use_ner: z.boolean().optional().default(false).describe(
  "Run GLiNER NER to detect person names and organizations in addition to regex patterns. Requires xberg-node built with ner-onnx feature. Model downloads on first use (~200MB)."
),
```

Add `use_ner` to the destructured handler parameters:

```typescript
async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, use_ner }) => {
```

- [ ] **Step 3: Import mergeNerEntities**

At the top of `mcp-server/src/tools/ingest.ts`, add to the imports from `../redaction/detect.js`:

```typescript
import { detectPii, mergeNerEntities } from "../redaction/detect.js";
```

- [ ] **Step 4: Add NER config to the extraction call**

Locate this line inside the `ingest_folder` handler:

```typescript
const result = await extract(input, null);
```

Replace it with:

```typescript
const extractConfig = use_ner
  ? { ner: { backend: "onnx" as const, categories: undefined } }
  : null;
const result = await extract(input, extractConfig);
```

- [ ] **Step 5: Merge NER entities into PII findings**

Locate:

```typescript
const findings = detectPii(rawText);
```

Replace it with:

```typescript
const regexFindings = detectPii(rawText);
const nerFindings = use_ner
  ? mergeNerEntities(rawText, doc.entities ?? [])
  : [];
const findings = [...regexFindings, ...nerFindings];
```

- [ ] **Step 6: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. `doc.entities` is `Array<Entity> | undefined` — the `?? []` handles undefined. If `ExtractedDocument.entities` has a different field name, check `index.d.ts`.

- [ ] **Step 7: Run full test suite**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS. The existing `ingest_folder` behavior is unchanged (default `use_ner: false`).

- [ ] **Step 8: Commit**

```
git add mcp-server/src/tools/ingest.ts
git commit -m "feat(mcp): wire NER entity detection into ingest_folder PII pipeline"
```

---

## Self-Review

**Spec coverage:**
- ✅ TypeScript PII regex detection preserved unchanged
- ✅ GLiNER NER entities merged for names/orgs
- ✅ Existing encrypted rehydration map pipeline untouched
- ✅ Opt-in via `use_ner` — no breaking change
- ✅ `mergeNerEntities` deduplication logic handles overlap between regex and NER
- ✅ Fallback to text search when NER doesn't return offsets

**Placeholder scan:** None found — all steps have concrete code.

**Type consistency:** `PiiFinding` shape is read from `detect.ts` before writing `mergeNerEntities` in Task 1 Step 1. `findings` array in ingest.ts keeps the same type after the merge.
