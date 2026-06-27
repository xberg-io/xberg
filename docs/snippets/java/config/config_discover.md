```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;

ExtractionConfig config = Xberg.discoverExtractionConfig();
ExtractionResult result = Xberg.extract("document.pdf", config);
```
