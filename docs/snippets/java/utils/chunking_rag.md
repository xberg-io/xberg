```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("research_paper.pdf", config);

System.out.println("Content: " + result.getContent()
    .substring(0, Math.min(100, result.getContent().length())) + "...");
```
