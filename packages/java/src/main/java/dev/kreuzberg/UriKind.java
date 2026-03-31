package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Semantic classification of a URI found in a document.
 *
 * <p>
 * Categorizes URIs into types such as hyperlinks, image references,
 * anchors, citations, general references, and email addresses.
 *
 * @since 4.6.0
 */
public enum UriKind {
	/** A clickable hyperlink (web URL, file link). */
	HYPERLINK("hyperlink"),

	/** An image or media resource reference. */
	IMAGE("image"),

	/** An internal anchor or cross-reference target. */
	ANCHOR("anchor"),

	/** A citation or bibliographic reference. */
	CITATION("citation"),

	/** A general reference. */
	REFERENCE("reference"),

	/** An email address. */
	EMAIL("email");

	private final String wireValue;

	UriKind(String wireValue) {
		this.wireValue = wireValue;
	}

	/**
	 * Get the wire format value for this URI kind.
	 *
	 * @return wire value used in serialization
	 */
	@JsonValue
	public String wireValue() {
		return wireValue;
	}

	/**
	 * Parse a UriKind from its wire value.
	 *
	 * @param wireValue
	 *            the wire format value (snake_case string)
	 * @return the corresponding UriKind
	 * @throws IllegalArgumentException
	 *             if the value is not recognized
	 */
	public static UriKind fromWireValue(String wireValue) {
		for (UriKind type : values()) {
			if (type.wireValue.equals(wireValue)) {
				return type;
			}
		}
		throw new IllegalArgumentException("Unknown UriKind: " + wireValue);
	}
}
