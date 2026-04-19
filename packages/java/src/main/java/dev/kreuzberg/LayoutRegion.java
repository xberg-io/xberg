package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Objects;

/**
 * A detected layout region on a page.
 *
 * <p>
 * When layout detection is enabled, each page may carry layout regions
 * identifying different content types (text, pictures, tables, etc.) with
 * confidence scores and spatial positions.
 *
 * @see PageContent
 * @since 4.1.0
 */
public final class LayoutRegion {
	private final String className;
	private final double confidence;
	private final BoundingBox boundingBox;
	private final double areaFraction;

	/**
	 * Create a new layout region.
	 *
	 * @param className
	 *            the detected content class (e.g. "text", "picture", "table")
	 * @param confidence
	 *            detection confidence in [0, 1]
	 * @param boundingBox
	 *            spatial location of the region on the page
	 * @param areaFraction
	 *            fraction of page area covered by this region in [0, 1]
	 */
	@JsonCreator
	public LayoutRegion(
			@JsonProperty("class") String className,
			@JsonProperty("confidence") double confidence,
			@JsonProperty("bounding_box") BoundingBox boundingBox,
			@JsonProperty("area_fraction") double areaFraction) {
		this.className = className != null ? className : "";
		this.confidence = confidence;
		this.boundingBox = boundingBox != null ? boundingBox : new BoundingBox(0, 0, 0, 0);
		this.areaFraction = areaFraction;
	}

	/**
	 * Get the detected content class name.
	 *
	 * @return content class (e.g. "text", "picture", "table")
	 */
	@JsonProperty("class")
	public String getClassName() {
		return className;
	}

	/**
	 * Get the detection confidence score.
	 *
	 * @return confidence in [0, 1]
	 */
	public double getConfidence() {
		return confidence;
	}

	/**
	 * Get the bounding box of this region on the page.
	 *
	 * @return bounding box (never null)
	 */
	@JsonProperty("bounding_box")
	public BoundingBox getBoundingBox() {
		return boundingBox;
	}

	/**
	 * Get the fraction of the page area covered by this region.
	 *
	 * @return area fraction in [0, 1]
	 */
	@JsonProperty("area_fraction")
	public double getAreaFraction() {
		return areaFraction;
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof LayoutRegion)) {
			return false;
		}
		LayoutRegion other = (LayoutRegion) obj;
		return Double.doubleToLongBits(confidence) == Double.doubleToLongBits(other.confidence)
				&& Double.doubleToLongBits(areaFraction) == Double.doubleToLongBits(other.areaFraction)
				&& className.equals(other.className)
				&& Objects.equals(boundingBox, other.boundingBox);
	}

	@Override
	public int hashCode() {
		return Objects.hash(className, Double.doubleToLongBits(confidence), boundingBox,
				Double.doubleToLongBits(areaFraction));
	}

	@Override
	public String toString() {
		return "LayoutRegion{class=" + className + ", confidence=" + confidence
				+ ", areaFraction=" + areaFraction + '}';
	}
}
