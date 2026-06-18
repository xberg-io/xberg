import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class KreuzbergExtractJava {
  private static final double NANOS_IN_MILLISECOND = 1_000_000.0;
  private static final int WARMUP_ITERATIONS = 10;
  private static final char LAST_CONTROL_CHAR = 0x1F;

  private KreuzbergExtractJava() {}

  private static ExtractionConfig buildBenchmarkConfig(boolean ocrEnabled) {
    ExtractionConfig.Builder builder = ExtractionConfig.builder().useCache(false);
    if (ocrEnabled) {
      builder.ocr(OcrConfig.builder().build());
    }
    return builder.build();
  }

  /**
   * Parse JSON request from stdin.
   * Supports both plain file paths and JSON objects like:
   * {"path": "/path/to/file", "force_ocr": true}
   *
   * Returns: [filePath, forceOcrString]
   */
  private static String[] parseRequest(String line) {
    String trimmed = line.trim();
    if (trimmed.startsWith("{")) {
      // Minimal JSON parsing for {"path": "...", "force_ocr": true/false}
      int pathStart = trimmed.indexOf("\"path\"");
      String path = "";
      boolean forceOcr = false;

      if (pathStart >= 0) {
        int colonIdx = trimmed.indexOf(':', pathStart);
        int firstQuote = trimmed.indexOf('"', colonIdx + 1);
        int lastQuote = trimmed.indexOf('"', firstQuote + 1);
        if (firstQuote >= 0 && lastQuote > firstQuote) {
          path = trimmed.substring(firstQuote + 1, lastQuote);
        }
      }

      if (trimmed.contains("\"force_ocr\":true") || trimmed.contains("\"force_ocr\": true")) {
        forceOcr = true;
      }

      return new String[] {path, String.valueOf(forceOcr)};
    }

    // Plain file path
    return new String[] {trimmed, "false"};
  }

  public static void main(String[] args) throws Exception {
    boolean ocrEnabled = false;
    List<String> positionalArgs = new ArrayList<>();

    // Parse OCR flags
    for (String arg : args) {
      if ("--ocr".equals(arg)) {
        ocrEnabled = true;
      } else if ("--no-ocr".equals(arg)) {
        ocrEnabled = false;
      } else {
        positionalArgs.add(arg);
      }
    }

    if (positionalArgs.isEmpty()) {
      System.err.println(
          "Usage: KreuzbergExtractJava [--ocr|--no-ocr] <mode> <file_path> [additional_files...]");
      System.err.println("Modes: sync, warmup, batch, server");
      System.exit(1);
    }

    String mode = positionalArgs.get(0);
    if (!"sync".equals(mode)
        && !"warmup".equals(mode)
        && !"batch".equals(mode)
        && !"server".equals(mode)) {
      System.err.printf("Unsupported mode '%s'%n", mode);
      System.exit(1);
    }

    // Enable debug logging if KREUZBERG_BENCHMARK_DEBUG is set
    boolean debug = "true".equalsIgnoreCase(System.getenv("KREUZBERG_BENCHMARK_DEBUG"));

    if ("warmup".equals(mode)) {
      handleWarmupMode(positionalArgs, ocrEnabled, debug);
      return;
    } else if ("server".equals(mode)) {
      handleServerMode(ocrEnabled, debug);
      return;
    } else if ("batch".equals(mode)) {
      handleBatchMode(positionalArgs, ocrEnabled, debug);
      return;
    }

    handleSyncMode(positionalArgs, ocrEnabled, debug);
  }

  private static void handleWarmupMode(
      List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
    if (positionalArgs.size() < 2) {
      System.err.println("Usage: KreuzbergExtractJava warmup <file_path>");
      System.exit(1);
    }

    if (debug) {
      debugLog("Warmup phase starting", "");
    }

    Path path = Path.of(positionalArgs.get(1));
    ExtractionConfig benchConfig = buildBenchmarkConfig(ocrEnabled);
    try {
      for (int i = 0; i < WARMUP_ITERATIONS; i++) {
        Kreuzberg.extractFile(path, benchConfig);
        if (debug && i % 2 == 0) {
          debugLog("Warmup iteration", String.valueOf(i + 1));
        }
      }
      if (debug) {
        debugLog("Warmup phase complete", String.valueOf(WARMUP_ITERATIONS) + " iterations");
      }
      System.out.println("{\"status\":\"warmup_complete\"}");
    } catch (KreuzbergException | RuntimeException | java.io.IOException | Error e) {
      if (debug) {
        debugLog("Warmup failed", e.getClass().getName());
        e.printStackTrace(System.err);
      }
      System.exit(1);
    }
  }

  private static void handleServerMode(boolean ocrEnabled, boolean debug) throws Exception {
    if (debug) {
      debugLog("Server mode starting", "");
    }

    // Signal readiness after JVM + JNI initialization is complete
    System.out.println("READY");
    System.out.flush();

    BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
    String line;
    while ((line = reader.readLine()) != null) {
      String[] req = parseRequest(line);
      String filePath = req[0];
      boolean forceOcr = Boolean.parseBoolean(req[1]);

      if (filePath.isEmpty()) {
        continue;
      }

      // Determine OCR config for this request
      boolean useOcr = ocrEnabled || forceOcr;
      ExtractionConfig benchConfig = buildBenchmarkConfig(useOcr);

      long start = System.nanoTime();
      try {
        Path path = Path.of(filePath);
        ExtractionResult result = Kreuzberg.extractFile(path, benchConfig);
        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
        String json = toJson(result, elapsedMs, useOcr);
        System.out.println(json);
        System.out.flush();
      } catch (Exception | Error e) {
        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
        String errorJson = String.format(
            "{\"error\":%s,\"_extraction_time_ms\":%.3f,\"_ocr_used\":false}",
            quote(fullMessage(e)), elapsedMs);
        System.out.println(errorJson);
        System.out.flush();
      }
    }
  }

  private static void handleBatchMode(
      List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
    if (positionalArgs.size() < 2) {
      System.err.println("Usage: KreuzbergExtractJava batch <file_path> [additional_files...]");
      System.exit(1);
    }

    if (debug) {
      debugLog("Batch mode starting", String.valueOf(positionalArgs.size() - 1) + " files");
    }

    List<Path> paths = new ArrayList<>();
    for (int i = 1; i < positionalArgs.size(); i++) {
      paths.add(Path.of(positionalArgs.get(i)));
    }

    ExtractionConfig benchConfig = buildBenchmarkConfig(ocrEnabled);
    List<String> jsonResults = new ArrayList<>();
    for (Path path : paths) {
      long fileStart = System.nanoTime();
      try {
        ExtractionResult result = Kreuzberg.extractFile(path, benchConfig);
        double fileMs = (System.nanoTime() - fileStart) / NANOS_IN_MILLISECOND;
        jsonResults.add(toJsonWithBatch(result, fileMs, fileMs, ocrEnabled));
      } catch (KreuzbergException | RuntimeException | java.io.IOException | Error e) {
        double fileMs = (System.nanoTime() - fileStart) / NANOS_IN_MILLISECOND;
        if (debug) {
          debugLog(
              "File extraction failed: " + path, e.getClass().getName() + ": " + e.getMessage());
        }
        jsonResults.add("{\"error\":\""
            + e.getMessage().replace("\"", "\\\"")
            + "\",\"_extraction_time_ms\":" + fileMs + ",\"_ocr_used\":false}");
      }
    }

    if (debug) {
      debugLog("Batch extraction completed", String.valueOf(jsonResults.size()) + " results");
    }

    if (jsonResults.size() == 1) {
      System.out.print(jsonResults.get(0));
    } else {
      System.out.print("[");
      for (int i = 0; i < jsonResults.size(); i++) {
        if (i > 0) {
          System.out.print(",");
        }
        System.out.print(jsonResults.get(i));
      }
      System.out.print("]");
    }
  }

  private static void handleSyncMode(
      List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
    if (positionalArgs.size() < 2) {
      System.err.println("Usage: KreuzbergExtractJava sync <file_path>");
      System.exit(1);
    }

    if (debug) {
      debugLog("java.version", System.getProperty("java.version"));
      debugLog("os.name", System.getProperty("os.name"));
      debugLog("os.arch", System.getProperty("os.arch"));
      debugLog("KREUZBERG_FFI_DIR", System.getenv("KREUZBERG_FFI_DIR"));
      debugLog("java.library.path", System.getProperty("java.library.path"));
      debugLog("LD_LIBRARY_PATH", System.getenv("LD_LIBRARY_PATH"));
      debugLog("DYLD_LIBRARY_PATH", System.getenv("DYLD_LIBRARY_PATH"));
      debugLog("Input file", positionalArgs.get(1));
      debugLog("OCR enabled", String.valueOf(ocrEnabled));
    }

    Path path = Path.of(positionalArgs.get(1));
    ExtractionResult result;
    ExtractionConfig benchConfig = buildBenchmarkConfig(ocrEnabled);
    long start = System.nanoTime();
    try {
      if (debug) {
        debugLog("Starting extraction", "");
      }
      result = Kreuzberg.extractFile(path, benchConfig);
      if (debug) {
        debugLog("Extraction completed", "");
      }
    } catch (KreuzbergException | RuntimeException | java.io.IOException | Error e) {
      double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
      if (debug) {
        debugLog("Extraction failed with exception", e.getClass().getName());
        e.printStackTrace(System.err);
      }
      String errorJson = String.format(
          "{\"error\":%s,\"_extraction_time_ms\":%.3f,\"_ocr_used\":false}",
          quote(fullMessage(e)), elapsedMs);
      System.out.println(errorJson);
      return;
    }
    double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

    String json = toJson(result, elapsedMs, ocrEnabled);
    System.out.print(json);
  }

  /**
   * Determine if OCR was actually used based on extraction result metadata.
   * Mirrors the native Rust adapter logic: OCR is used when format_type is "ocr",
   * or when format_type is "pdf" or "image" and OCR was enabled in config.
   */
  private static boolean determineOcrUsed(ExtractionResult result, boolean ocrEnabled) {
    Object formatTypeObj = result.getMetadata().getAdditional().get("format_type");
    String formatType = formatTypeObj != null ? formatTypeObj.toString() : "";
    if ("ocr".equals(formatType)) {
      return true;
    }
    if (("image".equals(formatType) || "pdf".equals(formatType)) && ocrEnabled) {
      return true;
    }
    return false;
  }

  private static String toJson(ExtractionResult result, double elapsedMs, boolean ocrEnabled) {
    StringBuilder builder = new StringBuilder();
    builder.append('{');
    builder.append("\"content\":").append(quote(result.getContent())).append(',');
    builder.append("\"metadata\":{");
    builder.append("\"mimeType\":").append(quote(result.getMimeType())).append(',');
    builder
        .append("\"language\":")
        .append(optionalToJson(result.getMetadata().getLanguage()))
        .append(',');
    builder
        .append("\"date\":")
        .append(optionalToJson(result.getMetadata().getModifiedAt()))
        .append(',');
    builder.append("\"subject\":").append(optionalToJson(result.getMetadata().getSubject()));
    builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
    builder.append(",\"_ocr_used\":").append(determineOcrUsed(result, ocrEnabled));
    builder.append('}');
    return builder.toString();
  }

  private static String toJsonWithBatch(
      ExtractionResult result, double perFileMs, double batchTotalMs, boolean ocrEnabled) {
    StringBuilder builder = new StringBuilder();
    builder.append('{');
    builder.append("\"content\":").append(quote(result.getContent())).append(',');
    builder.append("\"metadata\":{");
    builder.append("\"mimeType\":").append(quote(result.getMimeType()));
    builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", perFileMs));
    builder.append(",\"_batch_total_ms\":").append(String.format("%.3f", batchTotalMs));
    builder.append(",\"_ocr_used\":").append(determineOcrUsed(result, ocrEnabled));
    builder.append('}');
    return builder.toString();
  }

  private static String optionalToJson(java.util.Optional<String> value) {
    return value.isPresent() ? quote(value.get()) : "null";
  }

  // CPD-OFF: quote() is intentionally duplicated in standalone benchmark scripts (no shared
  // classpath)
  private static String quote(String value) {
    if (value == null) {
      return "null";
    }
    StringBuilder sb = new StringBuilder(value.length() + 2);
    sb.append('"');
    for (int i = 0; i < value.length(); i++) {
      char c = value.charAt(i);
      switch (c) {
        case '\\':
          sb.append("\\\\");
          break;
        case '"':
          sb.append("\\\"");
          break;
        case '\n':
          sb.append("\\n");
          break;
        case '\r':
          sb.append("\\r");
          break;
        case '\t':
          sb.append("\\t");
          break;
        case '\b':
          sb.append("\\b");
          break;
        case '\f':
          sb.append("\\f");
          break;
        default:
          if (c <= LAST_CONTROL_CHAR) {
            sb.append(String.format("\\u%04x", (int) c));
          } else {
            sb.append(c);
          }
      }
    }
    sb.append('"');
    return sb.toString();
  }
  // CPD-ON

  private static String fullMessage(Throwable e) {
    StringBuilder sb = new StringBuilder();
    sb.append(e.getMessage() != null ? e.getMessage() : e.getClass().getName());
    Throwable cause = e.getCause();
    while (cause != null) {
      String msg = cause.getMessage();
      if (msg != null && !msg.isEmpty()) {
        sb.append(": ").append(msg);
      }
      cause = cause.getCause();
    }
    return sb.toString();
  }

  private static void debugLog(String key, String value) {
    if (value == null) {
      value = "(null)";
    }
    System.err.printf("[BENCHMARK_DEBUG] %-30s = %s%n", key, value);
  }
}
