```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.TokenReductionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveImportantWords(true)
        .build())
    .build();
```
