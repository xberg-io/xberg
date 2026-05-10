```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.LanguageDetectionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("multilingual_document.pdf", config);

System.out.println("Detected languages: " + result.getDetectedLanguages());
```
