```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import java.nio.file.Path;

public final class ConfigFileExample {
    public static void main(String[] args) throws Exception {
        ExtractionConfig config = Xberg.loadExtractionConfigFromFile(Path.of("xberg.toml"));
        ExtractionResult result = Xberg.extract(Path.of("document.pdf"), config);
        System.out.printf("Detected MIME: %s%n", result.getMimeType());
    }
}
```
