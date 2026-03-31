package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Content layer classification within a document.
 *
 * <p>
 * Identifies which structural layer of a document a piece of content
 * belongs to, such as the main body, headers, footers, or footnotes.
 *
 * @since 4.6.0
 */
public enum ContentLayer {
	/** Main document body content. */
	BODY("body"),

	/** Page/section header (running header). */
	HEADER("header"),

	/** Page/section footer (running footer). */
	FOOTER("footer"),

	/** Footnote content. */
	FOOTNOTE("footnote");

	private final String wireValue;

	ContentLayer(String wireValue) {
		this.wireValue = wireValue;
	}

	/**
	 * Get the wire format value for this content layer.
	 *
	 * @return wire value used in serialization
	 */
	@JsonValue
	public String wireValue() {
		return wireValue;
	}

	/**
	 * Parse a ContentLayer from its wire value.
	 *
	 * @param wireValue
	 *            the wire format value (snake_case string)
	 * @return the corresponding ContentLayer
	 * @throws IllegalArgumentException
	 *             if the value is not recognized
	 */
	public static ContentLayer fromWireValue(String wireValue) {
		for (ContentLayer type : values()) {
			if (type.wireValue.equals(wireValue)) {
				return type;
			}
		}
		throw new IllegalArgumentException("Unknown ContentLayer: " + wireValue);
	}
}
