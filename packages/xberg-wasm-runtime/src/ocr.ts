import type { CacheConfig, OcrInterface, OcrOpts, OcrResult } from "./types";
import type {
  PaddleOcrResult,
  RecognitionResult,
} from "ppu-paddle-ocr";

const LANGUAGE_MODEL_MAP: Record<string, string> = {
  en: "V4_EN_MOBILE_MODEL",
};
const DEFAULT_MODEL_EXPORT = "V6_SMALL_MODEL";

/**
 * Create an OCR interface using ppu-paddle-ocr (PaddleOCR over ONNX Runtime).
 * Returns null if the model/backend cannot be loaded or the feature is
 * disabled. Optional; if not injected into the engine, the engine falls
 * back to in-binary Tesseract OCR.
 *
 * IMPORTANT — real v6 API, not the `Paddle`/`ocr()` shape from earlier
 * drafts of this module's spec: `ppu-paddle-ocr` major version 6 exports a
 * `PaddleOcrService` class (constructed with `new PaddleOcrService(options)`,
 * then `await service.initialize()`), not a `Paddle` class with a bare
 * `.ocr()` method. There is no `/web` re-export usable here: `ppu-paddle-ocr
 * /web` delegates image decoding to `ppu-ocv/web`, which requires browser DOM
 * globals (`OffscreenCanvas`, `document`, `createImageBitmap`) that do not
 * exist in a plain Node/Vitest environment. The Node entry point
 * (`ppu-paddle-ocr` bare import) is used instead, which decodes images via
 * `ppu-ocv`'s `@napi-rs/canvas` backend and runs inference through
 * `onnxruntime-node`.
 *
 * `onnxruntime-node` is an *optional* peer dependency of `ppu-paddle-ocr`
 * and is not bundled with this package (avoiding a mandatory large native
 * binary for consumers who don't need OCR). When it — or any other part of
 * the load/initialize chain — is unavailable, this factory returns `null`
 * rather than throwing, matching the optional-injection contract shared
 * with `createNer`.
 */
export async function createOcr(
  config?: CacheConfig
): Promise<OcrInterface | null> {
  try {
    const { PaddleOcrService, ...models } = await import("ppu-paddle-ocr");

    const modelId = config?.models?.ocr;
    const modelPreset =
      (modelId && (models as Record<string, unknown>)[modelId]) ||
      (models as Record<string, unknown>)[DEFAULT_MODEL_EXPORT];

    const service = new PaddleOcrService(
      modelPreset ? { model: modelPreset as never } : undefined
    );
    await service.initialize();

    /**
     * Run OCR on an image. `opts.languages` selects a language-specific
     * preset model (only `"en"` is currently mapped; unmapped languages
     * fall back to the multilingual default model chosen at construction).
     * `opts.useCpu` is accepted for interface compatibility but has no
     * effect here — the Node execution provider is CPU-only regardless
     * (WebGPU acceleration is a browser-only capability of this backend).
     */
    async function ocr(bytes: Uint8Array, opts?: OcrOpts): Promise<OcrResult> {
      try {
        const requestedLanguage = opts?.languages?.[0];
        const languageModelKey =
          requestedLanguage && LANGUAGE_MODEL_MAP[requestedLanguage];
        if (languageModelKey && languageModelKey !== DEFAULT_MODEL_EXPORT) {
          const preset = (models as Record<string, unknown>)[languageModelKey];
          if (preset) {
            await service.changeDetectionModel(
              (preset as { detection: string }).detection
            );
            await service.changeRecognitionModel(
              (preset as { recognition: string }).recognition
            );
            await service.changeTextDictionary(
              (preset as { charactersDictionary: string }).charactersDictionary
            );
          }
        }

        const buffer = bytes.buffer.slice(
          bytes.byteOffset,
          bytes.byteOffset + bytes.byteLength
        ) as ArrayBuffer;

        const result = (await service.recognize(buffer, {
          flatten: false,
        })) as PaddleOcrResult;

        return toOcrResult(result);
      } catch (err) {
        console.error("[ocr] inference failed:", err);
        return { text: "", lines: [] };
      }
    }

    return { ocr };
  } catch (err) {
    console.warn("[ocr] ppu-paddle-ocr load failed, falling back to in-binary:", err);
    return null;
  }
}

/**
 * Convert a `PaddleOcrResult` (grouped by detected text lines, each line an
 * array of per-word `RecognitionResult`s) into the xberg `OcrResult` shape:
 * one entry per line with merged text, averaged confidence, and a bbox that
 * is the union of all word boxes in that line.
 */
function toOcrResult(result: PaddleOcrResult): OcrResult {
  const lines = result.lines.map((words: RecognitionResult[]) => {
    const text = words.map((w) => w.text).join(" ");
    const confidence =
      words.length > 0
        ? words.reduce((sum, w) => sum + w.confidence, 0) / words.length
        : 0;

    const bbox =
      words.length > 0
        ? unionBox(words.map((w) => w.box))
        : undefined;

    return { text, confidence, bbox };
  });

  return { text: result.text, lines };
}

function unionBox(
  boxes: Array<{ x: number; y: number; width: number; height: number }>
): { x: number; y: number; w: number; h: number } {
  const minX = Math.min(...boxes.map((b) => b.x));
  const minY = Math.min(...boxes.map((b) => b.y));
  const maxX = Math.max(...boxes.map((b) => b.x + b.width));
  const maxY = Math.max(...boxes.map((b) => b.y + b.height));

  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}
