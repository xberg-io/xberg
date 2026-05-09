```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.PdfConfig;
import dev.kreuzberg.HierarchyConfig;
import java.util.Arrays;

ExtractionConfig config = ExtractionConfig.builder()
    .pdfOptions(PdfConfig.builder()
        .extractImages(true)
        .extractMetadata(true)
        .passwords(Arrays.asList("password1", "password2"))
        .hierarchyConfig(HierarchyConfig.builder().build())
        .build())
    .build();
```
