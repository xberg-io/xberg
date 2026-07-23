import { createHash } from "node:crypto";
import { basename, resolve } from "node:path";

import { Document } from "@llamaindex/core/schema";

import type { ExtractInput, ExtractionConfig } from "@xberg-io/xberg";

import type { DocumentMetadata, SerializedChunk, SerializedElement, XbergBytesInput, XbergInput } from "./types.js";

// Default result format so `ExtractedDocument.elements` is populated and the
// companion XbergNodeParser can split documents element-by-element. ~keep
const DEFAULT_RESULT_FORMAT = "element_based";
const PAGE_RESULT_FORMAT = "unified";

/**
 * The camelCase shapes the mappers read from the `@xberg-io/xberg` binding.
 * The binding exposes these via unexported `Js*` aliases, so structural
 * interfaces are declared locally and the native result is cast through them.
 */
interface XMetadata {
  title?: string | null;
  subject?: string | null;
  authors?: string[] | null;
  keywords?: string[] | null;
  language?: string | null;
  createdAt?: string | null;
  modifiedAt?: string | null;
  createdBy?: string | null;
  modifiedBy?: string | null;
  category?: string | null;
  tags?: string[] | null;
  documentVersion?: string | null;
  abstractText?: string | null;
  outputFormat?: string | null;
}

interface XTable {
  markdown?: string | null;
}

interface XPage {
  pageNumber: number;
  content: string;
  tables?: XTable[] | null;
}

interface XElement {
  elementType: unknown;
  text: string;
  metadata?: { pageNumber?: number | null; elementIndex?: number | null } | null;
}

interface XChunk {
  content: string;
  chunkType: unknown;
  metadata?: {
    chunkIndex?: number | null;
    totalChunks?: number | null;
    firstPage?: number | null;
    lastPage?: number | null;
    headingPath?: string[] | null;
    tokenCount?: number | null;
  } | null;
}

interface XKeyword {
  text: string;
  score: number;
  algorithm: unknown;
}

interface XWarning {
  source: string;
  message: string;
}

interface XAnnotation {
  annotationType: unknown;
  content?: string | null;
  pageNumber: number;
}

interface XBoundingBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

interface XImage {
  data?: Uint8Array | Buffer | number[] | null;
  format?: string | null;
  imageIndex?: number | null;
  pageNumber?: number | null;
  width?: number | null;
  height?: number | null;
  colorspace?: string | null;
  bitsPerComponent?: number | null;
  isMask?: boolean | null;
  description?: string | null;
  boundingBox?: XBoundingBox | null;
  ocrResult?: { content?: string | null } | null;
}

interface XCounts {
  pages?: number | null;
}

export interface XDocument {
  content?: string | null;
  mimeType?: string | null;
  metadata?: XMetadata | null;
  counts?: XCounts | null;
  tables?: XTable[] | null;
  pages?: XPage[] | null;
  elements?: XElement[] | null;
  chunks?: XChunk[] | null;
  images?: XImage[] | null;
  qualityScore?: number | null;
  detectedLanguages?: string[] | null;
  processingWarnings?: XWarning[] | null;
  extractedKeywords?: XKeyword[] | null;
  annotations?: XAnnotation[] | null;
}

export interface XError {
  index: number;
  errorType: string;
  message: string;
}

export interface XResult {
  results?: XDocument[] | null;
  errors?: XError[] | null;
}

/** Tracks the origin of one extraction input for metadata/id purposes. */
export interface Source {
  path?: string;
  data?: Uint8Array;
}

/** A successfully extracted document paired with its source descriptor. */
export type DocSource = [XDocument, Source];

// Scalar / list `Metadata` fields copied verbatim into document metadata,
// mapping the binding's camelCase field to its snake_case output key. ~keep
const METADATA_FIELDS: ReadonlyArray<readonly [keyof XMetadata, string]> = [
  ["title", "title"],
  ["subject", "subject"],
  ["authors", "authors"],
  ["keywords", "keywords"],
  ["language", "language"],
  ["createdAt", "created_at"],
  ["modifiedAt", "modified_at"],
  ["createdBy", "created_by"],
  ["modifiedBy", "modified_by"],
  ["category", "category"],
  ["tags", "tags"],
  ["documentVersion", "document_version"],
  ["abstractText", "abstract_text"],
];

/** Return true when the config opts into page extraction. */
export function pagesRequested(config: ExtractionConfig | undefined): boolean {
  return Boolean(config?.pages?.extractPages);
}

/**
 * Return the `ExtractionConfig` to use, defaulting `resultFormat`.
 *
 * With no explicit `resultFormat` the reader defaults to `element_based` so the
 * element stream is populated and forwarded to the node parser. When the caller
 * opts into page extraction the reader defaults to `unified` instead, so pages
 * split cleanly without replicating the document-wide element stream. An
 * explicit `resultFormat` always wins.
 */
export function buildExtractionConfig(config: ExtractionConfig | undefined): ExtractionConfig {
  const base = { ...config };
  if (base.resultFormat !== undefined) {
    return base;
  }
  const resultFormat = pagesRequested(base) ? PAGE_RESULT_FORMAT : DEFAULT_RESULT_FORMAT;
  return { ...base, resultFormat } as unknown as ExtractionConfig;
}

function isBytesInput(input: XbergInput): input is XbergBytesInput {
  return typeof input === "object" && !Array.isArray(input) && "data" in input;
}

/** Validate the reader input and build parallel xberg inputs and sources. */
export function prepareInputs(input: XbergInput): { inputs: ExtractInput[]; sources: Source[] } {
  if (typeof input === "string" || Array.isArray(input)) {
    const paths = Array.isArray(input) ? input : [input];
    return {
      inputs: paths.map((path) => ({ kind: "uri", uri: path })),
      sources: paths.map((path) => ({ path })),
    };
  }

  if (isBytesInput(input)) {
    const { data, mimeType } = input;
    if (Array.isArray(data)) {
      if (!Array.isArray(mimeType) || data.length !== mimeType.length) {
        throw new Error("data and mimeType must be parallel lists of equal length");
      }
      return {
        inputs: data.map((bytes, index) => ({ kind: "bytes", bytes, mimeType: mimeType[index] })),
        sources: data.map((bytes) => ({ data: bytes })),
      };
    }
    if (typeof mimeType !== "string") {
      throw new Error("mimeType must be a string for single bytes input");
    }
    return {
      inputs: [{ kind: "bytes", bytes: data, mimeType }],
      sources: [{ data }],
    };
  }

  throw new Error("Either file_path or data must be provided");
}

/**
 * Pair extracted documents with their sources, handling per-input errors.
 *
 * Successful documents preserve input order, so the surviving sources are the
 * inputs whose index is not in the error set. When `raiseOnError` is set the
 * first error is rethrown.
 */
export function mapResults(result: XResult, sources: Source[], raiseOnError: boolean): DocSource[] {
  const errors = result.errors ?? [];
  const failedIndices = new Set(errors.map((error) => error.index));
  for (const error of errors) {
    console.warn(`xberg failed to extract input ${error.index} (${error.errorType}): ${error.message}`);
  }
  if (errors.length > 0 && raiseOnError) {
    const first = errors[0];
    throw new Error(`xberg extraction failed for input ${first.index}: ${first.message}`);
  }

  const surviving = sources.filter((_, index) => !failedIndices.has(index));
  const results = result.results ?? [];
  const count = Math.min(results.length, surviving.length);
  const paired: DocSource[] = [];
  for (let index = 0; index < count; index += 1) {
    paired.push([results[index], surviving[index]]);
  }
  return paired;
}

function serializeMetadata(metadata: XMetadata | null | undefined): DocumentMetadata {
  if (metadata == null) {
    return {};
  }
  const result: DocumentMetadata = {};
  for (const [field, key] of METADATA_FIELDS) {
    const value = metadata[field];
    if (value != null) {
      result[key] = value;
    }
  }
  return result;
}

/** Serialize xberg `Element` objects into the reader/node-parser contract. */
export function serializeElements(elements: XElement[]): SerializedElement[] {
  return elements.map((element) => ({
    text: element.text,
    element_type: String(element.elementType),
    metadata: {
      page_number: element.metadata?.pageNumber ?? null,
      element_index: element.metadata?.elementIndex ?? null,
    },
  }));
}

/** Serialize xberg native `Chunk` objects into the node-parser contract. */
export function serializeChunks(chunks: XChunk[]): SerializedChunk[] {
  return chunks.map((chunk) => ({
    content: chunk.content,
    chunk_type: String(chunk.chunkType),
    metadata: {
      chunk_index: chunk.metadata?.chunkIndex ?? null,
      total_chunks: chunk.metadata?.totalChunks ?? null,
      first_page: chunk.metadata?.firstPage ?? null,
      last_page: chunk.metadata?.lastPage ?? null,
      heading_path: [...(chunk.metadata?.headingPath ?? [])],
      token_count: chunk.metadata?.tokenCount ?? null,
    },
  }));
}

function serializeImages(images: XImage[], pageNumber: number | undefined): DocumentMetadata[] {
  const serialized: DocumentMetadata[] = [];
  for (const image of images) {
    if (pageNumber !== undefined && image.pageNumber !== pageNumber) {
      continue;
    }
    const raw = image.data;
    const bytes = raw == null ? null : Buffer.from(raw as Uint8Array | number[]);
    const entry: DocumentMetadata = {
      format: image.format,
      image_index: image.imageIndex,
      page_number: image.pageNumber,
      width: image.width,
      height: image.height,
      colorspace: image.colorspace,
      bits_per_component: image.bitsPerComponent,
      is_mask: image.isMask,
      description: image.description,
      data: bytes ? bytes.toString("base64") : null,
    };
    if (image.boundingBox != null) {
      entry.bounding_box = {
        x0: image.boundingBox.x0,
        y0: image.boundingBox.y0,
        x1: image.boundingBox.x1,
        y1: image.boundingBox.y1,
      };
    }
    if (image.ocrResult != null) {
      entry.ocr_result = image.ocrResult.content;
    }
    serialized.push(entry);
  }
  return serialized;
}

/** Options for {@link buildMetadata}. */
export interface BuildMetadataOptions {
  document: XDocument;
  filePath?: string;
  source?: string;
  extraInfo?: Record<string, unknown>;
  pageNumber?: number;
}

/** Flatten an `ExtractedDocument` into a JSON-serialisable metadata dict. */
export function buildMetadata(options: BuildMetadataOptions): DocumentMetadata {
  const { document, filePath, source, extraInfo, pageNumber } = options;
  const meta: DocumentMetadata = {};

  if (filePath !== undefined) {
    meta.file_name = basename(filePath);
    meta.file_path = filePath;
  } else if (source !== undefined) {
    meta.file_name = source;
    meta.file_path = source;
  }

  meta.file_type = document.mimeType;
  meta.total_pages = document.counts?.pages;

  if (pageNumber !== undefined) {
    meta.page_number = pageNumber;
  }

  Object.assign(meta, serializeMetadata(document.metadata));
  meta.output_format = document.metadata?.outputFormat;

  if (document.qualityScore != null) {
    meta.quality_score = document.qualityScore;
  }
  if (document.detectedLanguages != null) {
    meta.detected_languages = document.detectedLanguages;
  }
  if (document.processingWarnings && document.processingWarnings.length > 0) {
    meta.processing_warnings = document.processingWarnings.map((warning) => ({
      source: warning.source,
      message: warning.message,
    }));
  }
  if (document.extractedKeywords && document.extractedKeywords.length > 0) {
    meta.extracted_keywords = document.extractedKeywords.map((keyword) => ({
      text: keyword.text,
      score: keyword.score,
      algorithm: String(keyword.algorithm),
    }));
  }
  if (document.annotations && document.annotations.length > 0) {
    meta.annotations = document.annotations.map((annotation) => ({
      annotation_type: String(annotation.annotationType),
      content: annotation.content,
      page_number: annotation.pageNumber,
    }));
  }
  if (document.elements != null) {
    meta._xberg_elements = serializeElements(document.elements);
  }
  if (document.chunks && document.chunks.length > 0) {
    meta._xberg_chunks = serializeChunks(document.chunks);
  }
  if (document.images && document.images.length > 0) {
    meta.images = serializeImages(document.images, pageNumber);
  }

  if (extraInfo) {
    Object.assign(meta, extraInfo);
  }

  return meta;
}

/** Options for {@link generateDocId}. */
export interface GenerateDocIdOptions {
  filePath?: string;
  data?: Uint8Array;
  pageNumber?: number;
}

/** Generate a deterministic document ID via SHA-256 of the resolved source. */
export function generateDocId(options: GenerateDocIdOptions): string {
  const { filePath, data, pageNumber } = options;
  if (filePath === undefined && data === undefined) {
    throw new Error("Either file_path or data must be provided");
  }
  const hasher = createHash("sha256");
  if (filePath !== undefined) {
    hasher.update(resolve(filePath));
  } else if (data !== undefined) {
    hasher.update(data);
  }
  if (pageNumber !== undefined) {
    hasher.update(String(pageNumber));
  }
  return hasher.digest("hex");
}

/** Return metadata keys excluded from LLM and embedding input. */
export function excludedKeys(meta: DocumentMetadata): string[] {
  const keys: string[] = [];
  if ("_xberg_elements" in meta) {
    keys.push("_xberg_elements");
  }
  if ("_xberg_chunks" in meta) {
    keys.push("_xberg_chunks");
  }
  if ("images" in meta) {
    keys.push("images");
  }
  return keys;
}

/** Append table markdown to content when a table is not already inlined. */
export function appendTables(content: string, tables: XTable[] | null | undefined): string {
  if (!tables || tables.length === 0) {
    return content;
  }
  let result = content;
  for (const table of tables) {
    const markdown = table.markdown;
    if (markdown && !result.includes(markdown.trim())) {
      result = `${result.replace(/\s+$/, "")}\n\n${markdown}`;
    }
  }
  return result;
}

/**
 * Build Documents from extracted documents.
 *
 * When an element stream or native chunk list is present the source becomes a
 * single Document carrying `_xberg_elements` / `_xberg_chunks`. Otherwise, when
 * pages are present, one Document is emitted per page. Elements and chunks are
 * document-global, so per-page splitting is suppressed for them to avoid
 * replicating every element or chunk onto every page.
 */
export function resultsToDocuments(docSources: DocSource[], extraInfo?: Record<string, unknown>): Document[] {
  const documents: Document[] = [];
  for (const [document, source] of docSources) {
    const sourceLabel = source.data !== undefined ? "bytes" : undefined;
    const hasPages = Boolean(document.pages && document.pages.length > 0);
    const hasChunks = Boolean(document.chunks && document.chunks.length > 0);

    if (hasPages && document.elements == null && !hasChunks) {
      for (const page of document.pages ?? []) {
        const content = appendTables(page.content, page.tables);
        const meta = buildMetadata({
          document,
          filePath: source.path,
          source: sourceLabel,
          extraInfo,
          pageNumber: page.pageNumber,
        });
        const excluded = excludedKeys(meta);
        documents.push(
          new Document({
            text: content,
            id_: generateDocId({ filePath: source.path, data: source.data, pageNumber: page.pageNumber }),
            metadata: meta,
            excludedLlmMetadataKeys: excluded,
            excludedEmbedMetadataKeys: [...excluded],
          }),
        );
      }
    } else {
      const content = appendTables(document.content ?? "", document.tables);
      const meta = buildMetadata({ document, filePath: source.path, source: sourceLabel, extraInfo });
      const excluded = excludedKeys(meta);
      documents.push(
        new Document({
          text: content,
          id_: generateDocId({ filePath: source.path, data: source.data }),
          metadata: meta,
          excludedLlmMetadataKeys: excluded,
          excludedEmbedMetadataKeys: [...excluded],
        }),
      );
    }
  }
  return documents;
}
