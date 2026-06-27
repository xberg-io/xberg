```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .enableQualityProcessing(true)
    .build();
ExtractionResult result = Xberg.extract("document.pdf", config);
```
