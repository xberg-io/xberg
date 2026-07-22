package dev.xberg;

import java.util.List;
import java.util.Optional;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.fasterxml.jackson.datatype.jdk8.Jdk8Module;

/**
 * Test factory that parses JSON into ExtractionResult using a properly
 * configured ObjectMapper (with Jdk8Module for Optional support).
 *
 * <p>
 * This bypasses the xberg library's internal ResultParser which lacks
 * Jdk8Module registration and fails with Jackson 2.21+.
 */
public final class ExtractionResultFactory {

	private static final ObjectMapper MAPPER = new ObjectMapper().registerModule(new Jdk8Module())
			.setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
			.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

	private ExtractionResultFactory() {
	}

	@JsonIgnoreProperties(ignoreUnknown = true)
	record Wire(@JsonProperty("content") String content, @JsonProperty("mime_type") String mimeType,
			@JsonProperty("metadata") Metadata metadata, @JsonProperty("tables") List<Table> tables,
			@JsonProperty("detected_languages") List<String> detectedLanguages,
			@JsonProperty("chunks") List<Chunk> chunks, @JsonProperty("images") List<ExtractedImage> images,
			@JsonProperty("pages") List<PageContent> pages, @JsonProperty("page_structure") PageStructure pageStructure,
			@JsonProperty("elements") List<Element> elements,
			@JsonProperty("ocr_elements") List<OcrElement> ocrElements,
			@JsonProperty("djot_content") DjotContent djotContent, @JsonProperty("document") DocumentStructure document,
			@JsonProperty("extracted_keywords") List<ExtractedKeyword> extractedKeywords,
			@JsonProperty("quality_score") Double qualityScore,
			@JsonProperty("processing_warnings") List<ProcessingWarning> processingWarnings,
			@JsonProperty("annotations") List<PdfAnnotation> annotations) {
	}

	public static ExtractionResult fromJson(String json) throws XbergException {
		try {
			Wire w = MAPPER.readValue(json, Wire.class);
			return new ExtractionResult(Optional.ofNullable(w.content).orElse(""),
					Optional.ofNullable(w.mimeType).orElse(""),
					Optional.ofNullable(w.metadata).orElseGet(Metadata::empty), nullToEmpty(w.tables),
					nullToEmpty(w.detectedLanguages), nullToEmpty(w.chunks), nullToEmpty(w.images),
					nullToEmpty(w.pages), w.pageStructure, nullToEmpty(w.elements), nullToEmpty(w.ocrElements),
					w.djotContent, w.document, nullToEmpty(w.extractedKeywords), w.qualityScore,
					nullToEmpty(w.processingWarnings), nullToEmpty(w.annotations));
		} catch (Exception e) {
			throw new XbergException("Failed to parse result JSON", e);
		}
	}

	private static <T> List<T> nullToEmpty(List<T> list) {
		return list != null ? list : List.of();
	}
}
