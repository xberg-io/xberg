```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ImageExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .imageExtraction(ImageExtractionConfig.builder()
        .extractImages(true)
        .targetDpi(200)
        .maxImageDimension(2048)
        .injectPlaceholders(true) // set to false to extract images without markdown references
        .autoAdjustDpi(true)
        .build())
    .build();
```
