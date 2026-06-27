```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Xberg.extract("research_paper.pdf", config);

System.out.println("Content: " + result.getContent()
    .substring(0, Math.min(100, result.getContent().length())) + "...");
```
