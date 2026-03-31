package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Type classification for PDF annotations.
 *
 * <p>
 * Categorizes the kind of annotation found in a PDF document, such as
 * text comments, highlights, hyperlinks, stamps, and text markup.
 *
 * @since 4.6.0
 */
public enum PdfAnnotationType {
	/** Sticky note / text annotation. */
	TEXT("text"),

	/** Highlighted text region. */
	HIGHLIGHT("highlight"),

	/** Hyperlink annotation. */
	LINK("link"),

	/** Rubber stamp annotation. */
	STAMP("stamp"),

	/** Underline text markup. */
	UNDERLINE("underline"),

	/** Strikeout text markup. */
	STRIKE_OUT("strike_out"),

	/** Any other annotation type. */
	OTHER("other");

	private final String wireValue;

	PdfAnnotationType(String wireValue) {
		this.wireValue = wireValue;
	}

	/**
	 * Get the wire format value for this annotation type.
	 *
	 * @return wire value used in serialization
	 */
	@JsonValue
	public String wireValue() {
		return wireValue;
	}

	/**
	 * Parse a PdfAnnotationType from its wire value.
	 *
	 * @param wireValue
	 *            the wire format value (snake_case string)
	 * @return the corresponding PdfAnnotationType
	 * @throws IllegalArgumentException
	 *             if the value is not recognized
	 */
	public static PdfAnnotationType fromWireValue(String wireValue) {
		for (PdfAnnotationType type : values()) {
			if (type.wireValue.equals(wireValue)) {
				return type;
			}
		}
		throw new IllegalArgumentException("Unknown PdfAnnotationType: " + wireValue);
	}
}
