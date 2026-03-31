package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import java.util.Optional;

/**
 * A keyword extracted from document content with scoring metadata.
 *
 * <p>
 * Contains the keyword text, a relevance score, the algorithm used for
 * extraction, and optional position indices within the document.
 *
 * @since 4.5.0
 */
public class ExtractedKeyword {
	private final String text;
	private final float score;
	private final String algorithm;
	private final List<Integer> positions;

	@JsonCreator
	public ExtractedKeyword(@JsonProperty("text") String text, @JsonProperty("score") float score,
			@JsonProperty("algorithm") String algorithm, @JsonProperty("positions") List<Integer> positions) {
		this.text = Objects.requireNonNull(text, "text must not be null");
		this.score = score;
		this.algorithm = Objects.requireNonNull(algorithm, "algorithm must not be null");
		if (positions != null) {
			this.positions = Collections.unmodifiableList(new ArrayList<>(positions));
		} else {
			this.positions = null;
		}
	}

	/**
	 * Get the keyword text.
	 *
	 * @return the keyword text
	 */
	public String getText() {
		return text;
	}

	/**
	 * Get the relevance score for this keyword.
	 *
	 * @return the score value
	 */
	public float getScore() {
		return score;
	}

	/**
	 * Get the algorithm used to extract this keyword.
	 *
	 * @return the algorithm name (e.g., "yake", "rake")
	 */
	public String getAlgorithm() {
		return algorithm;
	}

	/**
	 * Get the positions of this keyword within the document.
	 *
	 * @return optional unmodifiable list of position indices, or empty if not
	 *         available
	 */
	public Optional<List<Integer>> getPositions() {
		return Optional.ofNullable(positions);
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof ExtractedKeyword)) {
			return false;
		}
		ExtractedKeyword other = (ExtractedKeyword) obj;
		return Float.compare(score, other.score) == 0 && Objects.equals(text, other.text)
				&& Objects.equals(algorithm, other.algorithm) && Objects.equals(positions, other.positions);
	}

	@Override
	public int hashCode() {
		return Objects.hash(text, score, algorithm, positions);
	}

	@Override
	public String toString() {
		return "ExtractedKeyword{" + "text='" + text + '\'' + ", score=" + score + ", algorithm='" + algorithm + '\''
				+ ", positions=" + positions + '}';
	}
}
