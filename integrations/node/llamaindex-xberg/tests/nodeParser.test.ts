import { describe, expect, it } from "vitest";

import { TextNode } from "@llamaindex/core/schema";

import { XbergNodeParser } from "../src/nodeParser";
import type { SerializedChunk, SerializedElement } from "../src/types";

function chunk(content: string, overrides: Partial<SerializedChunk> = {}): SerializedChunk {
  return {
    content,
    chunk_type: "Text",
    metadata: {
      chunk_index: 0,
      total_chunks: 1,
      first_page: 1,
      last_page: 1,
      heading_path: ["H1"],
      token_count: 4,
    },
    ...overrides,
  };
}

function element(text: string, overrides: Partial<SerializedElement> = {}): SerializedElement {
  return {
    text,
    element_type: "NarrativeText",
    metadata: { page_number: 1, element_index: 0 },
    ...overrides,
  };
}

function sourceNode(metadata: Record<string, unknown>): TextNode {
  return new TextNode({
    text: "source document",
    metadata,
    excludedLlmMetadataKeys: ["_xberg_chunks", "_xberg_elements"],
    excludedEmbedMetadataKeys: ["_xberg_chunks", "_xberg_elements"],
  });
}

const idParser = () => new XbergNodeParser({ idFunc: (index) => `node-${index}` });

describe("XbergNodeParser", () => {
  it("should prefer chunks over elements", () => {
    const node = sourceNode({
      _xberg_chunks: [chunk("chunk text")],
      _xberg_elements: [element("element text")],
    });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect(nodes).toHaveLength(1);
    expect(nodes[0].text).toBe("chunk text");
    expect(nodes[0].metadata.chunk_type).toBe("Text");
    expect(nodes[0].metadata.page_number).toBe(1);
    expect(nodes[0].metadata.chunk_index).toBe(0);
  });

  it("should fall back to elements when no chunks", () => {
    const node = sourceNode({ _xberg_elements: [element("element text")] });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect(nodes).toHaveLength(1);
    expect(nodes[0].text).toBe("element text");
    expect(nodes[0].metadata.element_type).toBe("NarrativeText");
    expect(nodes[0].metadata.element_index).toBe(0);
  });

  it("should pass through documents with no forwarding metadata", () => {
    const node = sourceNode({ title: "plain" });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toBe(node);
    expect(nodes[0].metadata.title).toBe("plain");
  });

  it("should skip blank chunks and index only non-empty items", () => {
    const node = sourceNode({
      _xberg_chunks: [chunk("first"), chunk("   ", { chunk_type: "Blank" }), chunk("second")],
    });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect(nodes.map((n) => n.text)).toEqual(["first", "second"]);
    expect(nodes.map((n) => n.id_)).toEqual(["node-0", "node-1"]);
  });

  it("should exclude chunk metadata keys from embeddings", () => {
    const node = sourceNode({ _xberg_chunks: [chunk("c")] });
    const nodes = idParser().getNodesFromDocuments([node]);
    for (const key of ["chunk_type", "page_number", "first_page", "chunk_index", "total_chunks", "token_count"]) {
      expect(nodes[0].excludedEmbedMetadataKeys).toContain(key);
    }
  });

  it("should strip forwarding keys from child nodes and set the source relationship", () => {
    const node = sourceNode({ _xberg_chunks: [chunk("c")] });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect("_xberg_chunks" in nodes[0].metadata).toBe(false);
    expect("_xberg_elements" in nodes[0].metadata).toBe(false);
    expect(nodes[0].sourceNode).toBeDefined();
  });

  it("should default chunk_type and heading_path when absent", () => {
    const node = sourceNode({
      _xberg_chunks: [
        {
          content: "c",
          chunk_type: undefined as unknown as string,
          metadata: {
            chunk_index: 0,
            total_chunks: 1,
            first_page: null,
            last_page: null,
            heading_path: [],
            token_count: null,
          },
        },
      ],
    });
    const nodes = idParser().getNodesFromDocuments([node]);
    expect(nodes[0].metadata.chunk_type).toBe("unknown");
    expect(nodes[0].metadata.heading_path).toEqual([]);
  });
});
