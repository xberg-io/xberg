```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.config.EmbeddingConfig;

// Embed with default config
float[][] embeddings = Kreuzberg.embed(List.of("Hello world", "How are you?"), null);

// Embed with specific preset
EmbeddingConfig config = EmbeddingConfig.withPreset("fast");
float[][] fastEmbeddings = Kreuzberg.embed(List.of("Hello world"), config);

// Async variant
CompletableFuture<float[][]> future = Kreuzberg.embedAsync(texts, null);
```
