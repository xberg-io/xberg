```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.EmbeddingConfig;
import dev.kreuzberg.EmbeddingModelType;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.preset("all-minilm-l6-v2"))
            .normalize(true)
            .batchSize(32)
            .build())
        .build())
    .build();
```
