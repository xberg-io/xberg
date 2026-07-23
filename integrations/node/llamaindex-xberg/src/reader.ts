import { extract, extractBatch } from "@xberg-io/xberg";
import type { ExtractionConfig, ExtractionResult } from "@xberg-io/xberg";

import type { BaseReader, Document } from "@llamaindex/core/schema";

import { buildExtractionConfig, mapResults, prepareInputs, resultsToDocuments, type XResult } from "./readerMapping.js";
import type { XbergInput, XbergReaderConfig } from "./types.js";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Reader for 90+ document formats powered by xberg's Rust extraction engine.
 *
 * Supports file paths, raw bytes, batch input, per-page splitting, and true
 * async via xberg's native `extract` / `extractBatch` functions. A single input
 * is dispatched to `extract`; multiple inputs go through `extractBatch`.
 */
export class XbergReader implements BaseReader<Document> {
  private readonly raiseOnError: boolean;
  private readonly extractionConfig?: ExtractionConfig;

  constructor(config: XbergReaderConfig = {}) {
    this.raiseOnError = config.raiseOnError ?? false;
    this.extractionConfig = config.extractionConfig;
  }

  async loadData(input: XbergInput, extraInfo?: Record<string, unknown>): Promise<Document[]> {
    const { inputs, sources } = prepareInputs(input);
    const config = buildExtractionConfig(this.extractionConfig);

    let result: ExtractionResult;
    try {
      result = inputs.length === 1 ? await extract(inputs[0], config) : await extractBatch(inputs, config);
    } catch (error) {
      if (this.raiseOnError) {
        throw error;
      }
      console.warn(`xberg extraction failed: ${errorMessage(error)}`);
      return [];
    }

    const docSources = mapResults(result as unknown as XResult, sources, this.raiseOnError);
    return resultsToDocuments(docSources, extraInfo);
  }
}
