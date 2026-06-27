```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractInput;
import io.xberg.LanguageDetectionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())
    .build();

ExtractionResult output = Xberg.extract(
    ExtractInput.fromUri("multilingual_document.pdf"),
    config
);

ExtractedDocument result = output.results().get(0);

System.out.println("Detected languages: " + result.getDetectedLanguages());
```
