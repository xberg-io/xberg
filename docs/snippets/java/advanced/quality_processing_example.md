```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

ExtractionResult result = Xberg.extract("scanned_document.pdf", config);

double qualityScore = result.getQualityScore() != null ? result.getQualityScore() : 0.0;

if (qualityScore < 0.5) {
    System.out.println(String.format("Warning: Low quality extraction (%.2f)", qualityScore));
    System.out.println("Consider re-scanning with higher DPI or adjusting OCR settings");
} else {
    System.out.println(String.format("Quality score: %.2f", qualityScore));
}
```
