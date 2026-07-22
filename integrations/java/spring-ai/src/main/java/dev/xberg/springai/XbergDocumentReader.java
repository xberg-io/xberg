package dev.xberg.springai;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jdk8.Jdk8Module;
import dev.xberg.BoundingBox;
import dev.xberg.Chunk;
import dev.xberg.ChunkMetadata;
import dev.xberg.Element;
import dev.xberg.ElementMetadata;
import dev.xberg.ExtractionResult;
import dev.xberg.Xberg;
import dev.xberg.XbergException;
import dev.xberg.Metadata;
import dev.xberg.PageContent;
import dev.xberg.config.ExtractionConfig;
import java.io.IOException;
import java.net.URLConnection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.springframework.ai.document.Document;
import org.springframework.ai.document.DocumentReader;
import org.springframework.core.io.FileSystemResource;
import org.springframework.core.io.Resource;

/**
 * A Spring AI {@link DocumentReader} that uses Xberg for document
 * extraction.
 *
 * <p>
 * Supports 100+ document formats including PDF, DOCX, PPTX, images (with OCR),
 * and more. Documents are split into Spring AI {@link Document} instances using
 * a priority-based strategy: chunks &gt; elements &gt; pages &gt; whole
 * document.
 *
 * <p>
 * Use the {@link #builder()} to configure the reader:
 *
 * <pre>{@code
 * var reader = XbergDocumentReader.builder().resource(new FileSystemResource("report.pdf")).build();
 * List<Document> docs = reader.get();
 * }</pre>
 *
 * <p>
 * For non-file resources (e.g. {@code ByteArrayResource}), a MIME type must be
 * provided:
 *
 * <pre>{@code
 * var reader = XbergDocumentReader.builder().resource(new ByteArrayResource(bytes)).mimeType("application/pdf")
 * 		.build();
 * }</pre>
 */
public final class XbergDocumentReader implements DocumentReader {

	private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper().registerModule(new Jdk8Module());

	private final Resource resource;
	private final String mimeType;
	private final ExtractionConfig extractionConfig;
	private final Map<String, Object> additionalMetadata;

	private XbergDocumentReader(Builder builder) {
		this.resource = builder.resource;
		this.mimeType = builder.mimeType;
		this.extractionConfig = builder.extractionConfig;
		this.additionalMetadata = Map.copyOf(builder.additionalMetadata);
	}

	/**
	 * Extracts the document and returns a list of Spring AI {@link Document}
	 * instances.
	 *
	 * <p>
	 * The splitting strategy follows priority order: if the extraction result
	 * contains chunks, each chunk becomes a document; otherwise elements are used,
	 * then pages, and finally the whole content as a single document.
	 *
	 * @return list of documents with extracted text and metadata
	 * @throws RuntimeException
	 *             if extraction or I/O fails
	 */
	@Override
	public List<Document> get() {
		try {
			ExtractionResult result = extract();
			return mapToDocuments(result);
		} catch (IOException e) {
			throw new RuntimeException("Failed to extract document", e);
		} catch (XbergException e) {
			throw new RuntimeException("Xberg extraction failed", e);
		}
	}

	/**
	 * Returns a new {@link Builder} for constructing a
	 * {@link XbergDocumentReader}.
	 */
	public static Builder builder() {
		return new Builder();
	}

	/**
	 * Builder for {@link XbergDocumentReader}.
	 *
	 * <p>
	 * At minimum, a {@link Resource} must be provided. For resources without a
	 * filename (e.g. {@code ByteArrayResource}), a MIME type is also required so
	 * Xberg knows how to parse the content.
	 */
	public static final class Builder {

		private Resource resource;
		private String mimeType;
		private ExtractionConfig extractionConfig;
		private final Map<String, Object> additionalMetadata = new LinkedHashMap<>();

		private Builder() {
		}

		/** Sets the Spring {@link Resource} to extract text from. Required. */
		public Builder resource(Resource resource) {
			this.resource = resource;
			return this;
		}

		/**
		 * Sets an explicit MIME type for the resource. Required when the resource has
		 * no filename (e.g. {@code ByteArrayResource}). Overrides any MIME type guessed
		 * from the filename.
		 */
		public Builder mimeType(String mimeType) {
			this.mimeType = mimeType;
			return this;
		}

		/**
		 * Sets the Xberg {@link ExtractionConfig} to control extraction behavior.
		 */
		public Builder extractionConfig(ExtractionConfig config) {
			this.extractionConfig = config;
			return this;
		}

		/**
		 * Adds all entries from the given map as additional metadata on each output
		 * document.
		 */
		public Builder metadata(Map<String, Object> metadata) {
			this.additionalMetadata.putAll(metadata);
			return this;
		}

		/**
		 * Adds a single key-value pair as additional metadata on each output document.
		 */
		public Builder metadata(String key, Object value) {
			this.additionalMetadata.put(key, value);
			return this;
		}

		/**
		 * Builds the reader, validating that required fields are set.
		 *
		 * @throws IllegalArgumentException
		 *             if resource is null or MIME type cannot be determined
		 */
		public XbergDocumentReader build() {
			if (resource == null) {
				throw new IllegalArgumentException("resource is required");
			}
			if (resource.getFilename() == null && mimeType == null) {
				throw new IllegalArgumentException(
						"mimeType is required when resource has no filename (e.g. ByteArrayResource)");
			}
			return new XbergDocumentReader(this);
		}
	}

	/**
	 * Routes extraction to the appropriate Xberg method based on resource type.
	 * FileSystemResource uses file-path extraction; all others read bytes into
	 * memory.
	 */
	private ExtractionResult extract() throws IOException, XbergException {
		if (resource instanceof FileSystemResource) {
			if (extractionConfig != null) {
				return Xberg.extractFile(resource.getFile().toPath(), extractionConfig);
			}
			return Xberg.extractFile(resource.getFile().toPath());
		}

		byte[] bytes = resource.getInputStream().readAllBytes();
		ExtractionConfig config = extractionConfig != null ? extractionConfig : ExtractionConfig.builder().build();
		return Xberg.extractBytes(bytes, resolveMimeType(), config);
	}

	private String resolveSource() {
		String filename = resource.getFilename();
		if (filename != null) {
			return filename;
		}
		return "bytes://" + resolveMimeType();
	}

	private String resolveMimeType() {
		if (mimeType != null) {
			return mimeType;
		}
		String filename = resource.getFilename();
		if (filename != null) {
			String guessed = URLConnection.guessContentTypeFromName(filename);
			if (guessed != null) {
				return guessed;
			}
			return "application/octet-stream";
		}
		throw new IllegalStateException("Cannot resolve MIME type: no explicit mimeType and resource has no filename");
	}

	/**
	 * Maps an extraction result to Spring AI documents using the
	 * highest-granularity splitting available: chunks > elements > pages > whole
	 * document.
	 */
	private List<Document> mapToDocuments(ExtractionResult result) {
		Map<String, Object> baseMetadata = buildBaseMetadata(result);

		List<Chunk> chunks = result.getChunks();
		if (chunks != null && !chunks.isEmpty()) {
			return mapChunksToDocuments(chunks, baseMetadata);
		}

		List<Element> elements = result.getElements();
		if (elements != null && !elements.isEmpty()) {
			return mapElementsToDocuments(elements, baseMetadata);
		}

		List<PageContent> pages = result.getPages();
		if (pages != null && !pages.isEmpty()) {
			return mapPagesToDocuments(pages, baseMetadata);
		}

		return List.of(new Document(result.getContent(), baseMetadata));
	}

	private List<Document> mapChunksToDocuments(List<Chunk> chunks, Map<String, Object> baseMetadata) {
		return chunks.stream().map(chunk -> {
			Map<String, Object> metadata = new LinkedHashMap<>(baseMetadata);
			ChunkMetadata chunkMeta = chunk.getMetadata();
			metadata.put("chunk_index", chunkMeta.getChunkIndex());
			metadata.put("total_chunks", chunkMeta.getTotalChunks());
			chunkMeta.getTokenCount().ifPresent(tc -> metadata.put("token_count", tc));
			chunkMeta.getFirstPage().ifPresent(fp -> metadata.put("first_page", fp));
			chunkMeta.getLastPage().ifPresent(lp -> metadata.put("last_page", lp));
			chunkMeta.getHeadingContext().ifPresent(hc -> metadata.put("heading_context", toJson(hc)));
			return new Document(chunk.getContent(), metadata);
		}).toList();
	}

	private List<Document> mapElementsToDocuments(List<Element> elements, Map<String, Object> baseMetadata) {
		return elements.stream().map(element -> {
			Map<String, Object> metadata = new LinkedHashMap<>(baseMetadata);
			metadata.put("element_id", element.getElementId());
			metadata.put("element_type", element.getElementType().wireValue());
			ElementMetadata elemMeta = element.getMetadata();
			elemMeta.getElementIndex().ifPresent(ei -> metadata.put("element_index", ei));
			elemMeta.getPageNumber().ifPresent(pn -> metadata.put("page_number", pn));
			elemMeta.getCoordinates().ifPresent((BoundingBox bbox) -> {
				metadata.put("bbox_x0", bbox.getX0());
				metadata.put("bbox_y0", bbox.getY0());
				metadata.put("bbox_x1", bbox.getX1());
				metadata.put("bbox_y1", bbox.getY1());
			});
			return new Document(element.getText(), metadata);
		}).toList();
	}

	private List<Document> mapPagesToDocuments(List<PageContent> pages, Map<String, Object> baseMetadata) {
		return pages.stream().map(page -> {
			Map<String, Object> metadata = new LinkedHashMap<>(baseMetadata);
			metadata.put("page", page.pageNumber());
			return new Document(page.content(), metadata);
		}).toList();
	}

	/**
	 * Builds the base metadata map applied to every output document. Metadata is
	 * layered in priority order: format-specific pass-through (lowest), explicit
	 * extraction fields, user-supplied additional metadata (highest).
	 */
	private Map<String, Object> buildBaseMetadata(ExtractionResult result) {
		Map<String, Object> metadata = new LinkedHashMap<>();
		Metadata extractionMetadata = result.getMetadata();

		// 1. Format-specific pass-through (lowest priority among structured fields)
		addFormatSpecificMetadata(metadata, extractionMetadata);

		// 2. Explicit fields from ExtractionResult and Metadata
		metadata.put("source", resolveSource());
		metadata.put("mime_type", result.getMimeType());
		metadata.put("page_count", result.getPageCount());

		List<String> detectedLanguages = result.getDetectedLanguages();
		metadata.put("detected_languages", detectedLanguages != null ? String.join(", ", detectedLanguages) : "");

		result.getQualityScore().ifPresent(qs -> metadata.put("quality_score", qs));

		extractionMetadata.getTitle().ifPresent(v -> metadata.put("title", v));
		extractionMetadata.getSubject().ifPresent(v -> metadata.put("subject", v));
		extractionMetadata.getAuthors().ifPresent(v -> metadata.put("authors", String.join(", ", v)));
		extractionMetadata.getKeywords().ifPresent(v -> metadata.put("keywords", String.join(", ", v)));
		extractionMetadata.getLanguage().ifPresent(v -> metadata.put("language", v));
		extractionMetadata.getCreatedAt().ifPresent(v -> metadata.put("created_at", v));
		extractionMetadata.getModifiedAt().ifPresent(v -> metadata.put("modified_at", v));
		extractionMetadata.getCreatedBy().ifPresent(v -> metadata.put("created_by", v));
		extractionMetadata.getModifiedBy().ifPresent(v -> metadata.put("modified_by", v));
		extractionMetadata.getCategory().ifPresent(v -> metadata.put("category", v));
		extractionMetadata.getTags().ifPresent(v -> metadata.put("tags", String.join(", ", v)));
		extractionMetadata.getDocumentVersion().ifPresent(v -> metadata.put("document_version", v));
		extractionMetadata.getAbstractText().ifPresent(v -> metadata.put("abstract_text", v));
		extractionMetadata.getOutputFormat().ifPresent(v -> metadata.put("output_format", v));

		List<?> tables = result.getTables();
		metadata.put("table_count", tables != null ? tables.size() : 0);
		if (tables != null && !tables.isEmpty()) {
			metadata.put("tables", toJson(tables));
		}

		result.getExtractedKeywords().ifPresent(ek -> metadata.put("extracted_keywords", toJson(ek)));
		result.getProcessingWarnings().ifPresent(pw -> metadata.put("processing_warnings", toJson(pw)));

		// 3. User-supplied additional metadata (highest priority)
		metadata.putAll(additionalMetadata);

		return metadata;
	}

	/**
	 * Passes through format-specific metadata from the extraction result.
	 * Primitives are added directly; complex types (lists, maps) are serialized to
	 * JSON strings.
	 */
	private void addFormatSpecificMetadata(Map<String, Object> metadata, Metadata extractionMetadata) {
		Map<String, Object> additional = extractionMetadata.getAdditional();
		if (additional == null) {
			return;
		}
		for (Map.Entry<String, Object> entry : additional.entrySet()) {
			Object value = entry.getValue();
			if (value instanceof String || value instanceof Integer || value instanceof Long || value instanceof Float
					|| value instanceof Double || value instanceof Boolean) {
				metadata.put(entry.getKey(), value);
			} else if (value instanceof List || value instanceof Map) {
				metadata.put(entry.getKey(), toJson(value));
			}
		}
	}

	private static String toJson(Object value) {
		try {
			return OBJECT_MAPPER.writeValueAsString(value);
		} catch (JsonProcessingException e) {
			throw new RuntimeException("Failed to serialize to JSON", e);
		}
	}
}
