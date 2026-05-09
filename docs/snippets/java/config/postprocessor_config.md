```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.PostProcessorConfig;
import java.util.Arrays;

ExtractionConfig config = ExtractionConfig.builder()
    .postprocessor(PostProcessorConfig.builder()
        .enabled(true)
        .enabledProcessors(Arrays.asList("deduplication", "whitespace_normalization"))
        .disabledProcessors(Arrays.asList("mojibake_fix"))
        .build())
    .build();
```
