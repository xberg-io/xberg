```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.KreuzbergRsException;
import java.nio.file.Paths;

try {
    ExtractionConfig config = ExtractionConfig.builder().build();
    ExtractionResult result = Kreuzberg.extractFileSync(Paths.get("missing.pdf"), config);
    System.out.println(result.content());
} catch (KreuzbergRsException e) {
    System.err.println("Extraction failed: " + e.getMessage());
    System.err.println("Error code: " + e.getCode());
}
```
