import org.apache.tika.parser.AutoDetectParser;
import org.apache.tika.parser.ParseContext;
import org.apache.tika.parser.ocr.TesseractOCRConfig;
import org.apache.tika.sax.BodyContentHandler;
import org.apache.tika.metadata.Metadata;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class TikaExtract {
    private static final double NANOS_IN_MILLISECOND = 1_000_000.0;

    private TikaExtract() {
    }

    public static void main(String[] args) {
        boolean ocrEnabled = false;
        List<String> positionalArgs = new ArrayList<>();

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
            System.err.println("Usage: TikaExtract [--ocr|--no-ocr] <mode> <file1> [file2] ...");
            System.err.println("Modes: sync, batch, server");
            System.exit(1);
        }

        String mode = positionalArgs.get(0);
        if (!"sync".equals(mode) && !"batch".equals(mode) && !"server".equals(mode)) {
            System.err.printf("Unsupported mode '%s'%n", mode);
            System.exit(1);
        }

        // Enable debug logging if TIKA_BENCHMARK_DEBUG is set
        boolean debug = "true".equalsIgnoreCase(System.getenv("TIKA_BENCHMARK_DEBUG"));

        if (debug) {
            debugLog("java.version", System.getProperty("java.version"));
            debugLog("os.name", System.getProperty("os.name"));
            debugLog("os.arch", System.getProperty("os.arch"));
            debugLog("Mode", mode);
            debugLog("OCR enabled", String.valueOf(ocrEnabled));
            debugLog("Files to process", String.valueOf(positionalArgs.size() - 1));
        }

        try {
            if ("sync".equals(mode)) {
                if (positionalArgs.size() < 2) {
                    System.err.println("Sync mode requires exactly one file");
                    System.exit(1);
                }
                processSyncMode(positionalArgs.get(1), ocrEnabled, debug);
            } else if ("batch".equals(mode)) {
                processBatchMode(positionalArgs, ocrEnabled, debug);
            } else {
                processServerMode(ocrEnabled, debug);
            }
        } catch (Exception e) {
            if (debug) {
                debugLog("Processing failed with exception", e.getClass().getName());
                e.printStackTrace(System.err);
            } else {
                e.printStackTrace(System.err);
            }
            System.exit(1);
        }
    }

    private static void processSyncMode(String filePath, boolean ocrEnabled, boolean debug) throws Exception {
        if (debug) {
            debugLog("Input file", filePath);
        }

        Path path = Path.of(filePath);
        ExtractionData data;
        long start = System.nanoTime();

        try {
            if (debug) {
                debugLog("Starting extraction", "");
            }
            data = extractFile(path.toFile(), ocrEnabled, debug);
            if (debug) {
                debugLog("Extraction completed", "");
            }
        } catch (Exception e) {
            if (debug) {
                debugLog("Extraction failed", e.getClass().getName());
                e.printStackTrace(System.err);
            }
            throw e;
        }

        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
        String json = toJson(data, elapsedMs, ocrEnabled);
        System.out.print(json);
    }

    private static void processBatchMode(List<String> positionalArgs, boolean ocrEnabled, boolean debug) throws Exception {
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
                debugLog("Processing file", filePath);
            }

            try {
                Path path = Path.of(filePath);
                long start = System.nanoTime();
                ExtractionData data = extractFile(path.toFile(), ocrEnabled, debug);
                double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

                if (!first) {
                    jsonArray.append(',');
                }
                first = false;

                double batchTotalMs = (System.nanoTime() - batchStart) / NANOS_IN_MILLISECOND;
                jsonArray.append(toJsonWithBatch(data, elapsedMs, batchTotalMs, ocrEnabled));

                if (debug) {
                    debugLog("File processed", filePath);
                }
            } catch (Exception e) {
                if (debug) {
                    debugLog("Failed to process file", filePath);
                    debugLog("Exception", e.getClass().getName());
                    e.printStackTrace(System.err);
                } else {
                    System.err.printf("Error processing %s: %s%n", filePath, e.getMessage());
                }
            }
        }

        double totalBatchMs = (System.nanoTime() - batchStart) / NANOS_IN_MILLISECOND;
        jsonArray.append(']');

        if (first) {
            System.err.println("No files were successfully processed");
            System.exit(1);
            return;
        }

        System.out.print(jsonArray.toString());
    }

    private static void processServerMode(boolean ocrEnabled, boolean debug) throws Exception {
        // Pre-create shared parser and OCR config to avoid per-file construction overhead.
        // AutoDetectParser is thread-safe and reusable. Only BodyContentHandler and Metadata
        // need to be recreated per extraction since they accumulate state.
        AutoDetectParser sharedParser = new AutoDetectParser();
        TesseractOCRConfig sharedOcrConfig = new TesseractOCRConfig();
        if (!ocrEnabled) {
            sharedOcrConfig.setSkipOcr(true);
        } else {
            sharedOcrConfig.setLanguage("eng");
        }

        // Signal readiness after JVM + Tika parser initialization
        System.out.println("READY");
        System.out.flush();

        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            String filePath = line.trim();
            if (filePath.isEmpty()) {
                continue;
            }
            // Parse JSON request if the harness sends {"path":"...", "force_ocr": ...}
            if (filePath.startsWith("{")) {
                filePath = parseJsonPath(filePath);
            }
            try {
                Path path = Path.of(filePath);
                long start = System.nanoTime();
                ExtractionData data = extractFileWithParser(path.toFile(), sharedParser, sharedOcrConfig, debug);
                double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
                String json = toJson(data, elapsedMs, ocrEnabled);
                System.out.println(json);
                System.out.flush();
            } catch (Exception e) {
                String errorJson = String.format("{\"error\":%s,\"_extraction_time_ms\":0,\"_ocr_used\":false}", quote(e.getMessage()));
                System.out.println(errorJson);
                System.out.flush();
            }
        }
    }

    private static ExtractionData extractFileWithParser(
            File file, AutoDetectParser parser, TesseractOCRConfig ocrConfig, boolean debug) throws Exception {
        if (!file.exists()) {
            throw new IllegalArgumentException("File does not exist: " + file.getAbsolutePath());
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

    private static ExtractionData extractFile(File file, boolean ocrEnabled, boolean debug) throws Exception {
        if (!file.exists()) {
            throw new IllegalArgumentException("File does not exist: " + file.getAbsolutePath());
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
            ocrConfig.setLanguage("eng");
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

    private static String toJson(ExtractionData data, double elapsedMs, boolean ocrEnabled) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(data.getContent())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(data.getMimeType()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
        builder.append(",\"_ocr_used\":").append(determineOcrUsed(data.getMimeType(), ocrEnabled));
        builder.append('}');
        return builder.toString();
    }

    private static String toJsonWithBatch(ExtractionData data, double elapsedMs, double batchTotalMs, boolean ocrEnabled) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(data.getContent())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(data.getMimeType()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
        builder.append(",\"_batch_total_ms\":").append(String.format("%.3f", batchTotalMs));
        builder.append(",\"_ocr_used\":").append(determineOcrUsed(data.getMimeType(), ocrEnabled));
        builder.append('}');
        return builder.toString();
    }

    /**
     * Parse a JSON request line to extract the "path" field.
     * Minimal JSON parsing to avoid adding a dependency.
     */
    private static String parseJsonPath(String json) {
        int idx = json.indexOf("\"path\"");
        if (idx < 0) {
            return json;
        }
        // Skip past "path" key, colon, optional whitespace, and opening quote
        idx = json.indexOf(':', idx + 6);
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

    private static String quote(String value) {
        if (value == null) {
            return "null";
        }
        StringBuilder sb = new StringBuilder(value.length() + 2);
        sb.append('"');
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            switch (c) {
                case '\\': sb.append("\\\\"); break;
                case '"':  sb.append("\\\""); break;
                case '\n': sb.append("\\n");  break;
                case '\r': sb.append("\\r");  break;
                case '\t': sb.append("\\t");  break;
                case '\b': sb.append("\\b");  break;
                case '\f': sb.append("\\f");  break;
                default:
                    if (c < 0x20) {
                        sb.append(String.format("\\u%04x", (int) c));
                    } else {
                        sb.append(c);
                    }
            }
        }
        sb.append('"');
        return sb.toString();
    }

    private static void debugLog(String key, String value) {
        if (value == null) {
            value = "(null)";
        }
        System.err.printf("[BENCHMARK_DEBUG] %-30s = %s%n", key, value);
    }

    private static class ExtractionData {
        private final String content;
        private final String mimeType;

        ExtractionData(String content, String mimeType) {
            this.content = content;
            this.mimeType = mimeType;
        }

        String getContent() {
            return content;
        }

        String getMimeType() {
            return mimeType;
        }
    }
}
