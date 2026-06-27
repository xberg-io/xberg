```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.XbergRsException;
import java.nio.file.Paths;

try {
    ExtractionConfig config = ExtractionConfig.builder().build();
    var resultOutput = Xberg.extract(
        io.xberg.ExtractInput.builder()
            .withKind(io.xberg.ExtractInputKind.Uri)
            .withUri("missing.pdf")
            .build(),
        config
    );
    ExtractedDocument result = resultOutput.results().get(0);
    System.out.println(result.content());
} catch (XbergRsException e) {
    System.err.println("Extraction failed: " + e.getMessage());
    System.err.println("Error code: " + e.getCode());
}
```
