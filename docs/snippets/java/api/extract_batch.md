```java title="Java"
import io.xberg.ExtractInput;
import io.xberg.ExtractInputKind;
import io.xberg.ExtractionConfig;
import io.xberg.Xberg;
import java.nio.charset.StandardCharsets;
import java.util.List;

var inputs = List.of(
    ExtractInput.builder()
        .withKind(ExtractInputKind.Uri)
        .withUri("document.pdf")
        .build(),
    ExtractInput.builder()
        .withKind(ExtractInputKind.Bytes)
        .withBytes("Hello from memory".getBytes(StandardCharsets.UTF_8))
        .withMimeType("text/plain")
        .withFilename("note.txt")
        .build()
);

var output = Xberg.extractBatch(inputs, ExtractionConfig.builder().build());
for (var result : output.results()) {
    System.out.println(result.content());
}
```
