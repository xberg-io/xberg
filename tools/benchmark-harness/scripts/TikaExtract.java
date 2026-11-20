package io.xberg.benchmark;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.apache.tika.metadata.Metadata;
import org.apache.tika.parser.AutoDetectParser;
import org.apache.tika.parser.ParseContext;
import org.apache.tika.parser.ocr.TesseractOCRConfig;
import org.apache.tika.sax.BodyContentHandler;

public final class TikaExtract {
  private static final double NANOS_IN_MILLISECOND = 1_000_000.0;

  private TikaExtract() {}

  public static void main(String[] args) {
    boolean ocrEnabled = false;
    // "jpn_vert"). Tika's TesseractOCRConfig.setLanguage takes these codes
    // directly; the harness forwards the fixture's language via --ocr-lang,
    // defaulting to eng.
    String ocrLanguage = "eng";
    List<String> positionalArgs = new ArrayList<>();

    for (String arg : args) {
      if ("--ocr".equals(arg)) {
        ocrEnabled = true;
      } else if ("--no-ocr".equals(arg)) {
        ocrEnabled = false;
      } else if (arg.startsWith("--ocr-lang=")) {
        String value = arg.substring("--ocr-lang=".length());
        if (!value.isEmpty()) {
          ocrLanguage = value;
        }
      } else {
        positionalArgs.add(arg);
      }
    }

    if (positionalArgs.isEmpty()) {
      System.err.println(
          "Usage: TikaExtract [--ocr|--no-ocr] <mode> <file1> [file2] ...");
      System.err.println("Modes: sync, batch, server");
      System.exit(1);
    }

    String mode = positionalArgs.get(0);
    if (!"sync".equals(mode) && !"batch".equals(mode) &&
        !"server".equals(mode)) {
      System.err.printf("Unsupported mode '%s'%n", mode);
      System.exit(1);
    }

    // Enable debug logging if TIKA_BENCHMARK_DEBUG is set
    boolean debug =
        "true".equalsIgnoreCase(System.getenv("TIKA_BENCHMARK_DEBUG"));

    if (debug) {
      TikaJson.debugLog("java.version", System.getProperty("java.version"));
      TikaJson.debugLog("os.name", System.getProperty("os.name"));
      TikaJson.debugLog("os.arch", System.getProperty("os.arch"));
      TikaJson.debugLog("Mode", mode);
      TikaJson.debugLog("OCR enabled", String.valueOf(ocrEnabled));
      TikaJson.debugLog("Files to process",
                        String.valueOf(positionalArgs.size() - 1));
    }

    try {
      if ("sync".equals(mode)) {
        if (positionalArgs.size() < 2) {
          System.err.println("Sync mode requires exactly one file");
          System.exit(1);
        }
        processSyncMode(positionalArgs.get(1), ocrEnabled, ocrLanguage, debug);
      } else if ("batch".equals(mode)) {
        processBatchMode(positionalArgs, ocrEnabled, ocrLanguage, debug);
      } else {
        processServerMode(ocrEnabled, ocrLanguage);
      }
    } catch (Exception e) {
      if (debug) {
        TikaJson.debugLog("Processing failed with exception",
                          e.getClass().getName());
        e.printStackTrace(System.err);
      } else {
        e.printStackTrace(System.err);
      }
      System.exit(1);
    }
  }

  private static void processSyncMode(String filePath, boolean ocrEnabled,
                                      String ocrLanguage, boolean debug)
      throws Exception {
    if (debug) {
      TikaJson.debugLog("Input file", filePath);
    }

    Path path = Path.of(filePath);
    ExtractionData data;
    long start = System.nanoTime();

    try {
      if (debug) {
        TikaJson.debugLog("Starting extraction", "");
      }
      data = extractFile(path.toFile(), ocrEnabled, ocrLanguage);
      if (debug) {
        TikaJson.debugLog("Extraction completed", "");
      }
    } catch (Exception e) {
      if (debug) {
        TikaJson.debugLog("Extraction failed", e.getClass().getName());
        e.printStackTrace(System.err);
      }
      throw e;
    }

    double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
    String json = TikaJson.toJson(data, elapsedMs, ocrEnabled);
    System.out.print(json);
  }

  private static void processBatchMode(List<String> positionalArgs,
                                       boolean ocrEnabled, String ocrLanguage,
                                       boolean debug) throws Exception {
    List<String> filePaths = new ArrayList<>();
    for (int i = 1; i < positionalArgs.size(); i++) {
      filePaths.add(positionalArgs.get(i));
    }

    long batchStart = System.nanoTime();
    StringBuilder jsonArray = new StringBuilder();
    jsonArray.append('[');

    boolean first = true;
    for (String filePath : filePaths) {
      if (debug) {
        TikaJson.debugLog("Processing file", filePath);
      }

      try {
        Path path = Path.of(filePath);
        long start = System.nanoTime();
        ExtractionData data =
            extractFile(path.toFile(), ocrEnabled, ocrLanguage);
        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

        if (!first) {
          jsonArray.append(',');
        }
        first = false;

        double batchTotalMs =
            (System.nanoTime() - batchStart) / NANOS_IN_MILLISECOND;
        jsonArray.append(TikaJson.toJsonWithBatch(data, elapsedMs, batchTotalMs,
                                                  ocrEnabled));

        if (debug) {
          TikaJson.debugLog("File processed", filePath);
        }
      } catch (Exception e) {
        if (debug) {
          TikaJson.debugLog("Failed to process file", filePath);
          TikaJson.debugLog("Exception", e.getClass().getName());
          e.printStackTrace(System.err);
        } else {
          System.err.printf("Error processing %s: %s%n", filePath,
                            e.getMessage());
        }
      }
    }

    jsonArray.append(']');

    if (first) {
      System.err.println("No files were successfully processed");
      System.exit(1);
      return;
    }

    System.out.print(jsonArray.toString());
  }

  private static void processServerMode(boolean ocrEnabled, String ocrLanguage)
      throws Exception {
    AutoDetectParser sharedParser = new AutoDetectParser();
    TesseractOCRConfig sharedOcrConfig = new TesseractOCRConfig();
    if (!ocrEnabled) {
      sharedOcrConfig.setSkipOcr(true);
    } else {
      sharedOcrConfig.setLanguage(ocrLanguage);
    }

    System.out.println("READY");
    System.out.flush();

    try (BufferedReader reader =
             new BufferedReader(new InputStreamReader(System.in))) {
      String line;
      while ((line = reader.readLine()) != null) {
        String filePath = line.trim();
        if (filePath.isEmpty()) {
          continue;
        }
        if (filePath.startsWith("{")) {
          filePath = TikaJson.parseJsonPath(filePath);
        }
        try {
          Path path = Path.of(filePath);
          long start = System.nanoTime();
          ExtractionData data = extractFileWithParser(
              path.toFile(), sharedParser, sharedOcrConfig);
          double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
          String json = TikaJson.toJson(data, elapsedMs, ocrEnabled);
          System.out.println(json);
          System.out.flush();
        } catch (Exception e) {
          String errorJson = String.format(
              "{\"error\":%s,\"_extraction_time_ms\":0,\"_ocr_used\":false}",
              TikaJson.quote(e.getMessage()));
          System.out.println(errorJson);
          System.out.flush();
        }
      }
    }
  }

  private static ExtractionData
  extractFileWithParser(File file, AutoDetectParser parser,
                        TesseractOCRConfig ocrConfig) throws Exception {
    if (!file.exists()) {
      throw new IllegalArgumentException("File does not exist: " +
                                         file.getAbsolutePath());
    }

    BodyContentHandler handler = new BodyContentHandler(-1);
    Metadata metadata = new Metadata();
    ParseContext context = new ParseContext();
    context.set(TesseractOCRConfig.class, ocrConfig);

    try (InputStream stream = new FileInputStream(file)) {
      parser.parse(stream, handler, metadata, context);
    }

    String content = handler.toString();
    String mimeType = metadata.get(Metadata.CONTENT_TYPE);

    if (mimeType == null) {
      mimeType = "application/octet-stream";
    }

    return new ExtractionData(content, mimeType);
  }

  private static ExtractionData extractFile(File file, boolean ocrEnabled,
                                            String ocrLanguage)
      throws Exception {
    if (!file.exists()) {
      throw new IllegalArgumentException("File does not exist: " +
                                         file.getAbsolutePath());
    }

    AutoDetectParser parser = new AutoDetectParser();
    BodyContentHandler handler = new BodyContentHandler(-1);
    Metadata metadata = new Metadata();
    ParseContext context = new ParseContext();

    if (!ocrEnabled) {
      TesseractOCRConfig ocrConfig = new TesseractOCRConfig();
      ocrConfig.setSkipOcr(true);
      context.set(TesseractOCRConfig.class, ocrConfig);
    } else {
      TesseractOCRConfig ocrConfig = new TesseractOCRConfig();
      ocrConfig.setLanguage(ocrLanguage);
      context.set(TesseractOCRConfig.class, ocrConfig);
    }

    try (InputStream stream = new FileInputStream(file)) {
      parser.parse(stream, handler, metadata, context);
    }

    String content = handler.toString();
    String mimeType = metadata.get(Metadata.CONTENT_TYPE);

    if (mimeType == null) {
      mimeType = "application/octet-stream";
    }

    return new ExtractionData(content, mimeType);
  }
}

/**
 * Result of a single Tika extraction: the extracted text plus its detected MIME
 * type.
 *
 * <p>Top-level (not nested in {@link TikaExtract}) so it, and the JSON/logging
 * helpers in
 * {@link TikaJson} that consume it, don't count toward {@code TikaExtract}'s
 * own line budget.
 * {@code TikaExtract.java} is compiled via {@code javac -d <dir>
 * TikaExtract.java} with no
 * {@code -sourcepath}, so this class must live in the same source file rather
 * than a sibling one javac would need to discover on its own. ~keep
 */
final class ExtractionData {
  private final String content;
  private final String mimeType;

  ExtractionData(String content, String mimeType) {
    this.content = content;
    this.mimeType = mimeType;
  }

  String getContent() { return content; }

  String getMimeType() { return mimeType; }
}

/**
 * JSON rendering, debug logging, and the request-line parsing used by {@link
 * TikaExtract}.
 */
final class TikaJson {
  /** Length of the JSON key {@code "path"} including surrounding quotes. */
  private static final int PATH_KEY_LENGTH = 6;

  private static final char LAST_CONTROL_CHAR = 0x1F;

  private TikaJson() {}

  /**
   * Determine if OCR was actually used based on MIME type and OCR config.
   * OCR is used by Tika when enabled and the file is an image type.
   */
  private static boolean determineOcrUsed(String mimeType, boolean ocrEnabled) {
    if (!ocrEnabled) {
      return false;
    }
    return mimeType != null && mimeType.startsWith("image/");
  }

  static String toJson(ExtractionData data, double elapsedMs,
                       boolean ocrEnabled) {
    StringBuilder builder = new StringBuilder();
    builder.append('{');
    builder.append("\"content\":").append(quote(data.getContent())).append(',');
    builder.append("\"metadata\":{");
    builder.append("\"mimeType\":").append(quote(data.getMimeType()));
    builder.append("},\"_extraction_time_ms\":")
        .append(String.format("%.3f", elapsedMs));
    builder.append(",\"_ocr_used\":")
        .append(determineOcrUsed(data.getMimeType(), ocrEnabled));
    builder.append('}');
    return builder.toString();
  }

  static String toJsonWithBatch(ExtractionData data, double elapsedMs,
                                double batchTotalMs, boolean ocrEnabled) {
    StringBuilder builder = new StringBuilder();
    builder.append('{');
    builder.append("\"content\":").append(quote(data.getContent())).append(',');
    builder.append("\"metadata\":{");
    builder.append("\"mimeType\":").append(quote(data.getMimeType()));
    builder.append("},\"_extraction_time_ms\":")
        .append(String.format("%.3f", elapsedMs));
    builder.append(",\"_batch_total_ms\":")
        .append(String.format("%.3f", batchTotalMs));
    builder.append(",\"_ocr_used\":")
        .append(determineOcrUsed(data.getMimeType(), ocrEnabled));
    builder.append('}');
    return builder.toString();
  }

  /**
   * Parse a JSON request line to extract the "path" field.
   * Minimal JSON parsing to avoid adding a dependency.
   */
  static String parseJsonPath(String json) {
    int idx = json.indexOf("\"path\"");
    if (idx < 0) {
      return json;
    }
    idx = json.indexOf(':', idx + PATH_KEY_LENGTH);
    if (idx < 0) {
      return json;
    }
    idx = json.indexOf('"', idx + 1);
    if (idx < 0) {
      return json;
    }
    int start = idx + 1;
    int end = json.indexOf('"', start);
    if (end < 0) {
      return json;
    }
    return json.substring(start, end);
  }

  static String quote(String value) {
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
          sb.append(String.format("\\u%04x", (int)c));
        } else {
          sb.append(c);
        }
      }
    }
    sb.append('"');
    return sb.toString();
  }

  static void debugLog(String key, String value) {
    if (value == null) {
      value = "(null)";
    }
    System.err.printf("[BENCHMARK_DEBUG] %-30s = %s%n", key, value);
  }
}
