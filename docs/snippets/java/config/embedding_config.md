```java title="Java"
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.EmbeddingConfig;
import dev.kreuzberg.EmbeddingModelType;
import dev.kreuzberg.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.builder()
                .type("preset")
                .name("all-mpnet-base-v2")
                .build())
            .batchSize(16)
            .normalize(true)
            .showDownloadProgress(true)
            .build())
        .build())
    .build();
```
