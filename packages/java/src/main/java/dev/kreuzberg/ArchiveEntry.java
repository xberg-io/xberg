package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Objects;

/**
 * An entry extracted from an archive file (ZIP, TAR, etc.).
 *
 * <p>
 * Represents a single file within an archive, containing the
 * archive-relative file path, detected MIME type, and the full
 * extraction result for the file.
 *
 * @since 4.6.0
 */
@JsonIgnoreProperties(ignoreUnknown = true)
public final class ArchiveEntry {
	private final String path;
	private final String mimeType;
	private final ExtractionResult result;

	@JsonCreator
	public ArchiveEntry(@JsonProperty("path") String path, @JsonProperty("mime_type") String mimeType,
			@JsonProperty("result") ExtractionResult result) {
		this.path = Objects.requireNonNull(path, "path must not be null");
		this.mimeType = Objects.requireNonNull(mimeType, "mimeType must not be null");
		this.result = Objects.requireNonNull(result, "result must not be null");
	}

	/**
	 * Get the archive-relative file path.
	 *
	 * @return the file path within the archive (e.g., "folder/document.pdf")
	 */
	public String getPath() {
		return path;
	}

	/**
	 * Get the detected MIME type of the file.
	 *
	 * @return the MIME type string (never null)
	 */
	public String getMimeType() {
		return mimeType;
	}

	/**
	 * Get the full extraction result for this archive entry.
	 *
	 * @return the extraction result (never null)
	 */
	public ExtractionResult getResult() {
		return result;
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof ArchiveEntry)) {
			return false;
		}
		ArchiveEntry other = (ArchiveEntry) obj;
		return Objects.equals(path, other.path) && Objects.equals(mimeType, other.mimeType)
				&& Objects.equals(result, other.result);
	}

	@Override
	public int hashCode() {
		return Objects.hash(path, mimeType, result);
	}

	@Override
	public String toString() {
		return "ArchiveEntry{" + "path='" + path + '\'' + ", mimeType='" + mimeType + '\'' + '}';
	}
}
