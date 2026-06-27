```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.*;
import java.nio.file.Paths;
import java.util.Optional;

ExtractionConfig config = ExtractionConfig.builder()
    .withOcr(Optional.of(OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguages(Optional.of(java.util.List.of("eng", "deu")))
        .build()))
    .withChunking(Optional.of(ChunkingConfig.builder()
        .withMaxChars(Optional.of(512L))
        .withMaxOverlap(Optional.of(50L))
        .build()))
    .withEnableQualityProcessing(true)
    .build();

ExtractionResult result = Xberg.extractSync(Paths.get("document.pdf"), config);
System.out.println("Content: " + result.content().substring(0, 100) + "...");
if (result.tables() != null) {
    System.out.println("Tables: " + result.tables().size());
}
if (result.qualityScore() != null) {
    System.out.println("Quality: " + result.qualityScore());
}
```
