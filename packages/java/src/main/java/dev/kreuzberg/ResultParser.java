package dev.kreuzberg;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Optional;

final class ResultParser {
	private static final ObjectMapper MAPPER = new ObjectMapper()
			.setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
			.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

	private static final TypeReference<List<Table>> TABLE_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<String>> STRING_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<Chunk>> CHUNK_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<ExtractedImage>> IMAGE_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<Element>> ELEMENT_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<OcrElement>> OCR_ELEMENT_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<PageContent>> PAGE_CONTENT_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<ExtractedKeyword>> KEYWORD_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<ProcessingWarning>> WARNING_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<PdfAnnotation>> ANNOTATION_LIST = new TypeReference<>() {
	};
	private static final TypeReference<List<Uri>> URI_LIST = new TypeReference<>() {
	};
	private static final TypeReference<Map<String, Object>> METADATA_MAP = new TypeReference<>() {
	};
	private static final TypeReference<EmbeddingPreset> EMBEDDING_PRESET = new TypeReference<>() {
	};
	private static final TypeReference<PageStructure> PAGE_STRUCTURE = new TypeReference<>() {
	};
	private static final TypeReference<DjotContent> DJOT_CONTENT = new TypeReference<>() {
	};
	private static final TypeReference<DocumentStructure> DOCUMENT_STRUCTURE = new TypeReference<>() {
	};
	private static final TypeReference<float[][]> FLOAT_ARRAYS = new TypeReference<>() {
	};

	private ResultParser() {
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String djotContentJson) throws KreuzbergException {
		return parse(content, mimeType, tablesJson, detectedLanguagesJson, metadataJson, chunksJson, imagesJson,
				pagesJson, pageStructureJson, elementsJson, null, djotContentJson, null, null, null);
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String ocrElementsJson, String djotContentJson, String language, String date,
			String subject) throws KreuzbergException {
		return parse(content, mimeType, tablesJson, detectedLanguagesJson, metadataJson, chunksJson, imagesJson,
				pagesJson, pageStructureJson, elementsJson, ocrElementsJson, djotContentJson, language, date, subject,
				null);
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String ocrElementsJson, String djotContentJson, String language, String date,
			String subject, String documentStructureJson) throws KreuzbergException {
		return parse(content, mimeType, tablesJson, detectedLanguagesJson, metadataJson, chunksJson, imagesJson,
				pagesJson, pageStructureJson, elementsJson, ocrElementsJson, djotContentJson, language, date, subject,
				documentStructureJson, null, null, null);
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String ocrElementsJson, String djotContentJson, String language, String date,
			String subject, String documentStructureJson, String extractedKeywordsJson, String qualityScoreStr,
			String processingWarningsJson) throws KreuzbergException {
		return parse(content, mimeType, tablesJson, detectedLanguagesJson, metadataJson, chunksJson, imagesJson,
				pagesJson, pageStructureJson, elementsJson, ocrElementsJson, djotContentJson, language, date, subject,
				documentStructureJson, extractedKeywordsJson, qualityScoreStr, processingWarningsJson, null);
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String ocrElementsJson, String djotContentJson, String language, String date,
			String subject, String documentStructureJson, String extractedKeywordsJson, String qualityScoreStr,
			String processingWarningsJson, String annotationsJson) throws KreuzbergException {
		return parse(content, mimeType, tablesJson, detectedLanguagesJson, metadataJson, chunksJson, imagesJson,
				pagesJson, pageStructureJson, elementsJson, ocrElementsJson, djotContentJson, language, date, subject,
				documentStructureJson, extractedKeywordsJson, qualityScoreStr, processingWarningsJson, annotationsJson,
				null);
	}

	static ExtractionResult parse(String content, String mimeType, String tablesJson, String detectedLanguagesJson,
			String metadataJson, String chunksJson, String imagesJson, String pagesJson, String pageStructureJson,
			String elementsJson, String ocrElementsJson, String djotContentJson, String language, String date,
			String subject, String documentStructureJson, String extractedKeywordsJson, String qualityScoreStr,
			String processingWarningsJson, String annotationsJson, String urisJson) throws KreuzbergException {
		try {
			Map<String, Object> metadata = decode(metadataJson, METADATA_MAP, Collections.emptyMap());
			List<Table> tables = decode(tablesJson, TABLE_LIST, List.of());
			List<String> detectedLanguages = decode(detectedLanguagesJson, STRING_LIST, List.of());
			List<Chunk> chunks = decode(chunksJson, CHUNK_LIST, List.of());
			List<ExtractedImage> images = decode(imagesJson, IMAGE_LIST, List.of());
			List<PageContent> pages = decode(pagesJson, PAGE_CONTENT_LIST, List.of());
			PageStructure pageStructure = decode(pageStructureJson, PAGE_STRUCTURE, null);
			List<Element> elements = decode(elementsJson, ELEMENT_LIST, List.of());
			List<OcrElement> ocrElements = decode(ocrElementsJson, OCR_ELEMENT_LIST, List.of());
			DjotContent djotContent = decode(djotContentJson, DJOT_CONTENT, null);
			DocumentStructure documentStructure = decode(documentStructureJson, DOCUMENT_STRUCTURE, null);
			List<ExtractedKeyword> extractedKeywords = decode(extractedKeywordsJson, KEYWORD_LIST, null);
			Double qualityScore = qualityScoreStr != null && !qualityScoreStr.isBlank()
					? Double.valueOf(qualityScoreStr)
					: null;
			List<ProcessingWarning> processingWarnings = decode(processingWarningsJson, WARNING_LIST, null);
			List<PdfAnnotation> annotations = decode(annotationsJson, ANNOTATION_LIST, null);
			List<Uri> uris = decode(urisJson, URI_LIST, null);

			// Build Metadata with FFI-provided language, date, and subject if available
			Metadata metadataObj = buildMetadata(metadata, language, date, subject);

			return new ExtractionResult(content != null ? content : "", mimeType != null ? mimeType : "", metadataObj,
					tables, detectedLanguages, chunks, images, pages, pageStructure, elements, ocrElements, djotContent,
					documentStructure, extractedKeywords, qualityScore, processingWarnings, annotations, uris);
		} catch (Exception e) {
			throw new KreuzbergException("Failed to parse extraction result", e);
		}
	}

	private static final java.util.Set<String> KNOWN_METADATA_KEYS = java.util.Set.of("title", "subject", "language",
			"created", "modified", "created_at", "modified_at", "created_by", "modified_by", "authors", "keywords",
			"pages", "image_preprocessing", "json_schema", "error", "category", "tags", "document_version",
			"abstract_text", "output_format", "extraction_duration_ms");

	private static Metadata buildMetadata(Map<String, Object> metadataMap, String language, String date,
			String subject) {
		// Extract metadata fields, preferring FFI values over map values
		String title = getStringFromMap(metadataMap, "title");
		String actualSubject = subject != null ? subject : getStringFromMap(metadataMap, "subject");
		String actualLanguage = language != null ? language : getStringFromMap(metadataMap, "language");
		String createdAt = getStringFromMap(metadataMap, "created");
		String modifiedAt = date != null ? date : getStringFromMap(metadataMap, "modified");
		String createdBy = getStringFromMap(metadataMap, "created_by");
		String modifiedBy = getStringFromMap(metadataMap, "modified_by");

		List<String> authors = getStringListFromMap(metadataMap, "authors");
		List<String> keywords = getStringListFromMap(metadataMap, "keywords");
		// Convert the raw map to PageStructure using Jackson (cannot direct cast from
		// LinkedHashMap)
		PageStructure pages = convertValue(metadataMap.get("pages"), PageStructure.class);
		@SuppressWarnings("unchecked")
		Map<String, Object> imagePreprocessing = (Map<String, Object>) metadataMap.get("image_preprocessing");
		@SuppressWarnings("unchecked")
		Map<String, Object> jsonSchema = (Map<String, Object>) metadataMap.get("json_schema");
		@SuppressWarnings("unchecked")
		Map<String, Object> error = (Map<String, Object>) metadataMap.get("error");

		String category = getStringFromMap(metadataMap, "category");
		List<String> tags = getStringListFromMap(metadataMap, "tags");
		String documentVersion = getStringFromMap(metadataMap, "document_version");
		String abstractText = getStringFromMap(metadataMap, "abstract_text");
		String outputFormat = getStringFromMap(metadataMap, "output_format");
		Object extractionDurationRaw = metadataMap.get("extraction_duration_ms");
		Long extractionDurationMs = extractionDurationRaw instanceof Number
				? ((Number) extractionDurationRaw).longValue()
				: null;

		Metadata metadata = new Metadata(title != null ? Optional.of(title) : Optional.empty(),
				actualSubject != null ? Optional.of(actualSubject) : Optional.empty(),
				authors != null ? Optional.of(authors) : Optional.empty(),
				keywords != null ? Optional.of(keywords) : Optional.empty(),
				actualLanguage != null ? Optional.of(actualLanguage) : Optional.empty(),
				createdAt != null ? Optional.of(createdAt) : Optional.empty(),
				modifiedAt != null ? Optional.of(modifiedAt) : Optional.empty(),
				createdBy != null ? Optional.of(createdBy) : Optional.empty(),
				modifiedBy != null ? Optional.of(modifiedBy) : Optional.empty(),
				pages != null ? Optional.of(pages) : Optional.empty(),
				imagePreprocessing != null ? Optional.of(imagePreprocessing) : Optional.empty(),
				jsonSchema != null ? Optional.of(jsonSchema) : Optional.empty(),
				error != null ? Optional.of(error) : Optional.empty(),
				category != null ? Optional.of(category) : Optional.empty(),
				tags != null ? Optional.of(tags) : Optional.empty(),
				documentVersion != null ? Optional.of(documentVersion) : Optional.empty(),
				abstractText != null ? Optional.of(abstractText) : Optional.empty(),
				outputFormat != null ? Optional.of(outputFormat) : Optional.empty(),
				extractionDurationMs != null ? Optional.of(extractionDurationMs) : Optional.empty());

		// Add format-specific and other additional fields not handled above
		for (Map.Entry<String, Object> entry : metadataMap.entrySet()) {
			if (!KNOWN_METADATA_KEYS.contains(entry.getKey())) {
				metadata.setAdditionalProperty(entry.getKey(), entry.getValue());
			}
		}

		return metadata;
	}

	private static String getStringFromMap(Map<String, Object> map, String key) {
		Object value = map.get(key);
		return value instanceof String ? (String) value : null;
	}

	@SuppressWarnings({"unchecked", "PMD.ReturnEmptyCollectionRatherThanNull"})
	private static List<String> getStringListFromMap(Map<String, Object> map, String key) {
		Object value = map.get(key);
		if (value instanceof List) {
			return (List<String>) value;
		}
		if (value instanceof String) {
			return List.of((String) value);
		}
		// null signals "field absent" → mapped to Optional.empty() by caller
		return null;
	}

	private static <T> T convertValue(Object value, Class<T> targetType) {
		if (value == null) {
			return null;
		}
		try {
			return MAPPER.convertValue(value, targetType);
		} catch (IllegalArgumentException e) {
			// Jackson conversion failed (e.g., validation error in constructor)
			// Return null instead of propagating the exception
			return null;
		}
	}

	private static <T> T decode(String json, TypeReference<T> type, T fallback) throws Exception {
		if (json == null || json.isBlank()) {
			return fallback;
		}
		return MAPPER.readValue(json, type);
	}

	static String toJson(Map<String, Object> map) throws Exception {
		return MAPPER.writeValueAsString(map);
	}

	static EmbeddingPreset parseEmbeddingPreset(String json) throws Exception {
		if (json == null || json.isBlank()) {
			return null;
		}
		return MAPPER.readValue(json, EMBEDDING_PRESET);
	}

	static ExtractionResult fromJson(String json) throws KreuzbergException {
		if (json == null || json.isBlank()) {
			throw new KreuzbergException("Result JSON cannot be null or empty");
		}
		try {
			WireExtractionResult wire = MAPPER.readValue(json, WireExtractionResult.class);
			return new ExtractionResult(wire.content != null ? wire.content : "",
					wire.mimeType != null ? wire.mimeType : "",
					wire.metadata != null ? wire.metadata : Metadata.empty(),
					wire.tables != null ? wire.tables : List.of(),
					wire.detectedLanguages != null ? wire.detectedLanguages : List.of(),
					wire.chunks != null ? wire.chunks : List.of(), wire.images != null ? wire.images : List.of(),
					wire.pages != null ? wire.pages : List.of(), wire.pageStructure,
					wire.elements != null ? wire.elements : List.of(),
					wire.ocrElements != null ? wire.ocrElements : List.of(), wire.djotContent, wire.document,
					wire.extractedKeywords, wire.qualityScore, wire.processingWarnings, wire.annotations,
					wire.uris, wire.children);
		} catch (Exception e) {
			throw new KreuzbergException("Failed to parse result JSON", e);
		}
	}

	static String toJson(ExtractionResult result) throws Exception {
		WireExtractionResult wire = new WireExtractionResult(result.getContent(), result.getMimeType(),
				result.getMetadata(), result.getTables(), result.getDetectedLanguages(), result.getChunks(),
				result.getImages(), result.getPages(), result.getPageStructure().orElse(null), result.getElements(),
				result.getOcrElements(), result.getDjotContent().orElse(null),
				result.getDocumentStructure().orElse(null), result.getExtractedKeywords().orElse(null),
				result.getQualityScore().orElse(null), result.getProcessingWarnings().orElse(null),
				result.getAnnotations().orElse(null), result.getUris().orElse(null),
				result.getChildren().orElse(null));
		return MAPPER.writeValueAsString(wire);
	}

	static String toJsonValue(Object value) throws Exception {
		return MAPPER.writeValueAsString(value);
	}

	static List<String> parseStringList(String json) throws Exception {
		return MAPPER.readValue(json, STRING_LIST);
	}

	static float[][] parseFloatArrays(String json) throws Exception {
		if (json == null || json.isBlank()) {
			return new float[0][0];
		}
		return MAPPER.readValue(json, FLOAT_ARRAYS);
	}

	private static final class WireExtractionResult {
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
		private final DjotContent djotContent;
		private final DocumentStructure document;
		private final List<ExtractedKeyword> extractedKeywords;
		private final Double qualityScore;
		private final List<ProcessingWarning> processingWarnings;
		private final List<PdfAnnotation> annotations;
		private final List<Uri> uris;
		private final List<ArchiveEntry> children;

		WireExtractionResult(@JsonProperty("content") String content, @JsonProperty("mime_type") String mimeType,
				@JsonProperty("metadata") Metadata metadata, @JsonProperty("tables") List<Table> tables,
				@JsonProperty("detected_languages") List<String> detectedLanguages,
				@JsonProperty("chunks") List<Chunk> chunks, @JsonProperty("images") List<ExtractedImage> images,
				@JsonProperty("pages") List<PageContent> pages,
				@JsonProperty("page_structure") PageStructure pageStructure,
				@JsonProperty("elements") List<Element> elements,
				@JsonProperty("ocr_elements") List<OcrElement> ocrElements,
				@JsonProperty("djot_content") DjotContent djotContent,
				@JsonProperty("document") DocumentStructure document,
				@JsonProperty("extracted_keywords") List<ExtractedKeyword> extractedKeywords,
				@JsonProperty("quality_score") Double qualityScore,
				@JsonProperty("processing_warnings") List<ProcessingWarning> processingWarnings,
				@JsonProperty("annotations") List<PdfAnnotation> annotations,
				@JsonProperty("uris") List<Uri> uris,
				@JsonProperty("children") List<ArchiveEntry> children) {
			this.content = content;
			this.mimeType = mimeType;
			this.metadata = metadata;
			this.tables = tables;
			this.detectedLanguages = detectedLanguages;
			this.chunks = chunks;
			this.images = images;
			this.pages = pages;
			this.pageStructure = pageStructure;
			this.elements = elements;
			this.ocrElements = ocrElements;
			this.djotContent = djotContent;
			this.document = document;
			this.extractedKeywords = extractedKeywords;
			this.qualityScore = qualityScore;
			this.processingWarnings = processingWarnings;
			this.annotations = annotations;
			this.uris = uris;
			this.children = children;
		}
	}
}
