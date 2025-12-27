import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class KreuzbergExtractJava {
    private static final double NANOS_IN_MILLISECOND = 1_000_000.0;
    private static final int WARMUP_ITERATIONS = 10;

    private KreuzbergExtractJava() { }

    public static void main(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: KreuzbergExtractJava <mode> <file_path> [additional_files...]");
            System.err.println("Modes: sync, warmup, batch");
            System.exit(1);
        }

        String mode = args[0];
        if (!"sync".equals(mode) && !"warmup".equals(mode) && !"batch".equals(mode)) {
            System.err.printf("Unsupported mode '%s'%n", mode);
            System.exit(1);
        }

        if ("warmup".equals(mode)) {
            handleWarmupMode(args);
            return;
        } else if ("batch".equals(mode)) {
            handleBatchMode(args);
            return;
        }

        // Enable debug logging if KREUZBERG_BENCHMARK_DEBUG is set
        boolean debug = "true".equalsIgnoreCase(System.getenv("KREUZBERG_BENCHMARK_DEBUG"));
        handleSyncMode(args, debug);
    }

    private static void handleWarmupMode(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: KreuzbergExtractJava warmup <file_path>");
            System.exit(1);
        }

        boolean debug = "true".equalsIgnoreCase(System.getenv("KREUZBERG_BENCHMARK_DEBUG"));
        if (debug) {
            debugLog("Warmup phase starting", "");
        }

        Path path = Path.of(args[1]);
        try {
            for (int i = 0; i < WARMUP_ITERATIONS; i++) {
                Kreuzberg.extractFile(path);
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

    private static void handleBatchMode(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: KreuzbergExtractJava batch <file_path> [additional_files...]");
            System.exit(1);
        }

        boolean debug = "true".equalsIgnoreCase(System.getenv("KREUZBERG_BENCHMARK_DEBUG"));
        if (debug) {
            debugLog("Batch mode starting", String.valueOf(args.length - 1) + " files");
        }

        List<Path> paths = new ArrayList<>();
        for (int i = 1; i < args.length; i++) {
            paths.add(Path.of(args[i]));
        }

        long start = System.nanoTime();
        try {
            List<ExtractionResult> results = new ArrayList<>();
            for (Path path : paths) {
                results.add(Kreuzberg.extractFile(path));
            }
            double totalMs = (System.nanoTime() - start) / NANOS_IN_MILLISECOND;

            if (debug) {
                debugLog("Batch extraction completed", String.valueOf(results.size()) + " results");
            }

            if (results.size() == 1) {
                String json = toJson(results.get(0), totalMs / results.size());
                System.out.print(json);
            } else {
                System.out.print("[");
                for (int i = 0; i < results.size(); i++) {
                    if (i > 0) System.out.print(",");
                    System.out.print(toJson(results.get(i), totalMs / results.size()));
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

    private static void handleSyncMode(String[] args, boolean debug) {
        if (args.length < 2) {
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
            debugLog("Input file", args[1]);
        }

        Path path = Path.of(args[1]);
        ExtractionResult result;
        long start = System.nanoTime();
        try {
            if (debug) {
                debugLog("Starting extraction", "");
            }
            result = Kreuzberg.extractFile(path);
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
