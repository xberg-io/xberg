import { scanEuPatterns } from "./eu-patterns.js";

export interface PiiFinding {
  token: string;
  category: string;
  original: string;
  start: number;
  end: number;
  confidence: number;
}

export interface PiiReport {
  personCount: number;
  dateCount: number;
  locationCount: number;
  contactCount: number;
  idNumberCount: number;
  specialCategoryCount: number;
  entities: PiiFinding[];
  kAnonymityRisk: string;
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

/** Shape of Entity objects returned by xberg's NER pipeline. */
export interface NerEntity {
  /** EntityCategory enum value, e.g. 'person', 'organization', 'location'. */
  category: string;
  text: string;
  start: number;
  end: number;
  confidence?: number;
}

const NER_CATEGORY_TO_PII: Record<string, string> = {
  person: "NAME",
  organization: "ORG",
  location: "LOCATION",
  email: "EMAIL",
  phone: "PHONE",
  date: "DATE",
  money: "MONEY",
  url: "URL",
};

function spansOverlap(a: PiiFinding, b: { start: number; end: number }): boolean {
  return a.start < b.end && b.start < a.end;
}

export function mergeNerEntities(regex: PiiFinding[], entities: NerEntity[], _text: string): PiiFinding[] {
  const findings = [...regex];
  const counters: Record<string, number> = {};
  for (const f of findings) {
    counters[f.category] = Math.max(counters[f.category] ?? 0, Number(f.token.match(/_(\d+)\]$/)?.[1] ?? 0));
  }

  for (const entity of entities) {
    const category = NER_CATEGORY_TO_PII[entity.category.toLowerCase()] ?? `NER_${entity.category.toUpperCase()}`;
    const { text: entityText, start, end } = entity;
    const entityConfidence = entity.confidence ?? 0.8;

    const overlap = findings.find((f) => spansOverlap(f, { start, end }));
    if (overlap) {
      if (entityConfidence > overlap.confidence) {
        overlap.category = category;
        overlap.confidence = entityConfidence;
        overlap.original = entityText;
        overlap.start = start;
        overlap.end = end;
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

/**
 * Route a document through `detectPiiEu` or `detectPii` based on the caller's
 * `eu_patterns` opt-in flag. Used by `ingest_folder` -- pulled out as its own
 * function so the routing decision is unit-testable without the native
 * extraction/embedding bindings `ingest.ts` otherwise pulls in.
 */
export function selectPiiScan(rawText: string, euPatterns: boolean, filterCategories?: string[]): PiiFinding[] {
  return euPatterns ? detectPiiEu(rawText, filterCategories) : detectPii(rawText, filterCategories);
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

/** GDPR Art. 9 special-category keyword categories emitted by `scanArt9Keywords`. */
const SPECIAL_CATEGORY_PREFIX = "SPECIAL_CATEGORY_";

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
  let specialCategoryCount = 0;
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
    } else if (finding.category.startsWith(SPECIAL_CATEGORY_PREFIX)) {
      specialCategoryCount += 1;
    }
  }

  let kAnonymityRisk: string;
  if (idNumberCount > 0 || specialCategoryCount > 0) {
    kAnonymityRisk = "CRITICAL (direct identifiers or special-category data present)";
  } else if (uniqueNames.size > 5 && dateCount > 0 && locationCount > 0) {
    kAnonymityRisk = "HIGH (quasi-identifier combination)";
  } else if (uniqueNames.size > 3) {
    kAnonymityRisk = "MEDIUM (multiple names)";
  } else {
    kAnonymityRisk = "LOW";
  }

  return {
    personCount,
    dateCount,
    locationCount,
    contactCount,
    idNumberCount,
    specialCategoryCount,
    entities: findings,
    kAnonymityRisk,
  };
}
