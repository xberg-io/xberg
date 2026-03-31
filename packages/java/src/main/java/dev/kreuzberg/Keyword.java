package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A keyword extracted from document content with scoring metadata.
 *
 * <p>
 * This class provides parity with other language bindings that use the
 * shorter name "Keyword". It delegates all behavior to {@link ExtractedKeyword}.
 *
 * @since 4.6.0
 * @see ExtractedKeyword
 */
public final class Keyword extends ExtractedKeyword {

	@JsonCreator
	public Keyword(@JsonProperty("text") String text, @JsonProperty("score") float score,
			@JsonProperty("algorithm") String algorithm, @JsonProperty("positions") List<Integer> positions) {
		super(text, score, algorithm, positions);
	}
}
