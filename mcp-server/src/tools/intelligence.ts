import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  extract,
  extractInputFromBytes,
  extractInputFromUri,
  type ExtractionConfig,
  type NerConfig,
} from "@xberg-io/xberg";

const InputSchema = z.object({
  uri: z.string().optional().describe("File path or HTTPS URL"),
  bytes: z.array(z.number().int().min(0).max(255)).optional().describe("Raw bytes as number array"),
  mime_type: z.string().optional(),
  filename: z.string().optional(),
});

function buildExtractInput(input: z.infer<typeof InputSchema>) {
  if (input.bytes) {
    return extractInputFromBytes(
      Buffer.from(input.bytes),
      input.mime_type ?? "application/octet-stream",
      input.filename ?? null,
    );
  }
  if (input.uri) {
    return extractInputFromUri(input.uri);
  }
  return null;
}

export function registerIntelligenceTools(server: McpServer): void {
  server.tool(
    "extract_entities",
    "Run named-entity recognition (NER) on a document. Returns persons, organizations, locations, emails, and custom label categories. " +
    "Backend 'onnx' uses GLiNER — by default the pinned xberg-io/gliner-models catalog (requires HF_TOKEN + repo access), or pass hf_repo/hf_model_file/hf_tokenizer_file " +
    "to point at any public or private GLiNER ONNX export of your own instead (no access to the private catalog needed; downloads from a custom repo are not checksum-verified). " +
    "Backend 'llm' uses any LLM provider via llm_model (no GLiNER/HF_TOKEN needed — only the provider's own API key, e.g. ANTHROPIC_API_KEY or OPENAI_API_KEY). " +
    "Provide uri (file path or HTTPS URL) or bytes.",
    {
      input: InputSchema,
      backend: z.enum(["onnx", "llm"]).optional().default("llm").describe(
        "'llm' works out of the box with any provider API key. 'onnx' requires either HF_TOKEN + access to the private xberg-io/gliner-models repo, or hf_repo/hf_model_file/hf_tokenizer_file pointing at your own GLiNER ONNX export."
      ),
      categories: z.array(z.string()).optional().describe(
        "Entity categories to detect, e.g. ['PERSON', 'ORG', 'LOCATION', 'EMAIL']. Defaults to all."
      ),
      model: z.string().optional().describe(
        "Only used when backend='onnx' and hf_repo is unset. Pinned catalog model alias, e.g. 'fast' | 'balanced' | 'gliner_large-v2.5'."
      ),
      hf_repo: z.string().optional().describe(
        "Only used when backend='onnx'. Custom HuggingFace repo id (e.g. 'knowledgator/gliner-pii-base-v1.0') to load a GLiNER ONNX export from, bypassing the pinned private catalog. Must be set together with hf_model_file and hf_tokenizer_file."
      ),
      hf_model_file: z.string().optional().describe(
        "Only used when hf_repo is set. Path to the .onnx model file within hf_repo, e.g. 'onnx/model_fp16.onnx'."
      ),
      hf_tokenizer_file: z.string().optional().describe(
        "Only used when hf_repo is set. Path to the tokenizer file within hf_repo, e.g. 'tokenizer.json'."
      ),
      llm_model: z.string().optional().default("anthropic/claude-haiku-4-5").describe(
        "Only used when backend='llm'. Provider/model string, e.g. 'anthropic/claude-haiku-4-5' or 'openai/gpt-4o-mini'."
      ),
      disable_ocr: z.boolean().optional().default(true).describe(
        "Skip OCR when document has a text layer (faster for most docs)."
      ),
    },
    async ({ input, backend, categories, model, hf_repo, hf_model_file, hf_tokenizer_file, llm_model, disable_ocr }) => {
      try {
        const extractInput = buildExtractInput(input);
        if (!extractInput) {
          return { content: [{ type: "text" as const, text: "Error: provide input.uri or input.bytes" }], isError: true };
        }

        const nerConfig: NerConfig = {
          backend: backend as NerConfig["backend"],
          categories: categories as NerConfig["categories"],
          model: backend === "onnx" ? model : undefined,
          hfRepo: backend === "onnx" ? hf_repo : undefined,
          hfModelFile: backend === "onnx" ? hf_model_file : undefined,
          hfTokenizerFile: backend === "onnx" ? hf_tokenizer_file : undefined,
          llm: backend === "llm" ? { model: llm_model } : undefined,
        };

        const config: ExtractionConfig = {
          disableOcr: disable_ocr,
          ner: nerConfig,
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              entities: doc?.entities ?? [],
              entity_count: (doc?.entities ?? []).length,
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text" as const, text: `extract_entities failed: ${msg}` }], isError: true };
      }
    }
  );

  server.tool(
    "structured_extract",
    "Extract structured JSON from a document by providing a JSON Schema. The document is extracted, then the text is sent to an LLM which returns output matching your schema. " +
    "Requires LLM access configured via XBERG_LLM_MODEL env var or llm_model param. " +
    "Returns structured_output matching the schema.",
    {
      input: InputSchema,
      json_schema: z.record(z.unknown()).describe("JSON Schema defining the desired output structure"),
      schema_name: z.string().describe("Short identifier for the schema, e.g. 'invoice' or 'contract_parties'"),
      strict: z.boolean().optional().default(true),
      llm_model: z.string().optional().describe("LLM model to use, e.g. 'openai/gpt-4o'. Falls back to XBERG_LLM_MODEL env var."),
    },
    async ({ input, json_schema, schema_name, strict, llm_model }) => {
      try {
        const extractInput = buildExtractInput(input);
        if (!extractInput) {
          return { content: [{ type: "text" as const, text: "Error: provide input.uri or input.bytes" }], isError: true };
        }

        const config: ExtractionConfig = {
          structuredExtraction: {
            schema: json_schema,
            schemaName: schema_name,
            strict,
            llm: llm_model ? { model: llm_model } : undefined,
          },
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              structured_output: (doc as Record<string, unknown>)?.["structuredOutput"] ?? null,
              content_preview: (doc?.content ?? "").slice(0, 300),
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text" as const, text: `structured_extract failed: ${msg}` }], isError: true };
      }
    }
  );
}
