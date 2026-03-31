package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * Result of a document extraction operation.
 *
 * <p>
 * Includes extracted content, tables, metadata, detected languages, text
 * chunks, images, page structure information, Djot content, extracted keywords,
 * quality score, and processing warnings.
 */
public final class ExtractionResult {
	private final String content;
	private final String mimeType;
	private final Metadata metadata;
	private final List<Table> tables;
	private final List<String> detectedLanguages;
	private final List<Chunk> chunks;
	private final List<ExtractedImage> images;
	private final List<PageContent> pages;
	private final PageStructure pageStructure;
	private final List<Element> elements;
	private final List<OcrElement> ocrElements;
	@JsonProperty("djot_content")
	private final DjotContent djotContent;
	private final DocumentStructure document;
	@JsonProperty("extracted_keywords")
	private final List<ExtractedKeyword> extractedKeywords;
	@JsonProperty("quality_score")
	private final Double qualityScore;
	@JsonProperty("processing_warnings")
	private final List<ProcessingWarning> processingWarnings;
	@JsonProperty("annotations")
	private final List<PdfAnnotation> annotations;
	@JsonProperty("uris")
	private final List<Uri> uris;
	@JsonProperty("children")
	private final List<ArchiveEntry> children;

	ExtractionResult(String content, String mimeType, Metadata metadata, List<Table> tables,
			List<String> detectedLanguages, List<Chunk> chunks, List<ExtractedImage> images, List<PageContent> pages,
			PageStructure pageStructure, List<Element> elements, List<OcrElement> ocrElements, DjotContent djotContent,
			DocumentStructure document, List<ExtractedKeyword> extractedKeywords, Double qualityScore,
			List<ProcessingWarning> processingWarnings, List<PdfAnnotation> annotations, List<Uri> uris) {
		this(content, mimeType, metadata, tables, detectedLanguages, chunks, images, pages, pageStructure, elements,
				ocrElements, djotContent, document, extractedKeywords, qualityScore, processingWarnings, annotations,
				uris, null);
	}

	ExtractionResult(String content, String mimeType, Metadata metadata, List<Table> tables,
			List<String> detectedLanguages, List<Chunk> chunks, List<ExtractedImage> images, List<PageContent> pages,
			PageStructure pageStructure, List<Element> elements, List<OcrElement> ocrElements, DjotContent djotContent,
			DocumentStructure document, List<ExtractedKeyword> extractedKeywords, Double qualityScore,
			List<ProcessingWarning> processingWarnings, List<PdfAnnotation> annotations, List<Uri> uris,
			List<ArchiveEntry> children) {
		this.content = Objects.requireNonNull(content, "content must not be null");
		this.mimeType = Objects.requireNonNull(mimeType, "mimeType must not be null");
		this.metadata = metadata != null ? metadata : Metadata.empty();
		this.tables = Collections.unmodifiableList(tables != null ? tables : Collections.emptyList());
		if (detectedLanguages != null) {
			this.detectedLanguages = Collections.unmodifiableList(detectedLanguages);
		} else {
			this.detectedLanguages = List.of();
		}
		this.chunks = Collections.unmodifiableList(chunks != null ? chunks : List.of());
		this.images = Collections.unmodifiableList(images != null ? images : List.of());
		this.pages = Collections.unmodifiableList(pages != null ? pages : List.of());
		this.pageStructure = pageStructure;
		this.elements = Collections.unmodifiableList(elements != null ? elements : List.of());
		this.ocrElements = Collections.unmodifiableList(ocrElements != null ? ocrElements : List.of());
		this.djotContent = djotContent;
		this.document = document;
		this.extractedKeywords = extractedKeywords != null ? Collections.unmodifiableList(extractedKeywords) : null;
		this.qualityScore = qualityScore;
		this.processingWarnings = processingWarnings != null ? Collections.unmodifiableList(processingWarnings) : null;
		this.annotations = annotations != null ? Collections.unmodifiableList(annotations) : null;
		this.uris = uris != null ? Collections.unmodifiableList(uris) : null;
		this.children = children != null ? Collections.unmodifiableList(children) : null;
	}

	public String getContent() {
		return content;
	}

	public String getMimeType() {
		return mimeType;
	}

	public Metadata getMetadata() {
		return metadata;
	}

	/**
	 * Get metadata as a Map for backward compatibility.
	 *
	 * @return metadata converted to a Map representation
	 * @deprecated Use {@link #getMetadata()} instead for typed access
	 */
	@Deprecated(since = "0.8.0", forRemoval = true)
	public Map<String, Object> getMetadataMap() {
		Map<String, Object> map = new HashMap<>();
		metadata.getTitle().ifPresent(v -> map.put("title", v));
		metadata.getSubject().ifPresent(v -> map.put("subject", v));
		metadata.getAuthors().ifPresent(v -> map.put("authors", v));
		metadata.getKeywords().ifPresent(v -> map.put("keywords", v));
		metadata.getLanguage().ifPresent(v -> map.put("language", v));
		metadata.getCreatedAt().ifPresent(v -> map.put("created", v));
		metadata.getModifiedAt().ifPresent(v -> map.put("modified", v));
		metadata.getCreatedBy().ifPresent(v -> map.put("created_by", v));
		metadata.getModifiedBy().ifPresent(v -> map.put("modified_by", v));
		metadata.getPages().ifPresent(v -> map.put("pages", v));
		metadata.getImagePreprocessing().ifPresent(v -> map.put("image_preprocessing", v));
		metadata.getJsonSchema().ifPresent(v -> map.put("json_schema", v));
		metadata.getError().ifPresent(v -> map.put("error", v));
		metadata.getCategory().ifPresent(v -> map.put("category", v));
		metadata.getTags().ifPresent(v -> map.put("tags", v));
		metadata.getDocumentVersion().ifPresent(v -> map.put("document_version", v));
		metadata.getAbstractText().ifPresent(v -> map.put("abstract_text", v));
		metadata.getOutputFormat().ifPresent(v -> map.put("output_format", v));
		map.putAll(metadata.getAdditional());
		return Collections.unmodifiableMap(map);
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
	 * Get the per-page content when page extraction is enabled.
	 *
	 * @return unmodifiable list of page contents (never null, but may be empty)
	 * @since 4.2.4
	 */
	public List<PageContent> getPages() {
		return pages;
	}

	/**
	 * Get the semantic elements extracted from the document.
	 *
	 * <p>
	 * Available when extraction is configured with
	 * {@code output_format="element_based"}. Returns an empty list if element
	 * extraction is not enabled or if no elements were extracted.
	 *
	 * @return unmodifiable list of semantic elements (never null, but may be empty)
	 * @since 4.1.0
	 */
	public List<Element> getElements() {
		return elements;
	}

	/**
	 * Get the OCR elements extracted from the document.
	 *
	 * <p>
	 * Available when OCR element extraction is enabled via OcrElementConfig.
	 * Returns an empty list if OCR element extraction is not enabled or if no
	 * elements were extracted.
	 *
	 * @return unmodifiable list of OCR elements (never null, but may be empty)
	 * @since 4.4.0
	 */
	public List<OcrElement> getOcrElements() {
		return ocrElements;
	}

	/**
	 * Get the page structure information (optional).
	 *
	 * <p>
	 * Available when page tracking is enabled in the extraction configuration.
	 *
	 * @return page structure, or empty if not available
	 */
	public Optional<PageStructure> getPageStructure() {
		return Optional.ofNullable(pageStructure);
	}

	public Optional<DjotContent> getDjotContent() {
		return Optional.ofNullable(djotContent);
	}

	/**
	 * Get the document structure (optional).
	 *
	 * <p>
	 * Available when document structure extraction is enabled in the extraction
	 * configuration via {@code include_document_structure=true}.
	 *
	 * @return document structure, or empty if not available
	 * @since 4.3.0
	 */
	public Optional<DocumentStructure> getDocumentStructure() {
		return Optional.ofNullable(document);
	}

	/**
	 * Get the keywords extracted from the document (optional).
	 *
	 * <p>
	 * Available when keyword extraction is enabled in the extraction configuration.
	 *
	 * @return optional unmodifiable list of extracted keywords, or empty if not
	 *         available
	 * @since 4.5.0
	 */
	public Optional<List<ExtractedKeyword>> getExtractedKeywords() {
		return Optional.ofNullable(extractedKeywords);
	}

	/**
	 * Get the quality score of the extraction (optional).
	 *
	 * <p>
	 * A numeric score indicating the quality of the extraction result. Available
	 * when quality processing is enabled.
	 *
	 * @return optional quality score, or empty if not available
	 * @since 4.5.0
	 */
	public Optional<Double> getQualityScore() {
		return Optional.ofNullable(qualityScore);
	}

	/**
	 * Get the processing warnings generated during extraction (optional).
	 *
	 * <p>
	 * Contains warnings from various processing stages that did not prevent
	 * extraction but may indicate potential issues.
	 *
	 * @return optional unmodifiable list of processing warnings, or empty if none
	 * @since 4.5.0
	 */
	public Optional<List<ProcessingWarning>> getProcessingWarnings() {
		return Optional.ofNullable(processingWarnings);
	}

	/**
	 * Get the PDF annotations extracted from the document (optional).
	 *
	 * <p>
	 * Available when the document is a PDF containing annotations such as comments,
	 * highlights, links, stamps, underlines, or strikeouts.
	 *
	 * @return optional unmodifiable list of PDF annotations, or empty if none
	 */
	public Optional<List<PdfAnnotation>> getAnnotations() {
		return Optional.ofNullable(annotations);
	}

	/**
	 * Get the URIs/links discovered during document extraction (optional).
	 *
	 * <p>
	 * Contains hyperlinks, image references, citations, email addresses, and
	 * other URI-like references found in the document.
	 *
	 * @return optional unmodifiable list of URIs, or empty if none
	 */
	public Optional<List<Uri>> getUris() {
		return Optional.ofNullable(uris);
	}

	/**
	 * Get the archive children extracted from archive documents (optional).
	 *
	 * <p>
	 * Available when the document is an archive (ZIP, TAR, etc.) and archive
	 * extraction is enabled. Each child entry contains the extraction result
	 * for a file within the archive.
	 *
	 * @return optional unmodifiable list of archive entries, or empty if none
	 * @since 4.6.0
	 */
	public Optional<List<ArchiveEntry>> getChildren() {
		return Optional.ofNullable(children);
	}

	/**
	 * Check if the extraction was successful.
	 *
	 * <p>
	 * This method always returns true for a valid ExtractionResult. If extraction
	 * fails, an exception is thrown instead of returning an unsuccessful result.
	 *
	 * @return true (always, since invalid results throw exceptions)
	 * @deprecated This method is deprecated as extraction failures now throw
	 *             exceptions. All ExtractionResult instances represent successful
	 *             extractions.
	 */
	@Deprecated(since = "0.8.0", forRemoval = true)
	public boolean isSuccess() {
		return true;
	}

	/**
	 * Get the detected language from metadata.
	 *
	 * <p>
	 * Use {@link #getDetectedLanguage()} instead, which retrieves the primary
	 * detected language from either metadata or the detectedLanguages list.
	 *
	 * @return the language code from metadata, or empty if not available
	 * @deprecated Use {@link #getDetectedLanguage()} instead. This method only
	 *             retrieves language from metadata and doesn't check
	 *             detectedLanguages.
	 */
	@Deprecated(since = "0.8.0", forRemoval = true)
	public Optional<String> getLanguage() {
		return metadata.getLanguage();
	}

	/**
	 * Get the document creation date from metadata.
	 *
	 * @return the creation date from metadata, or empty if not available
	 * @deprecated Use {@link #getMetadataField(String)} with "created" or
	 *             "modified" instead for more precise date field access.
	 */
	@Deprecated(since = "0.8.0", forRemoval = true)
	public Optional<String> getDate() {
		return metadata.getModifiedAt();
	}

	/**
	 * Get the document subject from metadata.
	 *
	 * @return the subject from metadata, or empty if not available
	 * @deprecated Use {@link #getMetadataField(String)} with "subject" instead.
	 */
	@Deprecated(since = "0.8.0", forRemoval = true)
	public Optional<String> getSubject() {
		return metadata.getSubject();
	}

	/**
	 * Get the total page count from the result.
	 *
	 * <p>
	 * This calls the Rust FFI backend for efficient access to metadata.
	 *
	 * @return the page count, or -1 on error
	 * @since 4.0.0
	 */
	public int getPageCount() {
		Map<String, Object> additional = metadata.getAdditional();
		if (additional != null && !additional.isEmpty()) {
			Object pages = additional.get("pages");
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
	 * <p>
	 * Returns the number of text chunks when chunking is enabled.
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
	 * <p>
	 * Returns the primary detected language as an ISO 639 code.
	 *
	 * @return the detected language code (e.g., "en", "de"), or empty if not
	 *         detected
	 * @since 4.0.0
	 */
	public Optional<String> getDetectedLanguage() {
		// First check metadata.language
		Optional<String> langFromMetadata = metadata.getLanguage();
		if (langFromMetadata.isPresent() && !langFromMetadata.get().isEmpty()) {
			return langFromMetadata;
		}

		// Fall back to first item in detectedLanguages list
		if (this.detectedLanguages != null && !this.detectedLanguages.isEmpty()) {
			return Optional.of(this.detectedLanguages.get(0));
		}

		return Optional.empty();
	}

	/**
	 * Get a metadata field by name.
	 *
	 * <p>
	 * Supports nested field access with dot notation (e.g., "format.pages").
	 *
	 * @param fieldName
	 *            the field name to retrieve
	 * @return the field value as an Object, or empty if not found
	 * @throws KreuzbergException
	 *             if retrieval fails
	 * @since 4.0.0
	 */
	public Optional<Object> getMetadataField(String fieldName) throws KreuzbergException {
		if (fieldName == null || fieldName.isEmpty()) {
			throw new IllegalArgumentException("fieldName cannot be null or empty");
		}

		return switch (fieldName) {
			case "title" -> Optional.ofNullable(metadata.getTitle().orElse(null));
			case "author", "creator" -> Optional.ofNullable(metadata.getCreatedBy().orElse(null));
			case "subject" -> Optional.ofNullable(metadata.getSubject().orElse(null));
			case "keywords" -> Optional.ofNullable(metadata.getKeywords().map(v -> (Object) v).orElse(null));
			case "language" -> Optional.ofNullable(metadata.getLanguage().orElse(null));
			case "created" -> Optional.ofNullable(metadata.getCreatedAt().orElse(null));
			case "modified" -> Optional.ofNullable(metadata.getModifiedAt().orElse(null));
			case "creators" -> Optional.ofNullable(metadata.getAuthors().map(v -> (Object) v).orElse(null));
			case "format", "pages" -> {
				Map<String, Object> additional = metadata.getAdditional();
				if (!additional.isEmpty()) {
					yield Optional.ofNullable(additional.get(fieldName));
				}
				yield Optional.empty();
			}
			default -> Optional.empty();
		};
	}

	@Override
	public String toString() {
		return "ExtractionResult{" + "contentLength=" + content.length() + ", mimeType='" + mimeType + '\''
				+ ", tables=" + tables.size() + ", detectedLanguages=" + detectedLanguages + ", chunks=" + chunks.size()
				+ ", images=" + images.size() + ", pages=" + pages.size() + ", elements=" + elements.size()
				+ ", ocrElements=" + ocrElements.size() + ", hasDjotContent=" + (djotContent != null)
				+ ", hasDocumentStructure=" + (document != null) + ", extractedKeywords="
				+ (extractedKeywords != null ? extractedKeywords.size() : "null") + ", qualityScore=" + qualityScore
				+ ", processingWarnings=" + (processingWarnings != null ? processingWarnings.size() : "null")
				+ ", annotations=" + (annotations != null ? annotations.size() : "null")
				+ ", uris=" + (uris != null ? uris.size() : "null")
				+ ", children=" + (children != null ? children.size() : "null") + '}';
	}
}
