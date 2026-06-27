```java title="SimpleBenchmark.java"
import io.xberg.ExtractInput;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;
import io.xberg.Xberg;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.Callable;
import java.util.concurrent.ForkJoinPool;

public final class SimpleBenchmark {
  private SimpleBenchmark() {}

  public static void main(String[] args) throws Exception {
    ExtractionConfig config = ExtractionConfig.builder()
      .withUseCache(false)
      .build();

    String filePath = "document.pdf";
    ExtractInput input = ExtractInput.fromUri(filePath);
    int numRuns = 10;

    System.out.println("Sequential extraction (" + numRuns + " runs):");
    long start = System.nanoTime();
    for (int i = 0; i < numRuns; i++) {
      Xberg.extract(input, config);
    }
    double sequentialDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    double avgSequential = sequentialDuration / numRuns;
    System.out.println("  - Total time: " + String.format("%.3f", sequentialDuration) + "s");
    System.out.println("  - Average: " + String.format("%.3f", avgSequential) + "s per extraction");

    System.out.println("\nParallel extraction (" + numRuns + " runs):");
    List<Callable<ExtractionResult>> tasks = new ArrayList<>();
    for (int i = 0; i < numRuns; i++) {
      tasks.add(() -> Xberg.extract(input, config));
    }

    start = System.nanoTime();
    ForkJoinPool.commonPool().invokeAll(tasks);
    double parallelDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Total time: " + String.format("%.3f", parallelDuration) + "s");
    System.out.println("  - Average: " + String.format("%.3f", parallelDuration / numRuns) + "s per extraction");
    System.out.println("  - Speedup: " + String.format("%.1f", sequentialDuration / parallelDuration) + "x");

    ExtractionConfig cacheConfig = ExtractionConfig.builder()
      .withUseCache(true)
      .build();

    System.out.println("\nFirst extraction (populates cache)...");
    start = System.nanoTime();
    Xberg.extract(input, cacheConfig);
    double firstDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Time: " + String.format("%.3f", firstDuration) + "s");

    System.out.println("Second extraction (from cache)...");
    start = System.nanoTime();
    Xberg.extract(input, cacheConfig);
    double cachedDuration = (System.nanoTime() - start) / 1_000_000_000.0;
    System.out.println("  - Time: " + String.format("%.3f", cachedDuration) + "s");
    System.out.println("  - Cache speedup: " + String.format("%.1f", firstDuration / cachedDuration) + "x");
  }
}
```
