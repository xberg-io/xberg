```java title="Java"
import io.xberg.ExtractionConfig;
import io.xberg.KeywordAlgorithm;
import io.xberg.KeywordConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .withKeywords(KeywordConfig.builder()
        .withAlgorithm(KeywordAlgorithm.Yake)
        .withMaxKeywords(10L)
        .withMinScore(0.3f)
        .withLanguage("en")
        .build())
    .build();
```
