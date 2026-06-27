```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.XbergRsException;
import java.nio.file.Paths;

try {
    ExtractionConfig config = ExtractionConfig.builder().build();
    ExtractionResult result = Xberg.extractSync(Paths.get("missing.pdf"), config);
    System.out.println(result.content());
} catch (XbergRsException e) {
    System.err.println("Extraction failed: " + e.getMessage());
    System.err.println("Error code: " + e.getCode());
}
```
