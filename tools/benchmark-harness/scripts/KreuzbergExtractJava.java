import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class KreuzbergExtractJava {
    private static final double NANOS_IN_MILLISECOND = 1_000_000.0;
    private static final int WARMUP_ITERATIONS = 10;

    private KreuzbergExtractJava() { }

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
            System.err.println("Usage: KreuzbergExtractJava [--ocr|--no-ocr] <mode> <file_path> [additional_files...]");
            System.err.println("Modes: sync, warmup, batch, server");
            System.exit(1);
        }

        String mode = positionalArgs.get(0);
        if (!"sync".equals(mode) && !"warmup".equals(mode) && !"batch".equals(mode) && !"server".equals(mode)) {
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

    private static void handleWarmupMode(List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
        if (positionalArgs.size() < 2) {
            System.err.println("Usage: KreuzbergExtractJava warmup <file_path>");
            System.exit(1);
        }

        if (debug) {
            debugLog("Warmup phase starting", "");
        }

        Path path = Path.of(positionalArgs.get(1));
        try {
            for (int i = 0; i < WARMUP_ITERATIONS; i++) {
                Kreuzberg.extractFile(path, null);
                if (debug && i % 2 == 0) {
                    debugLog("Warmup iteration", String.valueOf(i + 1));
                }
            }
            if (debug) {
                debugLog("Warmup phase complete", String.valueOf(WARMUP_ITERATIONS) + " iterations");
            }
            System.out.println("{\"status\":\"warmup_complete\"}");
        } catch (KreuzbergException | RuntimeException | java.io.IOException e) {
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

        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            String filePath = line.trim();
            if (filePath.isEmpty()) {
                continue;
            }
            try {
                Path path = Path.of(filePath);
                long start = System.nanoTime();
                ExtractionResult result = Kreuzberg.extractFile(path, null);
                double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;
                String json = toJson(result, elapsedMs);
                System.out.println(json);
                System.out.flush();
            } catch (Exception e) {
                String errorJson = String.format("{\"error\":%s,\"_extraction_time_ms\":0}", quote(e.getMessage()));
                System.out.println(errorJson);
                System.out.flush();
            }
        }
    }

    private static void handleBatchMode(List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
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

        long start = System.nanoTime();
        try {
            List<ExtractionResult> results = new ArrayList<>();
            for (Path path : paths) {
                results.add(Kreuzberg.extractFile(path, null));
            }
            double totalMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

            if (debug) {
                debugLog("Batch extraction completed", String.valueOf(results.size()) + " results");
            }

            double perFileMs = totalMs / Math.max(results.size(), 1);

            if (results.size() == 1) {
                String json = toJsonWithBatch(results.get(0), perFileMs, totalMs);
                System.out.print(json);
            } else {
                System.out.print("[");
                for (int i = 0; i < results.size(); i++) {
                    if (i > 0) System.out.print(",");
                    System.out.print(toJsonWithBatch(results.get(i), perFileMs, totalMs));
                }
                System.out.print("]");
            }
        } catch (KreuzbergException | RuntimeException | java.io.IOException e) {
            if (debug) {
                debugLog("Batch extraction failed", e.getClass().getName());
                e.printStackTrace(System.err);
            }
            System.exit(1);
        }
    }

    private static void handleSyncMode(List<String> positionalArgs, boolean ocrEnabled, boolean debug) {
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
        long start = System.nanoTime();
        try {
            if (debug) {
                debugLog("Starting extraction", "");
            }
            result = Kreuzberg.extractFile(path, null);
            if (debug) {
                debugLog("Extraction completed", "");
            }
        } catch (KreuzbergException | RuntimeException | java.io.IOException e) {
            if (debug) {
                debugLog("Extraction failed with exception", e.getClass().getName());
                e.printStackTrace(System.err);
            } else {
                e.printStackTrace(System.err);
            }
            System.exit(1);
            return;
        }
        double elapsedMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

        String json = toJson(result, elapsedMs);
        System.out.print(json);
    }

    private static String toJson(ExtractionResult result, double elapsedMs) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(result.getContent())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(result.getMimeType())).append(',');
        builder.append("\"language\":").append(optionalToJson(result.getLanguage())).append(',');
        builder.append("\"date\":").append(optionalToJson(result.getDate())).append(',');
        builder.append("\"subject\":").append(optionalToJson(result.getSubject()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", elapsedMs));
        builder.append('}');
        return builder.toString();
    }

    private static String toJsonWithBatch(ExtractionResult result, double perFileMs, double batchTotalMs) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        builder.append("\"content\":").append(quote(result.getContent())).append(',');
        builder.append("\"metadata\":{");
        builder.append("\"mimeType\":").append(quote(result.getMimeType()));
        builder.append("},\"_extraction_time_ms\":").append(String.format("%.3f", perFileMs));
        builder.append(",\"_batch_total_ms\":").append(String.format("%.3f", batchTotalMs));
        builder.append('}');
        return builder.toString();
    }

    private static String optionalToJson(java.util.Optional<String> value) {
        return value.isPresent() ? quote(value.get()) : "null";
    }

    private static String quote(String value) {
        if (value == null) {
            return "null";
        }
        String escaped = value
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r");
        return "\"" + escaped + "\"";
    }

    private static void debugLog(String key, String value) {
        if (value == null) {
            value = "(null)";
        }
        System.err.printf("[BENCHMARK_DEBUG] %-30s = %s%n", key, value);
    }
}
