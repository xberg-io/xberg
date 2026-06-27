```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

ExtractionResult result = Xberg.extract("scanned_document.pdf", config);

double qualityScore = result.getQualityScore() != null ? result.getQualityScore() : 0.0;

if (qualityScore < 0.5) {
    System.out.printf("Warning: Low quality extraction (%.2f)%n", qualityScore);
    System.out.println("Consider re-scanning or adjusting OCR settings");
} else {
    System.out.printf("Quality score: %.2f%n", qualityScore);
}
```
