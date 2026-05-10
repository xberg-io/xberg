```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.LanguageDetectionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.9)
        .detectMultiple(true)
        .build())
    .build();
```
