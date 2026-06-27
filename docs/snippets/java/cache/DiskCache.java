```java title="DiskCache.java"
import io.xberg.*;
import java.nio.file.Files;
import java.nio.file.Paths;

public final class DiskCache {
  private DiskCache() {}

  public static void main(String[] args) throws Exception {
    String cacheDir = System.getProperty("user.home") + "/.cache/xberg";
    Files.createDirectories(Paths.get(cacheDir));

    CacheConfig cacheConfig = new CacheConfig(
      cacheDir,
      500L * 1024 * 1024,
      7L * 86400,
      true
    );

    ExtractionConfig config = ExtractionConfig.builder()
      .useCache(true)
      .cacheConfig(cacheConfig)
      .build();

    System.out.println("First extraction (will be cached)...");
    ExtractionResult output1 = Xberg.extract(
      ExtractInput.fromUri("document.pdf"),
      config
    );
    ExtractedDocument result1 = output1.results().get(0);
    System.out.println("  - Content length: " + result1.content().length());
    System.out.println("  - Cached: " + result1.metadata().wasCached());

    System.out.println("\nSecond extraction (from cache)...");
    ExtractionResult output2 = Xberg.extract(
      ExtractInput.fromUri("document.pdf"),
      config
    );
    ExtractedDocument result2 = output2.results().get(0);
    System.out.println("  - Content length: " + result2.content().length());
    System.out.println("  - Cached: " + result2.metadata().wasCached());

    System.out.println("\nResults are identical: " + result1.content().equals(result2.content()));

    CacheStats cacheStats = Xberg.getCacheStats();
    System.out.println("\nCache Statistics:");
    System.out.println("  - Total entries: " + cacheStats.totalEntries());
    System.out.println("  - Cache size: " + String.format("%.1f", cacheStats.cacheSizeBytes() / 1024.0 / 1024.0) + " MB");
    System.out.println("  - Hit rate: " + String.format("%.1f", cacheStats.hitRate() * 100) + "%");
  }
}
```
