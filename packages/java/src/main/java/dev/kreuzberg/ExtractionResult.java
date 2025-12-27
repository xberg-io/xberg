package dev.kreuzberg;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * Result of a document extraction operation.
 *
 * <p>Includes extracted content, tables, metadata, detected languages, text chunks, images,
 * page structure information, and success flag.</p>
 */
public final class ExtractionResult {
    private final String content;
    private final String mimeType;
    private final Map<String, Object> metadata;
    private final List<Table> tables;
    private final List<String> detectedLanguages;
    private final List<Chunk> chunks;
    private final List<ExtractedImage> images;
    private final PageStructure pageStructure;
    private final boolean success;
    private final Optional<String> language;
    private final Optional<String> date;
    private final Optional<String> subject;

    ExtractionResult(
        String content,
        String mimeType,
        Map<String, Object> metadata,
        List<Table> tables,
        List<String> detectedLanguages,
        List<Chunk> chunks,
        List<ExtractedImage> images,
        PageStructure pageStructure,
        boolean success
    ) {
        this.content = Objects.requireNonNull(content, "content must not be null");
        this.mimeType = Objects.requireNonNull(mimeType, "mimeType must not be null");
        this.metadata = Collections.unmodifiableMap(metadata != null ? metadata : Collections.emptyMap());
        this.tables = Collections.unmodifiableList(tables != null ? tables : Collections.emptyList());
        if (detectedLanguages != null) {
            this.detectedLanguages = Collections.unmodifiableList(detectedLanguages);
        } else {
            this.detectedLanguages = List.of();
        }
        this.chunks = Collections.unmodifiableList(chunks != null ? chunks : List.of());
        this.images = Collections.unmodifiableList(images != null ? images : List.of());
        this.pageStructure = pageStructure;
        this.success = success;
        this.language = Optional.ofNullable((String) this.metadata.get("language"));
        this.date = Optional.ofNullable((String) this.metadata.get("date"));
        this.subject = Optional.ofNullable((String) this.metadata.get("subject"));
    }

    public String getContent() {
        return content;
    }

    public String getMimeType() {
        return mimeType;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public List<Table> getTables() {
        return tables;
    }

    public List<String> getDetectedLanguages() {
        return detectedLanguages;
    }

    public List<Chunk> getChunks() {
        return chunks;
    }

    public List<ExtractedImage> getImages() {
        return images;
    }

    /**
     * Get the page structure information (optional).
     *
     * Available when page tracking is enabled in the extraction configuration.
     *
     * @return page structure, or empty if not available
     */
    public Optional<PageStructure> getPageStructure() {
        return Optional.ofNullable(pageStructure);
    }

    public boolean isSuccess() {
        return success;
    }

    public Optional<String> getLanguage() {
        return language;
    }

    public Optional<String> getDate() {
        return date;
    }

    public Optional<String> getSubject() {
        return subject;
    }

    /**
     * Get the total page count from the result.
     *
     * <p>This calls the Rust FFI backend for efficient access to metadata.</p>
     *
     * @return the page count, or -1 on error
     * @since 4.0.0
     */
    public int getPageCount() {
        if (this.metadata != null) {
            Object pages = this.metadata.get("pages");
            if (pages instanceof Map) {
                Object count = ((Map<?, ?>) pages).get("totalCount");
                if (count instanceof Number) {
                    return ((Number) count).intValue();
                }
            }
        }
        return 0;
    }

    /**
     * Get the total chunk count from the result.
     *
     * <p>Returns the number of text chunks when chunking is enabled.</p>
     *
     * @return the chunk count, or 0 if no chunks available
     * @since 4.0.0
     */
    public int getChunkCount() {
        if (this.chunks != null) {
            return this.chunks.size();
        }
        return 0;
    }

    /**
     * Get the detected primary language code.
     *
     * <p>Returns the primary detected language as an ISO 639 code.</p>
     *
     * @return the detected language code (e.g., "en", "de"), or empty if not detected
     * @since 4.0.0
     */
    public Optional<String> getDetectedLanguage() {
        if (this.metadata != null) {
            Object langObj = this.metadata.get("language");
            if (langObj instanceof String lang && !lang.isEmpty()) {
                return Optional.of(lang);
            }
        }

        if (this.detectedLanguages != null && !this.detectedLanguages.isEmpty()) {
            return Optional.of(this.detectedLanguages.get(0));
        }

        return Optional.empty();
    }

    /**
     * Get a metadata field by name.
     *
     * <p>Supports nested field access with dot notation (e.g., "format.pages").</p>
     *
     * @param fieldName the field name to retrieve
     * @return the field value as an Object, or empty if not found
     * @throws KreuzbergException if retrieval fails
     * @since 4.0.0
     */
    public Optional<Object> getMetadataField(String fieldName) throws KreuzbergException {
        if (fieldName == null || fieldName.isEmpty()) {
            throw new IllegalArgumentException("fieldName cannot be null or empty");
        }

        if ("title".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("title"));
        }
        if ("author".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("author"));
        }
        if ("subject".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("subject"));
        }
        if ("keywords".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("keywords"));
        }
        if ("language".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("language"));
        }
        if ("created".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("created"));
        }
        if ("modified".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("modified"));
        }
        if ("creators".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("creators"));
        }
        if ("format".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("format"));
        }
        if ("pages".equals(fieldName)) {
            return Optional.ofNullable(this.metadata.get("pages"));
        }

        return Optional.empty();
    }

    @Override
    public String toString() {
        return "ExtractionResult{"
            + "contentLength=" + content.length()
            + ", mimeType='" + mimeType + '\''
            + ", tables=" + tables.size()
            + ", detectedLanguages=" + detectedLanguages
            + ", chunks=" + chunks.size()
            + ", images=" + images.size()
            + ", success=" + success
            + '}';
    }
}
