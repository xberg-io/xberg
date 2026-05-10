```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import java.nio.file.Paths;

ExtractionConfig config = ExtractionConfig.builder().build();
ExtractionResult result = Kreuzberg.extractFileSync(Paths.get("document.pdf"), config);

System.out.println(result.content());
System.out.println("Tables: " + (result.tables() != null ? result.tables().size() : 0));
System.out.println("Metadata: " + result.metadata());
```
