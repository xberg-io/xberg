"use client";

import type { CSSProperties, ReactElement } from "react";

const CATEGORY_COLOR: Record<string, string> = {
  EMAIL: "bg-blue-200 text-gray-900",
  PERSON: "bg-purple-200 text-gray-900",
  PHONE: "bg-rose-200 text-gray-900",
  SSN: "bg-emerald-200 text-gray-900",
  IBAN: "bg-amber-200 text-gray-900",
  CREDIT_CARD: "bg-red-200 text-gray-900",
  IP: "bg-cyan-200 text-gray-900",
};

const FALLBACK_COLOR = "bg-gray-200 text-gray-900";

function colorFor(token: string): string {
  const match = token.match(/\[([A-Z_]+)_\d+\]/);
  const category = match ? (match[1] ?? "") : "";
  return CATEGORY_COLOR[category] ?? FALLBACK_COLOR;
}

export interface BoundingBoxCitationsProps {
  redactedText: string;
  /** Clear PII values keyed by token. MUST NEVER be rendered (PII safety). */
  map?: Record<string, string>;
  counts: Record<string, number>;
  file?: string;
}

export function BoundingBoxCitations({
  redactedText,
  counts,
  file,
}: BoundingBoxCitationsProps): ReactElement {
  const parts = redactedText.split(/(\[[A-Z_]+_\d+\])/g);

  const countRows = Object.entries(counts).map(([category, count]) => (
    <li
      key={category}
      data-testid="pii-count"
      className={`rounded px-2 py-1 ${CATEGORY_COLOR[category] ?? FALLBACK_COLOR}`}
      style={{ marginBottom: "0.25rem" } as CSSProperties}
    >
      <span data-testid="pii-count-category">{category}</span>{" "}
      <span data-testid="pii-count-value">{count}</span>
    </li>
  ));

  return (
    <div className="flex flex-col gap-4">
      {file ? (
        <p className="text-sm text-gray-500" data-testid="pii-file">
          {file}
        </p>
      ) : null}
      <div
        data-testid="pii-redacted"
        className="whitespace-pre-wrap rounded border border-gray-200 p-3 text-gray-900"
      >
        {parts.map((part, index) =>
          /^\[[A-Z_]+_\d+\]$/.test(part) ? (
            <mark key={index} data-testid="pii-token" className={colorFor(part)}>
              {part}
            </mark>
          ) : (
            <span key={index}>{part}</span>
          ),
        )}
      </div>
      <aside data-testid="pii-counts" className="rounded border border-gray-200 p-3">
        <h3 className="mb-2 text-sm font-semibold text-gray-900">PII counts</h3>
        <ul className="list-none p-0">{countRows}</ul>
      </aside>
    </div>
  );
}

export default BoundingBoxCitations;
