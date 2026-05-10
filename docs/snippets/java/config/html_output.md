```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.HtmlOutputConfig;
import dev.kreuzberg.HtmlTheme;
import dev.kreuzberg.OutputFormat;
import java.nio.file.Path;
import java.util.Optional;

public class HtmlOutput {
    public static void main(String[] args) throws Exception {
        HtmlOutputConfig htmlOutput = HtmlOutputConfig.builder()
            .withTheme(HtmlTheme.GitHub)
            .withEmbedCss(true)
            .build();

        ExtractionConfig config = ExtractionConfig.builder()
            .withOutputFormat(OutputFormat.Html)
            .withHtmlOutput(Optional.of(htmlOutput))
            .build();

        ExtractionResult result = Kreuzberg.extractFileSync(Path.of("document.pdf"), config);
        System.out.println(result.content()); // HTML with kb-* classes
    }
}
```
