import io.xberg.ExtractInput;
import io.xberg.ExtractInputKind;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;
import io.xberg.KeywordAlgorithm;
import io.xberg.KeywordConfig;
import io.xberg.RakeParams;
import io.xberg.Xberg;
import io.xberg.YakeParams;

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
public class KeywordConfigExample {

    public static void basicYake() throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .withKeywords(KeywordConfig.builder()
                .withAlgorithm(KeywordAlgorithm.Yake)
                .withMaxKeywords(10L)
                .withMinScore(0.0f)
                .withLanguage("en")
                .withYakeParams(null)
                .withRakeParams(null)
                .build())
            .build();

        ExtractionResult output = Xberg.extract(input("document.pdf"), config);
        ExtractedDocument result = output.results().get(0);
        System.out.println("Keywords: " + result.extractedKeywords());
    }

    // Example 2: Advanced YAKE with custom parameters
    // Fine-tunes YAKE with custom window size for co-occurrence analysis
    public static void advancedYake() throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .withKeywords(KeywordConfig.builder()
                .withAlgorithm(KeywordAlgorithm.Yake)
                .withMaxKeywords(15L)
                .withMinScore(0.1f)
                .withLanguage("en")
                .withYakeParams(YakeParams.builder()
                    .withWindowSize(1)
                    .build())
                .withRakeParams(null)
                .build())
            .build();

        ExtractionResult output = Xberg.extract(input("document.pdf"), config);
        ExtractedDocument result = output.results().get(0);
        System.out.println("Keywords: " + result.extractedKeywords());
    }

    // Example 3: RAKE configuration
    // Uses RAKE algorithm for rapid keyword extraction with phrase constraints
    public static void rakeConfig() throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .withKeywords(KeywordConfig.builder()
                .withAlgorithm(KeywordAlgorithm.Rake)
                .withMaxKeywords(10L)
                .withMinScore(5.0f)
                .withLanguage("en")
                .withYakeParams(null)
                .withRakeParams(RakeParams.builder()
                    .withMinWordLength(1)
                    .withMaxWordsPerPhrase(3)
                    .build())
                .build())
            .build();

        ExtractionResult output = Xberg.extract(input("document.pdf"), config);
        ExtractedDocument result = output.results().get(0);
        System.out.println("Keywords: " + result.extractedKeywords());
    }

    public static void main(String[] args) throws Exception {
        basicYake();
    }

    private static ExtractInput input(String uri) {
        return ExtractInput.builder()
            .withKind(ExtractInputKind.Uri)
            .withUri(uri)
            .build();
    }
}
