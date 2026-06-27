```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractInput;
import io.xberg.ExtractInputKind;
import io.xberg.Keyword;
import io.xberg.KeywordConfig;
import io.xberg.KeywordAlgorithm;

ExtractionConfig config = ExtractionConfig.builder()
    .withKeywords(KeywordConfig.builder()
        .withAlgorithm(KeywordAlgorithm.Yake)
        .withMaxKeywords(10L)
        .withMinScore(0.3f)
        .build())
    .build();

ExtractInput input = ExtractInput.builder()
    .withKind(ExtractInputKind.Uri)
    .withUri("research_paper.pdf")
    .build();

ExtractionResult output = Xberg.extract(input, config);
ExtractedDocument result = output.results().get(0);

if (result.extractedKeywords() != null) {
    for (Keyword keyword : result.extractedKeywords()) {
        System.out.println(keyword.text() + ": " + String.format("%.3f", keyword.score()));
    }
}
```
