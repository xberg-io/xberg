```java title="Document Structure Config (Java)"
import io.xberg.Xberg;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;

ExtractionConfig config = ExtractionConfig.builder()
    .includeDocumentStructure(true)
    .build();

ExtractionResult result = Xberg.extractSync("document.pdf", config);

if (result.getDocumentStructure().isPresent()) {
    var document = result.getDocumentStructure().get();
    for (var node : document.nodes()) {
        System.out.println("[" + node.content().nodeType() + "]");
    }
}
```
