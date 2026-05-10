```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1024)
        .maxOverlap(100)
        .embedding("balanced")
        .build())
    .build();
```
