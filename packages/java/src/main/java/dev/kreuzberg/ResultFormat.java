package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Result format for extraction output structure.
 *
 * <p>
 * Controls whether the extraction result uses a unified content field
 * or element-based semantic extraction.
 *
 * @since 4.6.0
 */
public enum ResultFormat {
	/** Unified format with all content in the content field. */
	UNIFIED("unified"),

	/** Element-based format with semantic element extraction. */
	ELEMENT_BASED("element_based");

	private final String wireValue;

	ResultFormat(String wireValue) {
		this.wireValue = wireValue;
	}

	/**
	 * Get the wire format value for this result format.
	 *
	 * @return wire value used in serialization
	 */
	@JsonValue
	public String wireValue() {
		return wireValue;
	}

	/**
	 * Parse a ResultFormat from its wire value.
	 *
	 * @param wireValue
	 *            the wire format value (snake_case string)
	 * @return the corresponding ResultFormat
	 * @throws IllegalArgumentException
	 *             if the value is not recognized
	 */
	public static ResultFormat fromWireValue(String wireValue) {
		for (ResultFormat type : values()) {
			if (type.wireValue.equals(wireValue)) {
				return type;
			}
		}
		throw new IllegalArgumentException("Unknown ResultFormat: " + wireValue);
	}
}
