```java title="Java"
import io.xberg.ExtractInput;
import io.xberg.ExtractInputKind;
import io.xberg.ExtractionConfig;
import io.xberg.Xberg;

var input = ExtractInput.builder()
    .withKind(ExtractInputKind.Uri)
    .withUri("document.pdf")
    .build();

var output = Xberg.extract(input, ExtractionConfig.builder().build());
System.out.println(output.results().get(0).content());
```
