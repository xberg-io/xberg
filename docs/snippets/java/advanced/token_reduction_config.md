```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.TokenReductionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveMarkdown(true)
        .preserveCode(true)
        .languageHint("eng")
        .build())
    .build();
```
