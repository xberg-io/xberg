```java title="Java"
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.EmbeddingConfig;
import dev.kreuzberg.EmbeddingModelType;
import dev.kreuzberg.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1500)
        .maxOverlap(200)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.builder()
                .type("preset")
                .name("text-embedding-all-minilm-l6-v2")
                .build())
            .build())
        .build())
    .build();
```
