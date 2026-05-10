```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import java.nio.file.Files;
import java.nio.file.Paths;

byte[] data = Files.readAllBytes(Paths.get("document.pdf"));
ExtractionConfig config = ExtractionConfig.builder().build();
ExtractionResult result = Kreuzberg.extractBytesSync(data, "application/pdf", config);

System.out.println(result.content());
System.out.println(result.mimeType());
```
