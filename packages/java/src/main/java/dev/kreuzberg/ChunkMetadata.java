package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Objects;
import java.util.Optional;

/**
 * Metadata describing where a chunk appears within the original document.
 *
 * <p>
 * Includes byte offsets into the content string (UTF-8 boundaries), optional
 * page tracking information, and chunking statistics.
 */
public final class ChunkMetadata {
	private final long byteStart;
	private final long byteEnd;
	private final Long firstPage;
	private final Long lastPage;
	private final Integer tokenCount;
	private final int chunkIndex;
	private final int totalChunks;
	private final HeadingContext headingContext;

	@JsonCreator
	public ChunkMetadata(@JsonProperty("byte_start") long byteStart, @JsonProperty("byte_end") long byteEnd,
			@JsonProperty("first_page") Long firstPage, @JsonProperty("last_page") Long lastPage,
			@JsonProperty("token_count") Integer tokenCount, @JsonProperty("chunk_index") int chunkIndex,
			@JsonProperty("total_chunks") int totalChunks,
			@JsonProperty("heading_context") HeadingContext headingContext) {
		if (byteStart < 0 || byteEnd < byteStart) {
			throw new IllegalArgumentException("Invalid chunk byte range: " + byteStart + "-" + byteEnd);
		}
		if (chunkIndex < 0) {
			throw new IllegalArgumentException("chunkIndex must be non-negative");
		}
		if (totalChunks < 1) {
			throw new IllegalArgumentException("totalChunks must be positive");
		}
		if (firstPage != null && firstPage < 1) {
			throw new IllegalArgumentException("firstPage must be positive");
		}
		if (lastPage != null && lastPage < 1) {
			throw new IllegalArgumentException("lastPage must be positive");
		}
		if (firstPage != null && lastPage != null && lastPage < firstPage) {
			throw new IllegalArgumentException("lastPage must be >= firstPage");
		}
		this.byteStart = byteStart;
		this.byteEnd = byteEnd;
		this.firstPage = firstPage;
		this.lastPage = lastPage;
		this.tokenCount = tokenCount;
		this.chunkIndex = chunkIndex;
		this.totalChunks = totalChunks;
		this.headingContext = headingContext;
	}

	/**
	 * Get the byte offset where this chunk starts in the content string (UTF-8
	 * boundary, inclusive).
	 *
	 * @return start byte offset
	 */
	public long getByteStart() {
		return byteStart;
	}

	/**
	 * Get the byte offset where this chunk ends in the content string (UTF-8
	 * boundary, exclusive).
	 *
	 * @return end byte offset
	 */
	public long getByteEnd() {
		return byteEnd;
	}

	/**
	 * Get the first page number this chunk spans (1-indexed, optional).
	 *
	 * @return first page number, or empty if not tracked
	 */
	public Optional<Long> getFirstPage() {
		return Optional.ofNullable(firstPage);
	}

	/**
	 * Get the last page number this chunk spans (1-indexed, optional).
	 *
	 * <p>
	 * Equal to firstPage for single-page chunks.
	 *
	 * @return last page number, or empty if not tracked
	 */
	public Optional<Long> getLastPage() {
		return Optional.ofNullable(lastPage);
	}

	/**
	 * Get the token count for this chunk (optional).
	 *
	 * @return token count, or empty if not available
	 */
	public Optional<Integer> getTokenCount() {
		return Optional.ofNullable(tokenCount);
	}

	/**
	 * Get the index of this chunk within the total chunks.
	 *
	 * @return zero-based chunk index
	 */
	public int getChunkIndex() {
		return chunkIndex;
	}

	/**
	 * Get the total number of chunks this document was split into.
	 *
	 * @return total chunk count
	 */
	public int getTotalChunks() {
		return totalChunks;
	}

	/**
	 * Get the heading context for this chunk's section (optional).
	 *
	 * @return heading context, or empty if not available
	 */
	public Optional<HeadingContext> getHeadingContext() {
		return Optional.ofNullable(headingContext);
	}

	@Override
	public boolean equals(Object obj) {
		if (this == obj) {
			return true;
		}
		if (!(obj instanceof ChunkMetadata)) {
			return false;
		}
		ChunkMetadata other = (ChunkMetadata) obj;
		return byteStart == other.byteStart && byteEnd == other.byteEnd && Objects.equals(firstPage, other.firstPage)
				&& Objects.equals(lastPage, other.lastPage) && Objects.equals(tokenCount, other.tokenCount)
				&& chunkIndex == other.chunkIndex && totalChunks == other.totalChunks
				&& Objects.equals(headingContext, other.headingContext);
	}

	@Override
	public int hashCode() {
		return Objects.hash(byteStart, byteEnd, firstPage, lastPage, tokenCount, chunkIndex, totalChunks,
				headingContext);
	}

	@Override
	public String toString() {
		return "ChunkMetadata{" + "byteStart=" + byteStart + ", byteEnd=" + byteEnd + ", firstPage=" + firstPage
				+ ", lastPage=" + lastPage + ", tokenCount=" + tokenCount + ", chunkIndex=" + chunkIndex
				+ ", totalChunks=" + totalChunks + ", headingContext=" + headingContext + '}';
	}
}
