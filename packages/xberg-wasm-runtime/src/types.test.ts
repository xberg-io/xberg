import { describe, it, expect } from "vitest";
import { injectionDescriptorSchema } from "./validation";

describe("injectionDescriptor validation", () => {
  it("accepts valid embedder + store + optional ner/ocr", () => {
    const descriptor = {
      embedder: {
        embed: async (texts: string[]) => {
          return texts.map(() => new Float32Array([0.1, 0.2]));
        },
      },
      store: {
        ensureCollection: async () => {},
        dropCollection: async () => {},
        getCollection: async () => null,
        upsertDocument: async () => "doc-1",
        deleteDocuments: async () => 0,
        deleteByFilter: async () => 0,
        retrieve: async () => ({ mode: "vector", chunks: [] }),
        collectionStats: async () => ({ documents: 0, chunks: 0 }),
      },
    };
    const result = injectionDescriptorSchema.safeParse(descriptor);
    expect(result.success).toBe(true);
  });

  it("rejects missing embedder", () => {
    const descriptor = {
      store: {
        ensureCollection: async () => {},
        dropCollection: async () => {},
        getCollection: async () => null,
        upsertDocument: async () => "doc-1",
        deleteDocuments: async () => 0,
        deleteByFilter: async () => 0,
        retrieve: async () => ({ mode: "vector", chunks: [] }),
        collectionStats: async () => ({ documents: 0, chunks: 0 }),
      },
    };
    const result = injectionDescriptorSchema.safeParse(descriptor);
    expect(result.success).toBe(false);
  });

  it("accepts optional ner", () => {
    const descriptor = {
      embedder: {
        embed: async (texts: string[]) => texts.map(() => new Float32Array([0.1])),
      },
      store: {
        ensureCollection: async () => {},
        dropCollection: async () => {},
        getCollection: async () => null,
        upsertDocument: async () => "doc-1",
        deleteDocuments: async () => 0,
        deleteByFilter: async () => 0,
        retrieve: async () => ({ mode: "vector", chunks: [] }),
        collectionStats: async () => ({ documents: 0, chunks: 0 }),
      },
      ner: {
        ner: async (text: string) => [],
      },
    };
    const result = injectionDescriptorSchema.safeParse(descriptor);
    expect(result.success).toBe(true);
  });
});
