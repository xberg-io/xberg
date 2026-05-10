```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import java.nio.file.Paths;

ExtractionConfig config = ExtractionConfig.builder().build();
ExtractionResult result = Kreuzberg.extractFile(Paths.get("document.pdf"), config);

System.out.println(result.content());
System.out.println(result.mimeType());
```
