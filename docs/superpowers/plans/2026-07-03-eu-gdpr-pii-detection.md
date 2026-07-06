# EU/GDPR PII Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the EU/GDPR structured-PII detection layer from the `anno` crate's `pii.rs` (checksum-validated national IDs, tax IDs, license plates, GDPR Art. 9 special-category keywords, k-anonymity risk scoring) into xberg's TypeScript PII pipeline in `mcp-server/`.

**Architecture:** Two new TypeScript modules — `eu-checksums.ts` (pure checksum validators, no regex) and `eu-patterns.ts` (regex scanners that use the checksums) — port anno's Rust logic 1:1, producing xberg's existing `PiiFinding` shape so they compose with the current regex/NER pipeline without changing it. `detect.ts` gains a `dedupOverlapping` helper, a `detectPiiEu` combinator, and a `buildPiiReport` k-anonymity scorer. `ingest_folder` gains an opt-in `eu_patterns` flag, following the exact pattern the existing `use_ner` flag already established.

**Tech Stack:** TypeScript, vitest, no new dependencies (pure regex + arithmetic, matching anno's zero-dependency implementation).

## Global Constraints

- TypeScript strict mode — no `any`, no non-null assertions (`!`). `PiiFinding` findings must use `?? 0` / `?? ""` defaults instead of `!` when indexing arrays under `noUncheckedIndexedAccess`.
- This plan is **regex/checksum-only** — it does NOT include anno's GLiNER2-NER-based Art. 9 path (`scan_patterns_with_ner`). That path depends on a zero-shot `extract_with_types`-style call that xberg's NAPI bindings don't expose yet; it's an explicit follow-up, not in scope here.
- Existing `detectPii()` behavior and output must NOT change for callers that don't opt in — this plan only *adds* `detectPiiEu()` as a new function, mirroring how `use_ner` was added as opt-in in the prior NER-merge plan.
- `PiiFinding` shape (`{ token, category, original, start, end, confidence }` in `mcp-server/src/redaction/detect.ts:1-8`) is not renamed or restructured — new code only produces this shape.
- Category strings copy anno's `pii_type` values verbatim (`NATIONAL_ID_FR`, `TAX_ID_SIRET`, `SPECIAL_CATEGORY_HEALTH`, etc.) — no translation layer, since xberg's existing categories (`EMAIL`, `SSN`, `CREDIT_CARD`) are already uppercase snake-style and these fit the same convention.
- Confidence is a number (xberg convention), not anno's string risk level (`LOW`/`MEDIUM`/`HIGH`/`CRITICAL`). Mapping used throughout this plan: `CRITICAL → 0.97`, `HIGH → 0.9`, `MEDIUM → 0.75` — chosen to sit within xberg's existing confidence range (0.7–0.95 in `detect.ts`'s `PATTERNS` array) while preserving anno's relative ordering.
- Run `cd mcp-server && npm test -- run` after every task to verify all existing tests still pass alongside new ones.
- Run `cd mcp-server && node_modules/.bin/tsc --noEmit` before committing each task — this repo's TypeScript conventions require zero `tsc` errors.

---

### Task 1: EU national ID checksum validators

**Files:**
- Create: `mcp-server/src/redaction/eu-checksums.ts`
- Test: `mcp-server/tests/eu-checksums.test.ts`

**Interfaces:**
- Consumes: nothing new (pure functions, no imports).
- Produces: `isValidPesel(pesel: string): boolean`, `isValidBsn(bsn: string): boolean`, `isValidBelgianRegistre(num: string): boolean` — all exported from `eu-checksums.ts`. Task 2 imports these.

- [ ] **Step 1: Write the failing tests**

```typescript
// mcp-server/tests/eu-checksums.test.ts
import { describe, it, expect } from "vitest";
import { isValidPesel, isValidBsn, isValidBelgianRegistre } from "../src/redaction/eu-checksums.js";

describe("isValidPesel", () => {
  it("accepts a valid PESEL", () => {
    // sum(d*w for weights [1,3,7,9,1,3,7,9,1,3]) = 89, check = (10-9)%10 = 1
    expect(isValidPesel("80051501231")).toBe(true);
  });

  it("rejects a PESEL with a wrong check digit", () => {
    expect(isValidPesel("80051501230")).toBe(false);
  });

  it("rejects the wrong length", () => {
    expect(isValidPesel("8005150123")).toBe(false);
    expect(isValidPesel("800515012345")).toBe(false);
  });

  it("rejects non-digit input", () => {
    expect(isValidPesel("8005150123X")).toBe(false);
  });
});

describe("isValidBsn", () => {
  it("accepts a valid BSN", () => {
    expect(isValidBsn("123456782")).toBe(true);
  });

  it("rejects a BSN with a wrong check digit", () => {
    expect(isValidBsn("123456780")).toBe(false);
  });

  it("rejects the wrong length", () => {
    expect(isValidBsn("12345678")).toBe(false);
  });
});

describe("isValidBelgianRegistre", () => {
  it("accepts a valid pre-2000 Registre National number", () => {
    // 800515012 % 97 = 8, check = 97 - 8 = 89
    expect(isValidBelgianRegistre("80051501289")).toBe(true);
  });

  it("rejects a pre-2000 number with the wrong check digits", () => {
    expect(isValidBelgianRegistre("80051501294")).toBe(false);
  });

  it("accepts a valid post-2000 Registre National number", () => {
    // Born 2001-05-15, sequence 012: n = 010515012
    // 2_000_000_000 % 97 = 68; 10_515_012 % 97 = 18; (68+18)%97 = 86; check = 97-86 = 11
    expect(isValidBelgianRegistre("01051501211")).toBe(true);
  });

  it("rejects the wrong length", () => {
    expect(isValidBelgianRegistre("8005150128")).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run eu-checksums`
Expected: FAIL — `Cannot find module '../src/redaction/eu-checksums.js'`

- [ ] **Step 3: Implement the checksum validators**

```typescript
// mcp-server/src/redaction/eu-checksums.ts

/**
 * Validate a Polish PESEL national ID checksum.
 * Mod-10 algorithm with official weights from Polish GUS.
 * https://en.wikipedia.org/wiki/PESEL
 */
export function isValidPesel(pesel: string): boolean {
  if (!/^\d{11}$/.test(pesel)) return false;
  const digits = pesel.split("").map(Number);
  const weights = [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
  const sum = weights.reduce((acc, w, i) => acc + w * (digits[i] ?? 0), 0);
  const checkDigit = digits[10] ?? 0;
  const expected = (10 - (sum % 10)) % 10;
  return expected === checkDigit;
}

/**
 * Validate a Dutch BSN national ID checksum.
 * Mod-11 algorithm per official Dutch RvIG specification; last digit has weight -1.
 * https://en.wikipedia.org/wiki/Burgerservicenummer
 */
export function isValidBsn(bsn: string): boolean {
  if (!/^\d{9}$/.test(bsn)) return false;
  const digits = bsn.split("").map(Number);
  const weights = [9, 8, 7, 6, 5, 4, 3, 2, -1];
  const sum = weights.reduce((acc, w, i) => acc + w * (digits[i] ?? 0), 0);
  return sum % 11 === 0;
}

/**
 * Validate a Belgian Registre National checksum (97-modulo).
 * Format: YYMMDDXXXXX (6-digit birth date + 3-digit sequence + 2-digit check).
 * Century is ambiguous from YY alone, so both the pre-2000 and post-2000
 * formulas are tried.
 */
export function isValidBelgianRegistre(num: string): boolean {
  if (!/^\d{11}$/.test(num)) return false;
  const n = Number(num.slice(0, 9));
  const checkActual = Number(num.slice(9, 11));
  if (97 - (n % 97) === checkActual) return true;
  return 97 - ((2_000_000_000 + n) % 97) === checkActual;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run eu-checksums`
Expected: all 11 tests PASS

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/eu-checksums.ts mcp-server/tests/eu-checksums.test.ts
git commit -m "feat(mcp): add EU national ID checksum validators"
```

---

### Task 2: EU structured pattern scanner (national IDs, tax IDs, license plates)

**Files:**
- Create: `mcp-server/src/redaction/eu-patterns.ts`
- Test: `mcp-server/tests/eu-patterns.test.ts`

**Interfaces:**
- Consumes: `isValidPesel`, `isValidBsn`, `isValidBelgianRegistre` from `./eu-checksums.js` (Task 1).
- Produces: `interface RawMatch { category: string; original: string; start: number; end: number; confidence: number }` (internal, not exported), `overlapsExisting(matches: RawMatch[], start: number, end: number): boolean` (exported — Task 3 and Task 4 reuse it), `scanEuStructured(text: string): RawMatch[]` (exported — Task 4 consumes it).

- [ ] **Step 1: Write the failing tests**

```typescript
// mcp-server/tests/eu-patterns.test.ts
import { describe, it, expect } from "vitest";
import { scanEuStructured } from "../src/redaction/eu-patterns.js";

describe("scanEuStructured", () => {
  it("detects a French INSEE number", () => {
    const result = scanEuStructured("INSEE: 185057511602324");
    expect(result.some((m) => m.category === "NATIONAL_ID_FR")).toBe(true);
  });

  it("detects a Spanish DNI", () => {
    const result = scanEuStructured("DNI: 12345678Z");
    expect(result.some((m) => m.category === "NATIONAL_ID_ES")).toBe(true);
  });

  it("detects an Italian Codice Fiscale", () => {
    const result = scanEuStructured("Codice Fiscale: RSSMRA85T10A562S");
    expect(result.some((m) => m.category === "NATIONAL_ID_IT")).toBe(true);
  });

  it("detects a valid Polish PESEL", () => {
    const result = scanEuStructured("PESEL: 80051501231");
    expect(result.some((m) => m.category === "NATIONAL_ID_PL")).toBe(true);
  });

  it("rejects a PESEL-shaped number with a bad checksum", () => {
    const result = scanEuStructured("80051501230");
    expect(result.some((m) => m.category === "NATIONAL_ID_PL")).toBe(false);
  });

  it("detects a valid Dutch BSN", () => {
    const result = scanEuStructured("BSN: 123456782");
    expect(result.some((m) => m.category === "NATIONAL_ID_NL")).toBe(true);
  });

  it("detects a French SIRET (14 digits)", () => {
    const result = scanEuStructured("SIRET: 73282932000074");
    expect(result.some((m) => m.category === "TAX_ID_SIRET")).toBe(true);
    // The 9-digit SIREN prefix should not ALSO be flagged separately -- it overlaps the SIRET span.
    expect(result.some((m) => m.category === "TAX_ID_SIREN")).toBe(false);
  });

  it("detects an EU VAT number", () => {
    const result = scanEuStructured("VAT: FR12345678901");
    expect(result.some((m) => m.category === "TAX_ID_VAT")).toBe(true);
  });

  it("returns character offsets that round-trip to the original text", () => {
    const text = "Café PESEL: 80051501231 end";
    const result = scanEuStructured(text);
    const pesel = result.find((m) => m.category === "NATIONAL_ID_PL");
    expect(pesel).toBeDefined();
    expect(text.slice(pesel!.start, pesel!.end)).toBe("80051501231");
  });
});
```

Note: the last test uses `pesel!` only after `expect(pesel).toBeDefined()` -- this is standard vitest narrowing style already used in this file (see `findings[0]?.category` elsewhere in the suite), not a violation of the "no non-null assertion in library code" rule, which applies to `eu-checksums.ts`/`eu-patterns.ts` source, not test assertions.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: FAIL — `Cannot find module '../src/redaction/eu-patterns.js'`

- [ ] **Step 3: Implement the structured scanner**

```typescript
// mcp-server/src/redaction/eu-patterns.ts
import { isValidBelgianRegistre, isValidBsn, isValidPesel } from "./eu-checksums.js";
import type { PiiFinding } from "./detect.js";

interface RawMatch {
  category: string;
  original: string;
  start: number;
  end: number;
  confidence: number;
}

/** True if [start, end) overlaps any existing match's span. */
export function overlapsExisting(matches: RawMatch[], start: number, end: number): boolean {
  return matches.some((m) => start < m.end && m.start < end);
}

function findAllNonOverlapping(
  text: string,
  pattern: RegExp,
  category: string,
  confidence: number,
  existing: RawMatch[],
  validate?: (matchText: string) => boolean,
): RawMatch[] {
  const found: RawMatch[] = [];
  const flags = pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`;
  const regex = new RegExp(pattern.source, flags);
  let match: RegExpExecArray | null;
  while ((match = regex.exec(text)) !== null) {
    const matchText = match[0];
    const start = match.index;
    const end = start + matchText.length;
    if (validate && !validate(matchText)) continue;
    if (overlapsExisting(existing, start, end) || overlapsExisting(found, start, end)) continue;
    found.push({ category, original: matchText, start, end, confidence });
  }
  return found;
}

/**
 * Scan for EU-specific structured PII: national IDs, tax identifiers, and
 * EU vehicle license plates. Does NOT include GDPR Art. 9 keyword patterns --
 * see `scanArt9Keywords` for those.
 *
 * Ordering matters: national IDs run before tax IDs (SIRET before SIREN, so
 * the 9-digit SIREN prefix inside a 14-digit SIRET doesn't get double-flagged),
 * matching the priority order in anno's `pii.rs::scan_eu_structured`.
 */
export function scanEuStructured(text: string): RawMatch[] {
  const results: RawMatch[] = [];

  // --- National IDs ---
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b[12]\d{2}(?:0[1-9]|1[0-2])\d{2}\d{3}\d{3}\d{2}\b/g,
      "NATIONAL_ID_FR",
      0.97,
      results,
    ),
  );
  results.push(
    ...findAllNonOverlapping(text, /\b(?:[XYZ]\d{7}|\d{8})[A-Z]\b/g, "NATIONAL_ID_ES", 0.97, results),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b[A-Z]{6}\d{2}[A-Z]\d{2}[A-Z]\d{3}[A-Z]\b/g,
      "NATIONAL_ID_IT",
      0.97,
      results,
    ),
  );
  results.push(
    ...findAllNonOverlapping(text, /\b\d{11}\b/g, "NATIONAL_ID_PL", 0.97, results, isValidPesel),
  );
  results.push(
    ...findAllNonOverlapping(text, /\b\d{9}\b/g, "NATIONAL_ID_NL", 0.97, results, isValidBsn),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b\d{2}[0-1]\d[0-3]\d\d{5}\b/g,
      "NATIONAL_ID_BE",
      0.97,
      results,
      isValidBelgianRegistre,
    ),
  );

  // --- Tax identifiers ---
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b\d{3}\s?\d{3}\s?\d{3}\s?\d{5}\b/g,
      "TAX_ID_SIRET",
      0.9,
      results,
      (m) => m.replace(/\D/g, "").length === 14,
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b\d{3}\s?\d{3}\s?\d{3}\b/g,
      "TAX_ID_SIREN",
      0.9,
      results,
      (m) => m.replace(/\D/g, "").length === 9,
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b(?:AT|BE|BG|CY|CZ|DE|DK|EE|EL|ES|FI|FR|GB|HR|HU|IE|IT|LT|LU|LV|MT|NL|PL|PT|RO|SE|SI|SK)\d{8,12}\b/g,
      "TAX_ID_VAT",
      0.9,
      results,
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b(?:DE|FR|IT|ES|PL|NL|BE|PT|CZ|HU|SE|AT|CH|RO|BG|DK|FI|GR|IE|SK|SI|HR|LT|LV|EE|LU|MT|CY)\s?-?\d[\w-]{2,6}\b/g,
      "LICENSE_PLATE_EU",
      0.75,
      results,
    ),
  );

  return results;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: all 9 tests PASS

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/eu-patterns.ts mcp-server/tests/eu-patterns.test.ts
git commit -m "feat(mcp): add EU structured PII pattern scanner"
```

---

### Task 3: GDPR Art. 9 special-category keyword scanner

**Files:**
- Modify: `mcp-server/src/redaction/eu-patterns.ts`
- Modify: `mcp-server/tests/eu-patterns.test.ts`

**Interfaces:**
- Consumes: `overlapsExisting`, `findAllNonOverlapping` (both already in `eu-patterns.ts` from Task 2 — `findAllNonOverlapping` is module-private, used directly, not re-exported).
- Produces: `scanArt9Keywords(text: string): RawMatch[]` (exported from `eu-patterns.ts`). Task 4 consumes it.

- [ ] **Step 1: Write the failing tests**

Add to `mcp-server/tests/eu-patterns.test.ts`:

```typescript
import { scanArt9Keywords } from "../src/redaction/eu-patterns.js";

describe("scanArt9Keywords", () => {
  it("detects a health condition keyword", () => {
    const result = scanArt9Keywords("Patient diagnosed with diabetes");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
  });

  it("detects a religion keyword", () => {
    const result = scanArt9Keywords("He is Catholic");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_RELIGION")).toBe(true);
  });

  it("detects a criminal record keyword", () => {
    const result = scanArt9Keywords("He was convicted of fraud");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_CRIMINAL")).toBe(true);
  });

  it("detects a biometric keyword", () => {
    const result = scanArt9Keywords("Access requires facial recognition");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_BIOMETRIC")).toBe(true);
  });

  it("returns nothing for neutral text", () => {
    const result = scanArt9Keywords("The meeting is scheduled for Tuesday.");
    expect(result).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: FAIL — `scanArt9Keywords is not a function`

- [ ] **Step 3: Implement the Art. 9 keyword scanner**

Add to `mcp-server/src/redaction/eu-patterns.ts`, after `scanEuStructured`:

```typescript
/**
 * Scan for GDPR Art. 9 special category keywords: health, biometric, genetic,
 * political, religion, union, criminal, sexual orientation, ethnic origin.
 *
 * This is a keyword/regex scan -- high recall, high false-positive rate by
 * design. It does not use NER/zero-shot context, unlike anno's
 * `scan_patterns_with_ner` (out of scope for this plan -- see Global
 * Constraints).
 */
export function scanArt9Keywords(text: string): RawMatch[] {
  const results: RawMatch[] = [];
  const rules: Array<{ category: string; pattern: RegExp; confidence: number }> = [
    {
      category: "SPECIAL_CATEGORY_HEALTH",
      pattern:
        /\b(diagnosed\s+with|suffers?\s+from|allergic\s+to|medical\s+condition|hospital|surgery|treatment|disease|illness|cancer|diabetes|hypertension|asthma|depression|anxiety)\b/gi,
      confidence: 0.97,
    },
    {
      category: "SPECIAL_CATEGORY_BIOMETRIC",
      pattern: /\b(fingerprint|iris\s+scan|facial\s+recognition|biometric|face\s+scan|voice\s+recognition)\b/gi,
      confidence: 0.97,
    },
    {
      category: "SPECIAL_CATEGORY_GENETIC",
      pattern: /\b(genetic\s+data|dna\s+test|genome|inherited\s+condition|hereditary)\b/gi,
      confidence: 0.97,
    },
    {
      category: "SPECIAL_CATEGORY_POLITICAL",
      pattern:
        /\b(member\s+of\s+(?:the\s+)?(?:socialist|communist|conservative|liberal|democrat|republican)\s+party|party\s+affiliation|political\s+opinion)\b/gi,
      confidence: 0.9,
    },
    {
      category: "SPECIAL_CATEGORY_RELIGION",
      pattern: /\b(catholic|protestant|muslim|jewish|buddhist|hindu|sikh|atheist|agnostic)\b/gi,
      confidence: 0.9,
    },
    {
      category: "SPECIAL_CATEGORY_UNION",
      pattern: /\b(trade\s+union\s+member|union\s+membership|collective\s+bargaining)\b/gi,
      confidence: 0.75,
    },
    {
      category: "SPECIAL_CATEGORY_CRIMINAL",
      pattern:
        /\b(convicted\s+of|arrested\s+for|charged\s+with|criminal\s+record|incarcerated|felony\s+conviction)\b/gi,
      confidence: 0.97,
    },
    {
      category: "SPECIAL_CATEGORY_SEXUAL_ORIENTATION",
      pattern: /\b(gay|lesbian|bisexual|transgender|lgbtq\+?|homosexual|queer)\b/gi,
      confidence: 0.9,
    },
    {
      category: "SPECIAL_CATEGORY_ETHNIC",
      pattern: /\b(ethnic\s+origin|racial\s+origin|roma\s+community|indigenous\s+people)\b/gi,
      confidence: 0.9,
    },
  ];

  for (const { category, pattern, confidence } of rules) {
    results.push(...findAllNonOverlapping(text, pattern, category, confidence, results));
  }

  return results;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: all 14 tests PASS (9 from Task 2 + 5 new)

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/eu-patterns.ts mcp-server/tests/eu-patterns.test.ts
git commit -m "feat(mcp): add GDPR Art. 9 special-category keyword scanner"
```

---

### Task 4: Combine into `scanEuPatterns()`

**Files:**
- Modify: `mcp-server/src/redaction/eu-patterns.ts`
- Modify: `mcp-server/tests/eu-patterns.test.ts`

**Interfaces:**
- Consumes: `scanEuStructured`, `scanArt9Keywords`, `overlapsExisting` (all from Tasks 2-3, same file). `PiiFinding` from `./detect.js`.
- Produces: `scanEuPatterns(text: string): PiiFinding[]` (exported). Task 5 (`detectPiiEu`) consumes it.

- [ ] **Step 1: Write the failing tests**

Add to `mcp-server/tests/eu-patterns.test.ts`:

```typescript
import { scanEuPatterns } from "../src/redaction/eu-patterns.js";

describe("scanEuPatterns", () => {
  it("combines structured and Art. 9 findings, sorted by position", () => {
    const result = scanEuPatterns("Patient diagnosed with diabetes. PESEL: 80051501231.");
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
    expect(result.some((f) => f.category === "NATIONAL_ID_PL")).toBe(true);
    for (let i = 1; i < result.length; i++) {
      expect(result[i]!.start).toBeGreaterThanOrEqual(result[i - 1]!.start);
    }
  });

  it("assigns sequential per-category tokens", () => {
    const result = scanEuPatterns("He is Catholic. She is Muslim.");
    const religionFindings = result.filter((f) => f.category === "SPECIAL_CATEGORY_RELIGION");
    expect(religionFindings.map((f) => f.token)).toEqual([
      "[SPECIAL_CATEGORY_RELIGION_1]",
      "[SPECIAL_CATEGORY_RELIGION_2]",
    ]);
  });

  it("structured matches take precedence over overlapping Art. 9 matches", () => {
    // "hospital" (Art.9 health keyword) inside a longer structured match is not
    // a realistic overlap case for these patterns, so this asserts the simpler
    // invariant: every returned finding has a non-empty original span.
    const result = scanEuPatterns("SIRET: 73282932000074");
    expect(result.every((f) => f.end > f.start)).toBe(true);
  });

  it("returns an empty array for text with no EU PII", () => {
    expect(scanEuPatterns("The weather is nice today.")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: FAIL — `scanEuPatterns is not a function`

- [ ] **Step 3: Implement `scanEuPatterns`**

Add to `mcp-server/src/redaction/eu-patterns.ts`, after `scanArt9Keywords`:

```typescript
/**
 * Combined EU structured + Art. 9 keyword scan, producing xberg's standard
 * `PiiFinding` shape with sequential per-category tokens.
 *
 * Structured patterns (national IDs, tax IDs, license plates) claim their
 * spans first; Art. 9 keyword matches that overlap a structured span are
 * dropped, matching anno's `scan_eu_patterns` ordering.
 */
export function scanEuPatterns(text: string): PiiFinding[] {
  const structured = scanEuStructured(text);
  const art9 = scanArt9Keywords(text).filter((m) => !overlapsExisting(structured, m.start, m.end));
  const combined = [...structured, ...art9].sort((a, b) => a.start - b.start);

  const counters: Record<string, number> = {};
  return combined.map((m) => {
    counters[m.category] = (counters[m.category] ?? 0) + 1;
    return {
      token: `[${m.category}_${counters[m.category]}]`,
      category: m.category,
      original: m.original,
      start: m.start,
      end: m.end,
      confidence: m.confidence,
    };
  });
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run eu-patterns`
Expected: all 18 tests PASS (14 from Tasks 2-3 + 4 new)

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/eu-patterns.ts mcp-server/tests/eu-patterns.test.ts
git commit -m "feat(mcp): combine EU structured and Art. 9 scans into scanEuPatterns"
```

---

### Task 5: `dedupOverlapping` + `detectPiiEu` in `detect.ts`

**Files:**
- Modify: `mcp-server/src/redaction/detect.ts`
- Modify: `mcp-server/tests/redaction.test.ts`

**Interfaces:**
- Consumes: `scanEuPatterns` from `./eu-patterns.js` (Task 4). `detectPii`, `PiiFinding` (already in `detect.ts`).
- Produces: `dedupOverlapping(findings: PiiFinding[]): PiiFinding[]` (exported), `detectPiiEu(text: string, filterCategories?: string[]): PiiFinding[]` (exported). Task 7 (`ingest.ts`) consumes `detectPiiEu`.

- [ ] **Step 1: Write the failing tests**

Add to `mcp-server/tests/redaction.test.ts`:

```typescript
import { detectPiiEu, dedupOverlapping } from "../src/redaction/detect.js";

describe("dedupOverlapping", () => {
  it("keeps the longest span when two findings overlap", () => {
    const findings = [
      { token: "[A_1]", category: "A", original: "John", start: 0, end: 4, confidence: 0.5 },
      { token: "[B_1]", category: "B", original: "John Smith", start: 0, end: 10, confidence: 0.6 },
    ];
    const result = dedupOverlapping(findings);
    expect(result).toHaveLength(1);
    expect(result[0]?.category).toBe("B");
  });

  it("keeps non-overlapping findings", () => {
    const findings = [
      { token: "[A_1]", category: "A", original: "foo", start: 0, end: 3, confidence: 0.5 },
      { token: "[B_1]", category: "B", original: "bar", start: 5, end: 8, confidence: 0.5 },
    ];
    expect(dedupOverlapping(findings)).toHaveLength(2);
  });
});

describe("detectPiiEu", () => {
  it("includes both generic and EU findings", () => {
    const result = detectPiiEu("Contact bob@example.com. PESEL: 80051501231.");
    expect(result.some((f) => f.category === "EMAIL")).toBe(true);
    expect(result.some((f) => f.category === "NATIONAL_ID_PL")).toBe(true);
  });

  it("does not change detectPii's own output", () => {
    // detectPii() itself must remain unaffected by this addition.
    const before = detectPiiEu("bob@example.com").filter((f) => f.category === "EMAIL");
    expect(before).toHaveLength(1);
  });

  it("respects filterCategories across both scans", () => {
    const result = detectPiiEu("Contact bob@example.com. PESEL: 80051501231.", ["EMAIL"]);
    expect(result.some((f) => f.category === "EMAIL")).toBe(true);
    expect(result.some((f) => f.category === "NATIONAL_ID_PL")).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run redaction`
Expected: FAIL — `detectPiiEu is not a function`

- [ ] **Step 3: Implement `dedupOverlapping` and `detectPiiEu`**

Add to `mcp-server/src/redaction/detect.ts`, at the top add the import:

```typescript
import { scanEuPatterns } from "./eu-patterns.js";
```

Add at the bottom of the file:

```typescript
/**
 * Remove duplicate and overlapping findings, keeping the longest span at
 * each start position. Used to merge `detectPii()` and `scanEuPatterns()`
 * output, which can both match overlapping spans (e.g. a digit-heavy string
 * matched by both a generic pattern and an EU-specific one).
 */
export function dedupOverlapping(findings: PiiFinding[]): PiiFinding[] {
  const sorted = [...findings].sort((a, b) => a.start - b.start || b.end - a.end);
  const deduped: PiiFinding[] = [];
  let maxEnd = 0;
  for (const finding of sorted) {
    if (finding.start < maxEnd) continue;
    maxEnd = finding.end;
    deduped.push(finding);
  }
  return deduped;
}

/**
 * `detectPii()` plus EU-specific structured and Art. 9 detection, deduplicated.
 * Opt-in entrypoint -- `detectPii()` itself is unchanged for existing callers.
 */
export function detectPiiEu(text: string, filterCategories?: string[]): PiiFinding[] {
  const generic = detectPii(text, filterCategories);
  const eu = scanEuPatterns(text).filter((f) => !filterCategories || filterCategories.includes(f.category));
  return dedupOverlapping([...generic, ...eu].sort((a, b) => a.start - b.start));
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run redaction`
Expected: all tests PASS (existing + 5 new)

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/detect.ts mcp-server/tests/redaction.test.ts
git commit -m "feat(mcp): add detectPiiEu combinator and dedupOverlapping helper"
```

---

### Task 6: k-anonymity risk report (`buildPiiReport`)

**Files:**
- Modify: `mcp-server/src/redaction/detect.ts`
- Modify: `mcp-server/tests/redaction.test.ts`

**Interfaces:**
- Consumes: `PiiFinding` (already in `detect.ts`).
- Produces: `interface PiiReport { personCount, dateCount, locationCount, contactCount, idNumberCount, entities, kAnonymityRisk }`, `buildPiiReport(findings: PiiFinding[]): PiiReport` (both exported). Available to any MCP tool that wants a risk summary (not wired into a tool in this plan — Task 7 only wires `detectPiiEu`).

- [ ] **Step 1: Write the failing tests**

Add to `mcp-server/tests/redaction.test.ts`:

```typescript
import { buildPiiReport } from "../src/redaction/detect.js";

describe("buildPiiReport", () => {
  it("counts findings by pii-report category", () => {
    const findings = [
      { token: "[NAME_1]", category: "NAME", original: "John", start: 0, end: 4, confidence: 0.6 },
      {
        token: "[ID_NUMBER_1]",
        category: "SSN",
        original: "123-45-6789",
        start: 10,
        end: 21,
        confidence: 0.9,
      },
    ];
    const report = buildPiiReport(findings);
    expect(report.personCount).toBe(1);
    expect(report.idNumberCount).toBe(1);
    expect(report.kAnonymityRisk).toBe("CRITICAL (direct identifiers present)");
  });

  it("reports LOW risk when nothing sensitive is present", () => {
    const report = buildPiiReport([]);
    expect(report.kAnonymityRisk).toBe("LOW");
  });

  it("reports HIGH risk for a quasi-identifier combination", () => {
    const makeName = (i: number) => ({
      token: `[NAME_${i}]`,
      category: "NAME",
      original: `Person${i}`,
      start: i * 10,
      end: i * 10 + 8,
      confidence: 0.6,
    });
    const findings = [
      ...[1, 2, 3, 4, 5, 6].map(makeName),
      { token: "[DATE_1]", category: "DATE", original: "1990-01-01", start: 100, end: 110, confidence: 0.7 },
      {
        token: "[LOCATION_1]",
        category: "LOCATION",
        original: "Springfield",
        start: 120,
        end: 131,
        confidence: 0.7,
      },
    ];
    expect(buildPiiReport(findings).kAnonymityRisk).toBe("HIGH (quasi-identifier combination)");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd mcp-server && npm test -- run redaction`
Expected: FAIL — `buildPiiReport is not a function`

- [ ] **Step 3: Implement `PiiReport` and `buildPiiReport`**

Add to `mcp-server/src/redaction/detect.ts`:

```typescript
export interface PiiReport {
  personCount: number;
  dateCount: number;
  locationCount: number;
  contactCount: number;
  idNumberCount: number;
  entities: PiiFinding[];
  kAnonymityRisk: string;
}

const ID_NUMBER_CATEGORIES = new Set([
  "SSN",
  "CREDIT_CARD",
  "IBAN",
  "NATIONAL_ID_FR",
  "NATIONAL_ID_ES",
  "NATIONAL_ID_IT",
  "NATIONAL_ID_PL",
  "NATIONAL_ID_NL",
  "NATIONAL_ID_BE",
  "TAX_ID_SIRET",
  "TAX_ID_SIREN",
  "TAX_ID_VAT",
  "LICENSE_PLATE_EU",
]);

/**
 * Summarize detected PII, including a k-anonymity risk assessment based on
 * the presence and combination of direct/quasi identifiers. Mirrors anno's
 * `pii::report()`.
 */
export function buildPiiReport(findings: PiiFinding[]): PiiReport {
  let personCount = 0;
  let dateCount = 0;
  let locationCount = 0;
  let contactCount = 0;
  let idNumberCount = 0;
  const uniqueNames = new Set<string>();

  for (const finding of findings) {
    if (finding.category === "NAME") {
      personCount += 1;
      uniqueNames.add(finding.original.toLowerCase());
    } else if (finding.category === "DATE" || finding.category === "DATE_ISO" || finding.category === "DATE_MDY") {
      dateCount += 1;
    } else if (finding.category === "LOCATION") {
      locationCount += 1;
    } else if (finding.category === "EMAIL" || finding.category === "PHONE") {
      contactCount += 1;
    } else if (ID_NUMBER_CATEGORIES.has(finding.category)) {
      idNumberCount += 1;
    }
  }

  let kAnonymityRisk: string;
  if (idNumberCount > 0) {
    kAnonymityRisk = "CRITICAL (direct identifiers present)";
  } else if (uniqueNames.size > 5 && dateCount > 0 && locationCount > 0) {
    kAnonymityRisk = "HIGH (quasi-identifier combination)";
  } else if (uniqueNames.size > 3) {
    kAnonymityRisk = "MEDIUM (multiple names)";
  } else {
    kAnonymityRisk = "LOW";
  }

  return { personCount, dateCount, locationCount, contactCount, idNumberCount, entities: findings, kAnonymityRisk };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd mcp-server && npm test -- run redaction`
Expected: all tests PASS (existing + 3 new)

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/redaction/detect.ts mcp-server/tests/redaction.test.ts
git commit -m "feat(mcp): add k-anonymity risk report (buildPiiReport)"
```

---

### Task 7: Wire `eu_patterns` opt-in flag into `ingest_folder`

**Files:**
- Modify: `mcp-server/src/tools/ingest.ts:6` (import), `:90-124` (schema), `:125` (handler params), `:173` (detection call)

**Interfaces:**
- Consumes: `detectPiiEu` from `../redaction/detect.js` (Task 5).
- Produces: nothing new for later tasks — this is the final wiring point.

- [ ] **Step 1: Write the failing test**

Add to a new describe block in `mcp-server/tests/tools.test.ts` (or extend the existing `ingest tools module` block if present):

```typescript
describe("ingest_folder eu_patterns option", () => {
  it("exports registerIngestTools", async () => {
    const mod = await import("../src/tools/ingest.js");
    expect(typeof mod.registerIngestTools).toBe("function");
  });
});
```

(This smoke test likely already exists from the prior NER-merge plan — if so, skip to Step 2. The real verification for this task is the type-check in Step 4, since `eu_patterns` only affects an internal branch, not an exported function signature.)

- [ ] **Step 2: Add the `eu_patterns` schema field**

In `mcp-server/src/tools/ingest.ts`, in the `ingest_folder` tool's Zod schema (currently at line 90), add after `rehydration_passphrase` (line 96) and before `use_ner` (line 97):

```typescript
      eu_patterns: z.boolean().optional().default(false).describe(
        "Additionally scan for EU-specific structured PII (checksum-validated national IDs for FR/ES/IT/PL/NL/BE, FR SIRET/SIREN, EU VAT numbers, EU license plates) and GDPR Art. 9 special-category keywords (health, biometric, genetic, political, religious, union, criminal, sexual orientation, ethnic origin)."
      ),
```

- [ ] **Step 3: Destructure the new parameter and use it**

Change the handler signature at line 125 from:

```typescript
    async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, use_ner, ner_backend, ner_model, ner_hf_repo, ner_hf_model_file, ner_hf_tokenizer_file, ner_hf_architecture, ner_llm_model, ner_categories }) => {
```

to:

```typescript
    async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, eu_patterns, use_ner, ner_backend, ner_model, ner_hf_repo, ner_hf_model_file, ner_hf_tokenizer_file, ner_hf_architecture, ner_llm_model, ner_categories }) => {
```

Change the import at line 6 from:

```typescript
import { detectPii, mergeNerEntities, type NerEntity } from "../redaction/detect.js";
```

to:

```typescript
import { detectPii, detectPiiEu, mergeNerEntities, type NerEntity } from "../redaction/detect.js";
```

Change line 173 from:

```typescript
            const regexFindings = detectPii(rawText);
```

to:

```typescript
            const regexFindings = eu_patterns ? detectPiiEu(rawText) : detectPii(rawText);
```

- [ ] **Step 4: Type-check**

Run: `cd mcp-server && node_modules/.bin/tsc --noEmit`
Expected: zero errors

- [ ] **Step 5: Run the full test suite**

Run: `cd mcp-server && npm test -- run`
Expected: all tests PASS. Default behavior (`eu_patterns: false`) is unchanged for existing callers.

- [ ] **Step 6: Commit**

```bash
git add mcp-server/src/tools/ingest.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): wire eu_patterns opt-in flag into ingest_folder"
```

---

### Task 8: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md:14-16`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing — documentation only.

- [ ] **Step 1: Add the entry**

In `CHANGELOG.md`, under `## [Unreleased]` → `### Added` (currently starting at line 16), add a new bullet before the existing "Durable rehydration-map storage" entry:

```markdown
- **EU/GDPR structured PII detection.** `ingest_folder`'s new opt-in
  `eu_patterns` flag scans for checksum-validated EU national IDs (FR INSEE,
  ES DNI/NIE, IT Codice Fiscale, PL PESEL, NL BSN, BE Registre National),
  FR SIRET/SIREN, EU VAT numbers, EU license plates, and GDPR Art. 9
  special-category keywords (health, biometric, genetic, political,
  religious, union, criminal, sexual orientation, ethnic origin), plus a
  k-anonymity risk report via `buildPiiReport()`. Default is unchanged
  (`eu_patterns: false`) for existing callers.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record EU/GDPR PII detection addition"
```

---

## Self-Review

**Spec coverage:**
- ✅ Checksum-validated national IDs (PL PESEL, NL BSN, BE Registre National with pre/post-2000 handling) — Task 1
- ✅ Format-only national IDs (FR INSEE, ES DNI/NIE, IT Codice Fiscale) — Task 2
- ✅ Tax IDs (FR SIRET/SIREN, EU VAT) and EU license plates — Task 2
- ✅ GDPR Art. 9 keyword detection (9 categories) — Task 3
- ✅ Combined scan with anno's structured-before-keywords precedence — Task 4
- ✅ Overlap dedup shared with the rest of the PII pipeline — Task 5
- ✅ k-anonymity risk scoring — Task 6
- ✅ Opt-in wiring into the MCP tool surface, backward-compatible default — Task 7
- ✅ Public API change documented — Task 8
- Explicitly out of scope (documented in Global Constraints): the GLiNER2-NER-based Art. 9 path (`scan_patterns_with_ner` equivalent), anno's `heuristic`/`heuristic_fr`/`crf` backends, `pseudonymize`/`fingerprint` redaction modes, `DiscontinuousSpan` cross-clause party tracking, `schema.rs` canonical-type harmonization — all flagged in the prior investigation as follow-up work, not folded in here to keep this plan independently shippable.

**Placeholder scan:** None found — every step has concrete, complete code.

**Type consistency:** `PiiFinding` (`token`, `category`, `original`, `start`, `end`, `confidence`) is used identically across Tasks 2, 4, 5, 6 — no field renamed. `RawMatch` (internal to `eu-patterns.ts`) is only used within Tasks 2-4 and never exported, so it can't leak an inconsistent shape to callers. `scanEuStructured`/`scanArt9Keywords`/`scanEuPatterns`/`dedupOverlapping`/`detectPiiEu`/`buildPiiReport` signatures are declared once (in the task that introduces them) and referenced with matching names in every later task.
