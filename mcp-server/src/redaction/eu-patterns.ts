import {
  isValidBelgianRegistre,
  isValidBsn,
  isValidEsDni,
  isValidFrInsee,
  isValidItCodiceFiscale,
  isValidLuhn,
  isValidPesel,
} from "./eu-checksums.js";
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
      isValidFrInsee,
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b(?:[XYZ]\d{7}|\d{8})[A-Z]\b/g,
      "NATIONAL_ID_ES",
      0.97,
      results,
      isValidEsDni,
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b[A-Z]{6}\d{2}[A-Z]\d{2}[A-Z]\d{3}[A-Z]\b/g,
      "NATIONAL_ID_IT",
      0.97,
      results,
      isValidItCodiceFiscale,
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
      (m) => isValidLuhn(m.replace(/\D/g, "")),
    ),
  );
  results.push(
    ...findAllNonOverlapping(
      text,
      /\b\d{3}\s?\d{3}\s?\d{3}\b/g,
      "TAX_ID_SIREN",
      0.9,
      results,
      (m) => isValidLuhn(m.replace(/\D/g, "")),
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
      /\b(?:DE|FR|IT|ES|PL|NL|BE|PT|CZ|HU|SE|AT|CH|RO|BG|DK|FI|GR|IE|SK|SI|HR|LT|LV|EE|LU|MT|CY)[\s-][A-Z\d-]{3,7}\b/g,
      "LICENSE_PLATE_EU",
      0.75,
      results,
      // Require a mandatory separator (already enforced by the regex) plus a
      // mix of letters and digits in the plate block -- rejects bare years
      // ("AT2024"), reference numbers, and other alphanumeric false positives
      // that a country-code prefix alone would over-match.
      (m) => {
        const block = m.slice(3);
        return /\d/.test(block) && /[A-Z]/.test(block);
      },
    ),
  );

  return results;
}

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
