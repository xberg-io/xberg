import { randomUUID } from "node:crypto";

import { NodeParser } from "@llamaindex/core/node-parser";
import { NodeRelationship, TextNode } from "@llamaindex/core/schema";
import type { BaseNode } from "@llamaindex/core/schema";

import type { DocumentMetadata, SerializedChunk, SerializedElement } from "./types.js";

const ELEMENT_METADATA_KEYS = ["element_type", "page_number", "element_index"] as const;
const CHUNK_METADATA_KEYS = [
  "chunk_type",
  "heading_path",
  "page_number",
  "first_page",
  "last_page",
  "chunk_index",
  "total_chunks",
  "token_count",
] as const;
const FORWARDED_KEYS = ["_xberg_chunks", "_xberg_elements"] as const;

const MISSING_ELEMENTS_WARNING =
  "has no '_xberg_chunks' or '_xberg_elements' metadata. Passing through unchanged. " +
  "Use XbergReader with ExtractionConfig(chunking) for native chunk nodes, or " +
  "ExtractionConfig(resultFormat='element_based') for element nodes.";

/** Generates the id for a child node from its running index and source node. */
export type NodeIdFunction = (index: number, source: BaseNode) => string;

/** Constructor options for {@link XbergNodeParser}. */
export interface XbergNodeParserConfig {
  idFunc?: NodeIdFunction;
}

/**
 * Structure-aware node parser for xberg-extracted documents.
 *
 * Turns xberg's output into individual `TextNode` objects, preferring xberg's
 * native chunks (`_xberg_chunks`) and falling back to structural elements
 * (`_xberg_elements`). Documents carrying neither pass through unchanged with a
 * warning. It never calls xberg — it consumes Documents produced by
 * {@link XbergReader}.
 */
export class XbergNodeParser extends NodeParser<TextNode[]> {
  private readonly idFunc: NodeIdFunction;

  constructor(config: XbergNodeParserConfig = {}) {
    super();
    this.idFunc = config.idFunc ?? (() => randomUUID());
  }

  protected parseNodes(documents: TextNode[]): TextNode[] {
    const output: TextNode[] = [];

    for (const node of documents) {
      const chunks = node.metadata[FORWARDED_KEYS[0]];
      if (Array.isArray(chunks) && chunks.length > 0) {
        output.push(...this.nodesFromChunks(node, chunks as SerializedChunk[]));
        continue;
      }

      const elements = node.metadata[FORWARDED_KEYS[1]];
      if (Array.isArray(elements) && elements.length > 0) {
        output.push(...this.nodesFromElements(node, elements as SerializedElement[]));
        continue;
      }

      console.warn(`Document ${node.id_} ${MISSING_ELEMENTS_WARNING}`);
      output.push(node);
    }

    return output;
  }

  private newTextNode(text: string, index: number, source: TextNode, metadata: DocumentMetadata): TextNode {
    return new TextNode({
      text,
      id_: this.idFunc(index, source),
      metadata,
      excludedLlmMetadataKeys: [...source.excludedLlmMetadataKeys],
      metadataSeparator: source.metadataSeparator,
      textTemplate: source.textTemplate,
      relationships: { [NodeRelationship.SOURCE]: source.asRelatedNodeInfo() },
    });
  }

  private nodesFromChunks(source: TextNode, chunks: SerializedChunk[]): TextNode[] {
    const excludedEmbed = [...source.excludedEmbedMetadataKeys, ...CHUNK_METADATA_KEYS];
    const result: TextNode[] = [];
    let index = 0;
    for (const chunk of chunks) {
      const text = chunk.content ?? "";
      if (text.trim().length === 0) {
        continue;
      }
      const meta = chunk.metadata;
      const textNode = this.newTextNode(text, index, source, {
        chunk_type: chunk.chunk_type ?? "unknown",
        heading_path: meta?.heading_path ?? [],
        page_number: meta?.first_page,
        first_page: meta?.first_page,
        last_page: meta?.last_page,
        chunk_index: meta?.chunk_index,
        total_chunks: meta?.total_chunks,
        token_count: meta?.token_count,
      });
      textNode.excludedEmbedMetadataKeys = excludedEmbed;
      result.push(textNode);
      index += 1;
    }
    return result;
  }

  private nodesFromElements(source: TextNode, elements: SerializedElement[]): TextNode[] {
    const excludedEmbed = [...source.excludedEmbedMetadataKeys, ...ELEMENT_METADATA_KEYS];
    const result: TextNode[] = [];
    let index = 0;
    for (const element of elements) {
      const text = element.text ?? "";
      if (text.trim().length === 0) {
        continue;
      }
      const meta = element.metadata;
      const textNode = this.newTextNode(text, index, source, {
        element_type: element.element_type ?? "unknown",
        page_number: meta?.page_number,
        element_index: meta?.element_index,
      });
      textNode.excludedEmbedMetadataKeys = excludedEmbed;
      result.push(textNode);
      index += 1;
    }
    return result;
  }

  protected override postProcessParsedNodes(nodes: TextNode[], parentDocMap: Map<string, TextNode>): TextNode[] {
    const processed = super.postProcessParsedNodes(nodes, parentDocMap);
    return stripForwardedMetadata(processed);
  }
}

/**
 * Remove reader forwarding keys from child nodes only. The base parser copies
 * parent metadata (including the forwarding keys) onto children, so they are
 * stripped here; passthrough documents keep their metadata untouched.
 */
function stripForwardedMetadata(nodes: TextNode[]): TextNode[] {
  for (const node of nodes) {
    if (node.sourceNode !== undefined) {
      for (const key of FORWARDED_KEYS) {
        delete node.metadata[key];
      }
    }
  }
  return nodes;
}
