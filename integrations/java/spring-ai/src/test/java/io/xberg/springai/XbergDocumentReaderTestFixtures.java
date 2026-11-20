package io.xberg.springai;

import static org.mockito.ArgumentMatchers.any;

import io.xberg.ExtractInput;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionResultFactory;
import io.xberg.Xberg;
import io.xberg.XbergRsException;
import java.util.List;
import org.mockito.MockedStatic;

/**
 * Shared {@code XbergDocumentReader} test fixtures: parses a fixture JSON
 * string into an {@link ExtractionResult} and stubs the static {@link Xberg}
 * entry points against it.
 *
 * <p>
 * Split out of {@code XbergDocumentReaderTest} (now several focused test
 * classes in this package) to keep each of those test classes within the repo's
 * type-length budget, without duplicating these helpers or reaching for a
 * shared abstract test base class. ~keep
 */
final class XbergDocumentReaderTestFixtures {

    private XbergDocumentReaderTestFixtures() {}

    static ExtractionResult createResult(String json) {
        return ExtractionResultFactory.fromJson(json);
    }

    /** Stubs the single-input {@code Xberg.extract} entry point. */
    static void mockExtract(MockedStatic<Xberg> mocked, ExtractionResult result)
      throws XbergRsException {
        mocked
        .when(()
                  -> Xberg.extract(any(ExtractInput.class),
                                   any(ExtractionConfig.class)))
        .thenReturn(result);
    }

    /** Stubs the multi-input {@code Xberg.extractBatch} entry point. */
    @SuppressWarnings("unchecked")
    static void mockExtractBatch(MockedStatic<Xberg> mocked,
                               ExtractionResult result)
      throws XbergRsException {
        mocked
        .when(()
                  -> Xberg.extractBatch(any(List.class),
                                        any(ExtractionConfig.class)))
        .thenReturn(result);
    }
}
