```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.BatchFileItem;
import dev.kreuzberg.ExtractionConfig;
import java.nio.file.Paths;
import java.util.List;
import java.util.Arrays;

List<BatchFileItem> items = Arrays.asList(
    new BatchFileItem(Paths.get("doc1.pdf"), null),
    new BatchFileItem(Paths.get("doc2.docx"), null),
    new BatchFileItem(Paths.get("doc3.pptx"), null)
);

ExtractionConfig config = ExtractionConfig.builder().build();
List<ExtractionResult> results = Kreuzberg.batchExtractFilesSync(items, config);

for (ExtractionResult result : results) {
    System.out.println("Content length: " + result.content().length());
}
```
