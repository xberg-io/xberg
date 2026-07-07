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
        upsertDocument: async () => ({ documentId: "1", chunksCount: 1 }),
        query: async () => [],
        delete: async () => {},
        listCollections: async () => [],
        dropCollection: async () => {},
        ensureCollection: async () => {},
      },
    };
    const result = injectionDescriptorSchema.safeParse(descriptor);
    expect(result.success).toBe(true);
  });

  it("rejects missing embedder", () => {
    const descriptor = {
      store: {
        upsertDocument: async () => ({ documentId: "1", chunksCount: 1 }),
        query: async () => [],
        delete: async () => {},
        listCollections: async () => [],
        dropCollection: async () => {},
        ensureCollection: async () => {},
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
        upsertDocument: async () => ({ documentId: "1", chunksCount: 1 }),
        query: async () => [],
        delete: async () => {},
        listCollections: async () => [],
        dropCollection: async () => {},
        ensureCollection: async () => {},
      },
      ner: {
        ner: async (text: string) => [],
      },
    };
    const result = injectionDescriptorSchema.safeParse(descriptor);
    expect(result.success).toBe(true);
  });
});
