```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.LanguageDetectionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())
    .build();

ExtractionResult result = Xberg.extract("multilingual_document.pdf", config);

System.out.println("Detected languages: " + result.getDetectedLanguages());
```
