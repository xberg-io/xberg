package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Kind of relationship between document structure elements.
 *
 * <p>
 * Describes semantic links between parts of a document, such as
 * footnote references, citations, internal anchors, captions, labels,
 * table-of-contents entries, and cross-references.
 *
 * @since 4.6.0
 */
public enum RelationshipKind {
	/** Footnote marker to footnote definition. */
	FOOTNOTE_REFERENCE("footnote_reference"),

	/** Citation marker to bibliography entry. */
	CITATION_REFERENCE("citation_reference"),

	/** Internal anchor link to target heading/element. */
	INTERNAL_LINK("internal_link"),

	/** Caption paragraph to figure/table it describes. */
	CAPTION("caption"),

	/** Label to labeled element. */
	LABEL("label"),

	/** TOC entry to target section. */
	TOC_ENTRY("toc_entry"),

	/** Cross-reference to target element. */
	CROSS_REFERENCE("cross_reference");

	private final String wireValue;

	RelationshipKind(String wireValue) {
		this.wireValue = wireValue;
	}

	/**
	 * Get the wire format value for this relationship kind.
	 *
	 * @return wire value used in serialization
	 */
	@JsonValue
	public String wireValue() {
		return wireValue;
	}

	/**
	 * Parse a RelationshipKind from its wire value.
	 *
	 * @param wireValue
	 *            the wire format value (snake_case string)
	 * @return the corresponding RelationshipKind
	 * @throws IllegalArgumentException
	 *             if the value is not recognized
	 */
	public static RelationshipKind fromWireValue(String wireValue) {
		for (RelationshipKind type : values()) {
			if (type.wireValue.equals(wireValue)) {
				return type;
			}
		}
		throw new IllegalArgumentException("Unknown RelationshipKind: " + wireValue);
	}
}
