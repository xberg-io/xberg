```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

ExtractionResult result = Kreuzberg.extractFile("scanned_document.pdf", config);

double qualityScore = result.getQualityScore() != null ? result.getQualityScore() : 0.0;

if (qualityScore < 0.5) {
    System.out.println(String.format("Warning: Low quality extraction (%.2f)", qualityScore));
    System.out.println("Consider re-scanning with higher DPI or adjusting OCR settings");
} else {
    System.out.println(String.format("Quality score: %.2f", qualityScore));
}
```
