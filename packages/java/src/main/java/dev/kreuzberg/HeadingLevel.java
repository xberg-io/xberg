package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Objects;

/**
 * A single heading in the document hierarchy.
 */
public final class HeadingLevel {
	private final int level;
	private final String text;

	@JsonCreator
	public HeadingLevel(@JsonProperty("level") int level, @JsonProperty("text") String text) {
		this.level = level;
		this.text = Objects.requireNonNull(text, "text must not be null");
	}

	/**
	 * Get the heading depth (1 = h1, 2 = h2, etc.).
	 *
	 * @return heading depth
	 */
	public int getLevel() {
		return level;
	}

	/**
	 * Get the text content of the heading.
	 *
	 * @return heading text
	 */
	public String getText() {
		return text;
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof HeadingLevel)) {
			return false;
		}
		HeadingLevel other = (HeadingLevel) obj;
		return level == other.level && Objects.equals(text, other.text);
	}

	@Override
	public int hashCode() {
		return Objects.hash(level, text);
	}

	@Override
	public String toString() {
		return "HeadingLevel{level=" + level + ", text='" + text + "'}";
	}
}
