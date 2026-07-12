import { describe, it, expect, beforeEach } from "vitest";
import "fake-indexeddb/auto";
import { putHistoryEntry, listHistory, getHistoryEntry, listFolders } from "../src/lib/ingest-history.js";
import type { IngestHistoryEntry } from "../src/lib/types.js";

function entry(overrides: Partial<IngestHistoryEntry> = {}): IngestHistoryEntry {
  return {
    collection: "c1",
    externalId: "doc-1.pdf",
    filename: "doc-1.pdf",
    mime: "application/pdf",
    redactedText: "Hello [EMAIL_1]",
    piiCategoryCounts: { EMAIL: 1 },
    documentId: "doc-1",
    status: "synced",
    ingestedAt: 1000,
    ...overrides,
  };
}

describe("lib/ingest-history", () => {
  beforeEach(async () => {
    indexedDB.deleteDatabase("xberg-web-ui");
  });

  it("round-trips a single entry", async () => {
    await putHistoryEntry(entry());
    const found = await getHistoryEntry("c1", "doc-1.pdf");
    expect(found).toEqual(entry());
  });

  it("returns null for a missing entry", async () => {
    expect(await getHistoryEntry("c1", "missing.pdf")).toBeNull();
  });

  it("listHistory filters by collection", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "b.pdf" }));
    const c1Only = await listHistory("c1");
    expect(c1Only.map((e) => e.externalId)).toEqual(["a.pdf"]);
  });

  it("listHistory with no filter returns everything", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "b.pdf" }));
    expect((await listHistory()).length).toBe(2);
  });

  it("putHistoryEntry upserts by (collection, externalId)", async () => {
    await putHistoryEntry(entry({ status: "pending" }));
    await putHistoryEntry(entry({ status: "synced" }));
    const all = await listHistory("c1");
    expect(all.length).toBe(1);
    expect(all[0]?.status).toBe("synced");
  });

  it("listFolders returns distinct collection names", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c1", externalId: "b.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "c.pdf" }));
    expect(await listFolders()).toEqual(["c1", "c2"]);
  });
});
