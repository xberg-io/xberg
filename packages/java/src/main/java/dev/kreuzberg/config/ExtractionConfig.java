package dev.kreuzberg.config;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.KreuzbergFFI;
import java.io.IOException;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;

/**
 * Main extraction configuration.
 *
 * @since 4.0.0
 */
@SuppressWarnings("PMD.AvoidCatchingThrowable")
public final class ExtractionConfig {
	private static final ObjectMapper CONFIG_MAPPER = new ObjectMapper()
			.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
	private static final TypeReference<Map<String, Object>> CONFIG_MAP_TYPE = new TypeReference<>() {
	};

	private final boolean useCache;
	private final boolean enableQualityProcessing;
	private final boolean forceOcr;
	private final boolean useCacheSet;
	private final boolean enableQualityProcessingSet;
	private final boolean forceOcrSet;
	private final String outputFormat;
	private final String resultFormat;
	private final OcrConfig ocr;
	private final ChunkingConfig chunking;
	private final LanguageDetectionConfig languageDetection;
	private final PdfConfig pdfOptions;
	private final ImageExtractionConfig imageExtraction;
	private final PostProcessorConfig postprocessor;
	private final TokenReductionConfig tokenReduction;
	private final HtmlOptions htmlOptions;
	private final KeywordConfig keywords;
	private final PageConfig pages;
	private final LayoutDetectionConfig layout;
	private final AccelerationConfig acceleration;
	private final EmailConfig email;
	private final ConcurrencyConfig concurrency;
	private final Integer maxConcurrentExtractions;
	private final SecurityLimitsConfig securityLimits;
	private final boolean includeDocumentStructure;
	private final boolean includeDocumentStructureSet;
	private final String cacheNamespace;
	private final Long cacheTtlSecs;

	private ExtractionConfig(Builder builder) {
		this.useCache = builder.useCache;
		this.enableQualityProcessing = builder.enableQualityProcessing;
		this.forceOcr = builder.forceOcr;
		this.useCacheSet = builder.useCacheSet;
		this.enableQualityProcessingSet = builder.enableQualityProcessingSet;
		this.forceOcrSet = builder.forceOcrSet;
		this.outputFormat = builder.outputFormat;
		this.resultFormat = builder.resultFormat;
		this.ocr = builder.ocr;
		this.chunking = builder.chunking;
		this.languageDetection = builder.languageDetection;
		this.pdfOptions = builder.pdfOptions;
		this.imageExtraction = builder.imageExtraction;
		this.postprocessor = builder.postprocessor;
		this.tokenReduction = builder.tokenReduction;
		this.htmlOptions = builder.htmlOptions;
		this.keywords = builder.keywords;
		this.pages = builder.pages;
		this.layout = builder.layout;
		this.acceleration = builder.acceleration;
		this.email = builder.email;
		this.concurrency = builder.concurrency;
		this.maxConcurrentExtractions = builder.maxConcurrentExtractions;
		this.securityLimits = builder.securityLimits;
		this.includeDocumentStructure = builder.includeDocumentStructure;
		this.includeDocumentStructureSet = builder.includeDocumentStructureSet;
		this.cacheNamespace = builder.cacheNamespace;
		this.cacheTtlSecs = builder.cacheTtlSecs;
	}

	public static Builder builder() {
		return new Builder();
	}

	public boolean isUseCache() {
		return useCache;
	}

	public boolean isEnableQualityProcessing() {
		return enableQualityProcessing;
	}

	public boolean isForceOcr() {
		return forceOcr;
	}

	/**
	 * Get the content output format.
	 *
	 * @return output format (plain, markdown, djot, html), or null if not set
	 * @since 4.2.0
	 */
	public String getOutputFormat() {
		return outputFormat;
	}

	/**
	 * Get the result structure format.
	 *
	 * @return result format (unified, element_based), or null if not set
	 * @since 4.2.0
	 */
	public String getResultFormat() {
		return resultFormat;
	}

	public OcrConfig getOcr() {
		return ocr;
	}

	public ChunkingConfig getChunking() {
		return chunking;
	}

	public LanguageDetectionConfig getLanguageDetection() {
		return languageDetection;
	}

	public PdfConfig getPdfOptions() {
		return pdfOptions;
	}

	public ImageExtractionConfig getImageExtraction() {
		return imageExtraction;
	}

	/**
	 * Get the image extraction configuration (alias for getImageExtraction).
	 *
	 * @return the image extraction configuration, or null if not set
	 * @since 4.2.0
	 */
	public ImageExtractionConfig getImages() {
		return imageExtraction;
	}

	public PostProcessorConfig getPostprocessor() {
		return postprocessor;
	}

	public TokenReductionConfig getTokenReduction() {
		return tokenReduction;
	}

	public HtmlOptions getHtmlOptions() {
		return htmlOptions;
	}

	public KeywordConfig getKeywords() {
		return keywords;
	}

	public PageConfig getPages() {
		return pages;
	}

	/**
	 * Get the layout detection configuration.
	 *
	 * @return the layout detection configuration, or null if not set
	 * @since 4.4.0
	 */
	public LayoutDetectionConfig getLayout() {
		return layout;
	}

	/**
	 * Get the acceleration configuration.
	 *
	 * @return the acceleration configuration, or null if not set
	 * @since 4.5.0
	 */
	public AccelerationConfig getAcceleration() {
		return acceleration;
	}

	/**
	 * Get the email extraction configuration.
	 *
	 * @return the email configuration, or null if not set
	 * @since 4.6.0
	 */
	public EmailConfig getEmail() {
		return email;
	}

	/**
	 * Get the concurrency configuration.
	 *
	 * @return the concurrency configuration, or null if not set
	 */
	public ConcurrencyConfig getConcurrency() {
		return concurrency;
	}

	public Integer getMaxConcurrentExtractions() {
		return maxConcurrentExtractions;
	}

	/**
	 * Check if document structure extraction is enabled.
	 *
	 * @return true if document structure extraction is enabled
	 * @since 4.3.0
	 */
	public boolean isIncludeDocumentStructure() {
		return includeDocumentStructure;
	}

	/**
	 * Get the cache namespace for tenant isolation.
	 *
	 * @return the cache namespace, or null if not set
	 */
	public String getCacheNamespace() {
		return cacheNamespace;
	}

	/**
	 * Get the per-request cache TTL in seconds.
	 *
	 * <p>
	 * A value of 0 means skip the cache entirely.
	 *
	 * @return the cache TTL in seconds, or null if not set
	 */
	public Long getCacheTtlSecs() {
		return cacheTtlSecs;
	}

	/**
	 * Parse configuration from JSON produced by the Rust core.
	 *
	 * @param json
	 *            serialized configuration
	 * @return parsed configuration
	 * @throws KreuzbergException
	 *             if parsing fails
	 */
	public static ExtractionConfig fromJson(String json) throws KreuzbergException {
		try {
			Map<String, Object> raw = CONFIG_MAPPER.readValue(json, CONFIG_MAP_TYPE);
			Builder builder = builder();
			applyTopLevelOverrides(builder, raw);
			return builder.build();
		} catch (IOException e) {
			throw new KreuzbergException("Failed to parse extraction config", e);
		}
	}

	/**
	 * Load configuration from a file (TOML, YAML, or JSON).
	 *
	 * @param path
	 *            path to the configuration file
	 * @return parsed configuration
	 * @throws KreuzbergException
	 *             if loading or parsing fails
	 * @since 4.0.0
	 */
	public static ExtractionConfig fromFile(String path) throws KreuzbergException {
		return dev.kreuzberg.Kreuzberg.loadExtractionConfigFromFile(java.nio.file.Path.of(path));
	}

	/**
	 * Discover configuration from current or parent directories.
	 *
	 * <p>
	 * Searches for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in the current
	 * directory and parent directories.
	 *
	 * @return Optional containing discovered configuration if found, empty Optional
	 *         otherwise
	 * @throws KreuzbergException
	 *             if an error occurs during discovery
	 * @since 4.0.0
	 */
	public static Optional<ExtractionConfig> discover() throws KreuzbergException {
		return dev.kreuzberg.Kreuzberg.discoverExtractionConfig();
	}

	/**
	 * Serialize configuration to JSON string via FFI.
	 *
	 * <p>
	 * Uses the Rust FFI backend for consistent serialization.
	 *
	 * @return JSON representation of this configuration
	 * @throws KreuzbergException
	 *             if serialization fails
	 * @since 4.0.0
	 */
	public String toJson() throws KreuzbergException {
		try (var arena = Arena.ofConfined()) {
			Map<String, Object> configMap = toMap();
			// Filter out null values before sending to Rust
			Map<String, Object> filteredMap = filterNullValues(configMap);
			String jsonInput = CONFIG_MAPPER.writeValueAsString(filteredMap);
			MemorySegment configJsonSeg = KreuzbergFFI.allocateCString(arena, jsonInput);

			MemorySegment configPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_FROM_JSON.invoke(configJsonSeg);
			if (configPtr == null || configPtr.address() == 0) {
				throw new KreuzbergException("Failed to serialize config");
			}

			try {
				MemorySegment jsonSeg = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_TO_JSON.invoke(configPtr);
				if (jsonSeg == null || jsonSeg.address() == 0) {
					throw new KreuzbergException("Failed to convert config to JSON");
				}

				String result = KreuzbergFFI.readCString(jsonSeg);
				KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(jsonSeg);
				return result;
			} finally {
				try {
					KreuzbergFFI.KREUZBERG_CONFIG_FREE.invoke(configPtr);
				} catch (Throwable ignored) {
				}
			}
		} catch (KreuzbergException e) {
			throw e;
		} catch (Throwable e) {
			throw new KreuzbergException("Failed to serialize config to JSON", e);
		}
	}

	/**
	 * Get a specific field from the configuration.
	 *
	 * <p>
	 * Supports dot notation for nested fields (e.g., "ocr.backend").
	 *
	 * @param fieldName
	 *            the field name or path (e.g., "use_cache", "ocr.backend")
	 * @return the field value as a JSON string, or empty if not found
	 * @throws KreuzbergException
	 *             if retrieval fails
	 * @since 4.0.0
	 */
	public Optional<String> getField(String fieldName) throws KreuzbergException {
		if (fieldName == null || fieldName.isEmpty()) {
			throw new IllegalArgumentException("fieldName cannot be null or empty");
		}

		try (var arena = Arena.ofConfined()) {
			Map<String, Object> configMap = toMap();
			// Filter out null values before sending to Rust
			Map<String, Object> filteredMap = filterNullValues(configMap);
			String jsonInput = CONFIG_MAPPER.writeValueAsString(filteredMap);
			MemorySegment configJsonSeg = KreuzbergFFI.allocateCString(arena, jsonInput);

			MemorySegment configPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_FROM_JSON.invoke(configJsonSeg);
			if (configPtr == null || configPtr.address() == 0) {
				throw new KreuzbergException("Failed to create config for field retrieval");
			}

			try {
				MemorySegment fieldNameSeg = KreuzbergFFI.allocateCString(arena, fieldName);
				MemorySegment valueSeg = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_GET_FIELD.invoke(configPtr,
						fieldNameSeg);

				if (valueSeg == null || valueSeg.address() == 0) {
					return Optional.empty();
				}

				String result = KreuzbergFFI.readCString(valueSeg);
				try {
					KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(valueSeg);
				} catch (Throwable ignored) {
				}
				return Optional.ofNullable(result);
			} finally {
				try {
					KreuzbergFFI.KREUZBERG_CONFIG_FREE.invoke(configPtr);
				} catch (Throwable ignored) {
				}
			}
		} catch (KreuzbergException e) {
			throw e;
		} catch (Throwable e) {
			throw new KreuzbergException("Failed to retrieve field: " + fieldName, e);
		}
	}

	/**
	 * Merge another configuration into this one.
	 *
	 * <p>
	 * Creates a new config with fields from the other config overriding this one.
	 *
	 * @param other
	 *            the configuration to merge in
	 * @return a new merged ExtractionConfig
	 * @throws KreuzbergException
	 *             if merging fails
	 * @since 4.0.0
	 */
	public ExtractionConfig merge(ExtractionConfig other) throws KreuzbergException {
		if (other == null) {
			throw new IllegalArgumentException("other config cannot be null");
		}

		try (var arena = Arena.ofConfined()) {
			String thisJson = CONFIG_MAPPER.writeValueAsString(toMap(true));
			String otherJson = CONFIG_MAPPER.writeValueAsString(other.toMap(false));

			MemorySegment thisJsonSeg = KreuzbergFFI.allocateCString(arena, thisJson);
			MemorySegment otherJsonSeg = KreuzbergFFI.allocateCString(arena, otherJson);

			MemorySegment thisPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_FROM_JSON.invoke(thisJsonSeg);
			MemorySegment otherPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_FROM_JSON.invoke(otherJsonSeg);

			if (thisPtr == null || thisPtr.address() == 0 || otherPtr == null || otherPtr.address() == 0) {
				throw new KreuzbergException("Failed to create configs for merging");
			}

			try {
				int result = (int) KreuzbergFFI.KREUZBERG_CONFIG_MERGE.invoke(thisPtr, otherPtr);
				if (result != 1) {
					throw new KreuzbergException("Config merge failed");
				}

				MemorySegment mergedJsonSeg = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_TO_JSON.invoke(thisPtr);
				if (mergedJsonSeg == null || mergedJsonSeg.address() == 0) {
					throw new KreuzbergException("Failed to serialize merged config");
				}

				String mergedJson = KreuzbergFFI.readCString(mergedJsonSeg);
				try {
					KreuzbergFFI.KREUZBERG_FREE_STRING.invoke(mergedJsonSeg);
				} catch (Throwable ignored) {
				}

				return fromJson(mergedJson);
			} finally {
				try {
					KreuzbergFFI.KREUZBERG_CONFIG_FREE.invoke(thisPtr);
				} catch (Throwable ignored) {
				}
				try {
					KreuzbergFFI.KREUZBERG_CONFIG_FREE.invoke(otherPtr);
				} catch (Throwable ignored) {
				}
			}
		} catch (KreuzbergException e) {
			throw e;
		} catch (Throwable e) {
			throw new KreuzbergException("Failed to merge configs", e);
		}
	}

	public Map<String, Object> toMap() {
		return toMap(true);
	}

	private static Map<String, Object> filterNullValues(Map<String, Object> map) {
		Map<String, Object> filtered = new HashMap<>();
		for (Map.Entry<String, Object> entry : map.entrySet()) {
			if (entry.getValue() != null) {
				filtered.put(entry.getKey(), entry.getValue());
			}
		}
		return filtered;
	}

	private Map<String, Object> toMap(boolean includeDefaults) {
		Map<String, Object> map = new HashMap<>();
		if (includeDefaults || useCacheSet) {
			map.put("use_cache", useCache);
		}
		if (includeDefaults || enableQualityProcessingSet) {
			map.put("enable_quality_processing", enableQualityProcessing);
		}
		if (includeDefaults || forceOcrSet) {
			map.put("force_ocr", forceOcr);
		}
		if (includeDefaults || includeDocumentStructureSet) {
			map.put("include_document_structure", includeDocumentStructure);
		}
		if (outputFormat != null) {
			map.put("output_format", outputFormat);
		}
		if (resultFormat != null) {
			map.put("result_format", resultFormat);
		}
		if (ocr != null) {
			map.put("ocr", ocr.toMap());
		}
		if (chunking != null) {
			map.put("chunking", chunking.toMap());
		}
		if (languageDetection != null) {
			map.put("language_detection", languageDetection.toMap());
		}
		if (pdfOptions != null) {
			map.put("pdf_options", pdfOptions.toMap());
		}
		if (imageExtraction != null) {
			map.put("images", imageExtraction.toMap());
		}
		if (postprocessor != null) {
			map.put("postprocessor", postprocessor.toMap());
		}
		if (tokenReduction != null) {
			map.put("token_reduction", tokenReduction.toMap());
		}
		if (htmlOptions != null) {
			map.put("html_options", htmlOptions.toMap());
		}
		if (keywords != null) {
			map.put("keywords", keywords.toMap());
		}
		if (pages != null) {
			map.put("pages", pages.toMap());
		}
		if (layout != null) {
			map.put("layout", layout.toMap());
		}
		if (acceleration != null) {
			map.put("acceleration", acceleration.toMap());
		}
		if (email != null) {
			map.put("email", email.toMap());
		}
		if (concurrency != null) {
			map.put("concurrency", concurrency.toMap());
		}
		if (maxConcurrentExtractions != null) {
			map.put("max_concurrent_extractions", maxConcurrentExtractions);
		}
		if (securityLimits != null) {
			map.put("security_limits", securityLimits.toMap());
		}
		if (cacheNamespace != null) {
			map.put("cache_namespace", cacheNamespace);
		}
		if (cacheTtlSecs != null) {
			map.put("cache_ttl_secs", cacheTtlSecs);
		}
		return map;
	}

	@SuppressWarnings("unchecked")
	private static void applyTopLevelOverrides(Builder builder, Map<String, Object> raw) {
		if (raw == null) {
			return;
		}
		if (raw.containsKey("use_cache")) {
			builder.useCache(asBoolean(raw.get("use_cache"), builder.useCache));
		}
		if (raw.containsKey("enable_quality_processing")) {
			builder.enableQualityProcessing(
					asBoolean(raw.get("enable_quality_processing"), builder.enableQualityProcessing));
		}
		if (raw.containsKey("force_ocr")) {
			builder.forceOcr(asBoolean(raw.get("force_ocr"), builder.forceOcr));
		}
		if (raw.containsKey("include_document_structure")) {
			builder.includeDocumentStructure(asBoolean(raw.get("include_document_structure"), false));
		}
		if (raw.containsKey("output_format")) {
			builder.outputFormat(asString(raw.get("output_format")));
		}
		if (raw.containsKey("result_format")) {
			builder.resultFormat(asString(raw.get("result_format")));
		}
		Map<String, Object> ocrMap = asMap(raw.get("ocr"));
		if (ocrMap != null) {
			builder.ocr(OcrConfig.fromMap(ocrMap));
		}
		Map<String, Object> chunkingMap = asMap(raw.get("chunking"));
		if (chunkingMap != null) {
			builder.chunking(ChunkingConfig.fromMap(chunkingMap));
		}
		Map<String, Object> languageMap = asMap(raw.get("language_detection"));
		if (languageMap != null) {
			builder.languageDetection(LanguageDetectionConfig.fromMap(languageMap));
		}
		Map<String, Object> pdfMap = asMap(raw.get("pdf_options"));
		if (pdfMap != null) {
			builder.pdfOptions(PdfConfig.fromMap(pdfMap));
		}
		Map<String, Object> imageMap = asMap(
				raw.containsKey("images") ? raw.get("images") : raw.get("image_extraction"));
		if (imageMap != null) {
			builder.imageExtraction(ImageExtractionConfig.fromMap(imageMap));
		}
		Map<String, Object> postprocessorMap = asMap(raw.get("postprocessor"));
		if (postprocessorMap != null) {
			builder.postprocessor(PostProcessorConfig.fromMap(postprocessorMap));
		}
		Map<String, Object> tokenReductionMap = asMap(raw.get("token_reduction"));
		if (tokenReductionMap != null) {
			builder.tokenReduction(TokenReductionConfig.fromMap(tokenReductionMap));
		}
		Map<String, Object> htmlMap = asMap(raw.get("html_options"));
		if (htmlMap != null) {
			builder.htmlOptions(HtmlOptions.fromMap(htmlMap));
		}
		Map<String, Object> keywordMap = asMap(raw.get("keywords"));
		if (keywordMap != null) {
			builder.keywords(KeywordConfig.fromMap(keywordMap));
		}
		Map<String, Object> pageMap = asMap(raw.get("pages"));
		if (pageMap != null) {
			builder.pages(PageConfig.fromMap(pageMap));
		}
		Map<String, Object> layoutMap = asMap(raw.get("layout"));
		if (layoutMap != null) {
			builder.layout(LayoutDetectionConfig.fromMap(layoutMap));
		}
		Map<String, Object> accelerationMap = asMap(raw.get("acceleration"));
		if (accelerationMap != null) {
			builder.acceleration(AccelerationConfig.fromMap(accelerationMap));
		}
		Map<String, Object> emailMap = asMap(raw.get("email"));
		if (emailMap != null) {
			builder.email(EmailConfig.fromMap(emailMap));
		}
		Map<String, Object> concurrencyMap = asMap(raw.get("concurrency"));
		if (concurrencyMap != null) {
			builder.concurrency(ConcurrencyConfig.fromMap(concurrencyMap));
		}
		if (raw.containsKey("max_concurrent_extractions")) {
			builder.maxConcurrentExtractions(asInteger(raw.get("max_concurrent_extractions")));
		}
		Map<String, Object> securityMap = asMap(raw.get("security_limits"));
		if (securityMap != null) {
			builder.securityLimits(SecurityLimitsConfig.fromMap(securityMap));
		}
		if (raw.containsKey("cache_namespace")) {
			builder.cacheNamespace((String) raw.get("cache_namespace"));
		}
		if (raw.containsKey("cache_ttl_secs")) {
			Object val = raw.get("cache_ttl_secs");
			if (val instanceof Number) {
				builder.cacheTtlSecs(((Number) val).longValue());
			}
		}
	}

	private static boolean asBoolean(Object value, boolean defaultValue) {
		if (value instanceof Boolean) {
			return (Boolean) value;
		}
		if (value instanceof Number) {
			return ((Number) value).intValue() != 0;
		}
		if (value instanceof String) {
			return Boolean.parseBoolean((String) value);
		}
		return defaultValue;
	}

	private static Integer asInteger(Object value) {
		if (value instanceof Number) {
			return ((Number) value).intValue();
		}
		if (value instanceof String) {
			try {
				return Integer.parseInt((String) value);
			} catch (NumberFormatException ignored) {
				return null;
			}
		}
		return null;
	}

	private static String asString(Object value) {
		if (value instanceof String) {
			return (String) value;
		}
		if (value != null) {
			return value.toString();
		}
		return null;
	}

	@SuppressWarnings({"unchecked", "PMD.ReturnEmptyCollectionRatherThanNull"})
	private static Map<String, Object> asMap(Object value) {
		if (value instanceof Map) {
			return (Map<String, Object>) value;
		}
		return null;
	}

	public static final class Builder {
		private boolean useCache = true;
		private boolean enableQualityProcessing = true;
		private boolean forceOcr = false;
		private boolean includeDocumentStructure = false;
		private boolean useCacheSet = false;
		private boolean enableQualityProcessingSet = false;
		private boolean forceOcrSet = false;
		private boolean includeDocumentStructureSet = false;
		private String outputFormat;
		private String resultFormat;
		private OcrConfig ocr;
		private ChunkingConfig chunking;
		private LanguageDetectionConfig languageDetection;
		private PdfConfig pdfOptions;
		private ImageExtractionConfig imageExtraction;
		private PostProcessorConfig postprocessor;
		private TokenReductionConfig tokenReduction;
		private HtmlOptions htmlOptions;
		private KeywordConfig keywords;
		private PageConfig pages;
		private LayoutDetectionConfig layout;
		private AccelerationConfig acceleration;
		private EmailConfig email;
		private ConcurrencyConfig concurrency;
		private Integer maxConcurrentExtractions;
		private SecurityLimitsConfig securityLimits;
		private String cacheNamespace;
		private Long cacheTtlSecs;

		private Builder() {
		}

		public Builder useCache(boolean useCache) {
			this.useCache = useCache;
			this.useCacheSet = true;
			return this;
		}

		public Builder enableQualityProcessing(boolean enableQualityProcessing) {
			this.enableQualityProcessing = enableQualityProcessing;
			this.enableQualityProcessingSet = true;
			return this;
		}

		public Builder forceOcr(boolean forceOcr) {
			this.forceOcr = forceOcr;
			this.forceOcrSet = true;
			return this;
		}

		/**
		 * Enable or disable document structure extraction.
		 *
		 * <p>
		 * When enabled, the extraction result will include a document structure
		 * representing the hierarchical tree of document nodes.
		 *
		 * @param includeDocumentStructure
		 *            true to enable document structure extraction
		 * @return this builder for chaining
		 * @since 4.3.0
		 */
		public Builder includeDocumentStructure(boolean includeDocumentStructure) {
			this.includeDocumentStructure = includeDocumentStructure;
			this.includeDocumentStructureSet = true;
			return this;
		}

		/**
		 * Set the content output format.
		 *
		 * <p>
		 * Valid formats: plain, markdown, djot, html
		 *
		 * @param format
		 *            the output format
		 * @return this builder for chaining
		 * @since 4.2.0
		 */
		public Builder outputFormat(String format) {
			this.outputFormat = format;
			return this;
		}

		/**
		 * Set the result structure format.
		 *
		 * <p>
		 * Valid formats: unified, element_based
		 *
		 * @param format
		 *            the result format
		 * @return this builder for chaining
		 * @since 4.2.0
		 */
		public Builder resultFormat(String format) {
			this.resultFormat = format;
			return this;
		}

		public Builder ocr(OcrConfig ocr) {
			this.ocr = ocr;
			return this;
		}

		public Builder chunking(ChunkingConfig chunking) {
			this.chunking = chunking;
			return this;
		}

		public Builder languageDetection(LanguageDetectionConfig languageDetection) {
			this.languageDetection = languageDetection;
			return this;
		}

		public Builder pdfOptions(PdfConfig pdfOptions) {
			this.pdfOptions = pdfOptions;
			return this;
		}

		public Builder imageExtraction(ImageExtractionConfig imageExtraction) {
			this.imageExtraction = imageExtraction;
			return this;
		}

		/**
		 * Set the image extraction configuration (alias for imageExtraction).
		 *
		 * @param images
		 *            the image extraction configuration
		 * @return this builder for chaining
		 * @since 4.2.0
		 */
		public Builder images(ImageExtractionConfig images) {
			this.imageExtraction = images;
			return this;
		}

		public Builder postprocessor(PostProcessorConfig postprocessor) {
			this.postprocessor = postprocessor;
			return this;
		}

		public Builder tokenReduction(TokenReductionConfig tokenReduction) {
			this.tokenReduction = tokenReduction;
			return this;
		}

		public Builder htmlOptions(HtmlOptions htmlOptions) {
			this.htmlOptions = htmlOptions;
			return this;
		}

		public Builder keywords(KeywordConfig keywords) {
			this.keywords = keywords;
			return this;
		}

		public Builder pages(PageConfig pages) {
			this.pages = pages;
			return this;
		}

		/**
		 * Set the layout detection configuration.
		 *
		 * @param layout
		 *            the layout detection configuration
		 * @return this builder for chaining
		 * @since 4.4.0
		 */
		public Builder layout(LayoutDetectionConfig layout) {
			this.layout = layout;
			return this;
		}

		/**
		 * Set the acceleration configuration.
		 *
		 * @param acceleration
		 *            the acceleration configuration
		 * @return this builder for chaining
		 * @since 4.5.0
		 */
		public Builder acceleration(AccelerationConfig acceleration) {
			this.acceleration = acceleration;
			return this;
		}

		/**
		 * Set the email extraction configuration.
		 *
		 * @param email
		 *            the email configuration
		 * @return this builder for chaining
		 * @since 4.6.0
		 */
		public Builder email(EmailConfig email) {
			this.email = email;
			return this;
		}

		/**
		 * Set the concurrency configuration.
		 *
		 * @param concurrency
		 *            the concurrency configuration
		 * @return this builder for chaining
		 */
		public Builder concurrency(ConcurrencyConfig concurrency) {
			this.concurrency = concurrency;
			return this;
		}

		public Builder maxConcurrentExtractions(Integer maxConcurrentExtractions) {
			this.maxConcurrentExtractions = maxConcurrentExtractions;
			return this;
		}

		public Builder securityLimits(SecurityLimitsConfig securityLimits) {
			this.securityLimits = securityLimits;
			return this;
		}

		/**
		 * Set the cache namespace for tenant isolation.
		 *
		 * @param cacheNamespace
		 *            the cache namespace string
		 * @return this builder for chaining
		 */
		public Builder cacheNamespace(String cacheNamespace) {
			this.cacheNamespace = cacheNamespace;
			return this;
		}

		/**
		 * Set the per-request cache TTL in seconds.
		 *
		 * <p>
		 * A value of 0 means skip the cache entirely.
		 *
		 * @param cacheTtlSecs
		 *            TTL in seconds
		 * @return this builder for chaining
		 */
		public Builder cacheTtlSecs(Long cacheTtlSecs) {
			this.cacheTtlSecs = cacheTtlSecs;
			return this;
		}

		public ExtractionConfig build() {
			return new ExtractionConfig(this);
		}
	}
}
