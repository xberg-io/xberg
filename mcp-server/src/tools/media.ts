import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  extract,
  extractInputFromBytes,
  extractInputFromUri,
  type ExtractionConfig,
} from "@xberg-io/xberg";

export function registerMediaTools(server: McpServer): void {
  server.tool(
    "transcribe_audio",
    "Transcribe audio or video files to text using Whisper ONNX (offline, runs locally). " +
    "Supports mp3, m4a, wav, ogg, flac, mp4, webm, mkv, and more. " +
    "Model downloads automatically on first use: tiny=75MB, base=150MB, small=480MB, medium=1.5GB. " +
    "Provide uri (file path or HTTPS URL) or bytes.",
    {
      uri: z.string().optional().describe("File path or HTTPS URL to audio/video file"),
      bytes: z.array(z.number().int().min(0).max(255)).optional(),
      mime_type: z.string().optional().describe("e.g. 'audio/mpeg' for mp3, 'video/mp4' for mp4"),
      filename: z.string().optional(),
      model: z.enum(["tiny", "base", "small", "medium", "large", "turbo"])
        .optional()
        .default("base")
        .describe("Whisper model size. 'base' is fast and accurate for most use cases."),
      language: z.string().optional().describe("ISO 639-1 code e.g. 'en', 'fr', 'de'. Omit for auto-detect."),
    },
    async ({ uri, bytes, mime_type, filename, model, language }) => {
      try {
        let extractInput;
        if (bytes) {
          extractInput = extractInputFromBytes(
            Buffer.from(bytes),
            mime_type ?? "audio/mpeg",
            filename ?? null,
          );
        } else if (uri) {
          extractInput = extractInputFromUri(uri);
        } else {
          return { content: [{ type: "text" as const, text: "Error: provide uri or bytes" }], isError: true };
        }

        const config: ExtractionConfig = {
          transcription: {
            enabled: true,
            model,
            language,
          },
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              transcript: doc?.content ?? "",
              duration_ms: doc?.metadata?.["audio"]?.["durationMs"] ?? null,
              detected_language: (doc?.detectedLanguages ?? [])[0] ?? null,
              model,
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text" as const, text: `transcribe_audio failed: ${msg}` }], isError: true };
      }
    }
  );
}
