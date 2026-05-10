```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.LanguageDetectionConfig;
import java.math.BigDecimal;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(new BigDecimal("0.8"))
        .detectMultiple(false)
        .build())
    .build();
```
