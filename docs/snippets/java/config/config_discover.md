```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractInput;

ExtractionConfig config = Xberg.discoverExtractionConfig();
ExtractionResult output = Xberg.extract(
    ExtractInput.fromUri("document.pdf"),
    config
);
ExtractedDocument result = output.results().get(0);
```
