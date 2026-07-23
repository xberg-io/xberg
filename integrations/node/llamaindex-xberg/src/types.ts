import type { ExtractionConfig } from "@xberg-io/xberg";

/** JSON-serialisable element dict stored under `_xberg_elements`. */
export interface SerializedElementMetadata {
  page_number: number | null;
  element_index: number | null;
}

/**
 * JSON-serialisable element dict stored under `_xberg_elements`. This shape is
 * the contract between {@link XbergReader} and {@link XbergNodeParser}.
 */
export interface SerializedElement {
  text: string;
  element_type: string;
  metadata: SerializedElementMetadata;
}

/** JSON-serialisable chunk metadata stored under `_xberg_chunks`. */
export interface SerializedChunkMetadata {
  chunk_index: number | null;
  total_chunks: number | null;
  first_page: number | null;
  last_page: number | null;
  heading_path: string[];
  token_count: number | null;
}

/**
 * JSON-serialisable chunk dict stored under `_xberg_chunks`. This shape is the
 * contract between {@link XbergReader} and {@link XbergNodeParser}.
 */
export interface SerializedChunk {
  content: string;
  chunk_type: string;
  metadata: SerializedChunkMetadata;
}

/** Open-ended document metadata mapping written onto every emitted node. */
export type DocumentMetadata = Record<string, unknown>;

/** Constructor options for {@link XbergReader}. */
export interface XbergReaderConfig {
  /** Propagate extraction failures instead of logging and skipping them. */
  raiseOnError?: boolean;
  /** xberg `ExtractionConfig` controlling output format, OCR, and result format. */
  extractionConfig?: ExtractionConfig;
}

/** Raw bytes input accepted by {@link XbergReader.loadData}. */
export interface XbergBytesInput {
  data: Uint8Array | Uint8Array[];
  mimeType: string | string[];
}

/** File path(s) or raw bytes accepted by {@link XbergReader.loadData}. */
export type XbergInput = string | string[] | XbergBytesInput;
