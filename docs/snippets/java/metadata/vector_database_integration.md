```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Xberg.extract("document.pdf", config);

System.out.println("Extracted content: " + result.getContent().length() + " characters");
```
