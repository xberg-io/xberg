package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * Heading context for a chunk's section in the document.
 *
 * <p>
 * Contains the heading hierarchy from document root to this chunk's section.
 * Index 0 is the outermost (h1), last element is the most specific.
 */
public final class HeadingContext {
	private final List<HeadingLevel> headings;

	@JsonCreator
	public HeadingContext(@JsonProperty("headings") List<HeadingLevel> headings) {
		this.headings = headings != null ? Collections.unmodifiableList(headings) : Collections.emptyList();
	}

	/**
	 * Get the heading hierarchy.
	 *
	 * @return list of heading levels from outermost to most specific
	 */
	public List<HeadingLevel> getHeadings() {
		return headings;
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof HeadingContext)) {
			return false;
		}
		HeadingContext other = (HeadingContext) obj;
		return Objects.equals(headings, other.headings);
	}

	@Override
	public int hashCode() {
		return Objects.hash(headings);
	}

	@Override
	public String toString() {
		return "HeadingContext{headings=" + headings + "}";
	}
}
