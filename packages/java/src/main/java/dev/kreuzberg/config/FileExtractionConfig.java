package dev.kreuzberg.config;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Per-file extraction configuration overrides for batch processing.
 *
 * <p>
 * All fields are nullable — null means "use the batch-level default." This type
 * is used with
 * {@link dev.kreuzberg.Kreuzberg#batchExtractFiles(java.util.List, ExtractionConfig, java.util.List)}
 * and
 * {@link dev.kreuzberg.Kreuzberg#batchExtractBytes(java.util.List, ExtractionConfig, java.util.List)}
 * to allow heterogeneous extraction settings within a single batch.
 *
 * <p>
 * Batch-level concerns (caching, concurrency, acceleration, security) are
 * excluded and can only be set on the batch-level {@link ExtractionConfig}.
 *
 * @since 4.6.0
 */
public final class FileExtractionConfig {
	private static final ObjectMapper MAPPER = new ObjectMapper()
			.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

	private final Boolean enableQualityProcessing;
	private final OcrConfig ocr;
	private final Boolean forceOcr;
	private final List<Long> forceOcrPages;
	private final ChunkingConfig chunking;
	private final ImageExtractionConfig images;
	private final PdfConfig pdfOptions;
	private final TokenReductionConfig tokenReduction;
	private final LanguageDetectionConfig languageDetection;
	private final PageConfig pages;
	private final KeywordConfig keywords;
	private final PostProcessorConfig postprocessor;
	private final HtmlOptions htmlOptions;
	private final LayoutDetectionConfig layout;
	private final Boolean includeDocumentStructure;
	private final String outputFormat;
	private final String resultFormat;
	private final Long timeoutSecs;

	private FileExtractionConfig(Builder builder) {
		this.enableQualityProcessing = builder.enableQualityProcessing;
		this.ocr = builder.ocr;
		this.forceOcr = builder.forceOcr;
		this.forceOcrPages = builder.forceOcrPages;
		this.chunking = builder.chunking;
		this.images = builder.images;
		this.pdfOptions = builder.pdfOptions;
		this.tokenReduction = builder.tokenReduction;
		this.languageDetection = builder.languageDetection;
		this.pages = builder.pages;
		this.keywords = builder.keywords;
		this.postprocessor = builder.postprocessor;
		this.htmlOptions = builder.htmlOptions;
		this.layout = builder.layout;
		this.includeDocumentStructure = builder.includeDocumentStructure;
		this.outputFormat = builder.outputFormat;
		this.resultFormat = builder.resultFormat;
		this.timeoutSecs = builder.timeoutSecs;
	}

	public static Builder builder() {
		return new Builder();
	}

	public Boolean getEnableQualityProcessing() {
		return enableQualityProcessing;
	}

	public OcrConfig getOcr() {
		return ocr;
	}

	public Boolean getForceOcr() {
		return forceOcr;
	}

	public List<Long> getForceOcrPages() {
		return forceOcrPages;
	}

	public ChunkingConfig getChunking() {
		return chunking;
	}

	public ImageExtractionConfig getImages() {
		return images;
	}

	public PdfConfig getPdfOptions() {
		return pdfOptions;
	}

	public TokenReductionConfig getTokenReduction() {
		return tokenReduction;
	}

	public LanguageDetectionConfig getLanguageDetection() {
		return languageDetection;
	}

	public PageConfig getPages() {
		return pages;
	}

	public KeywordConfig getKeywords() {
		return keywords;
	}

	public PostProcessorConfig getPostprocessor() {
		return postprocessor;
	}

	public HtmlOptions getHtmlOptions() {
		return htmlOptions;
	}

	public LayoutDetectionConfig getLayout() {
		return layout;
	}

	public Boolean getIncludeDocumentStructure() {
		return includeDocumentStructure;
	}

	public String getOutputFormat() {
		return outputFormat;
	}

	public String getResultFormat() {
		return resultFormat;
	}

	/**
	 * Get the per-file extraction timeout in seconds.
	 *
	 * <p>
	 * When the timeout is exceeded, the extraction for this file is cancelled and an error is returned.
	 *
	 * @return the extraction timeout in seconds, or null if not set
	 */
	public Long getTimeoutSecs() {
		return timeoutSecs;
	}

	/**
	 * Serialize to a JSON-compatible map, omitting null values.
	 *
	 * @return map of non-null configuration fields
	 */
	public Map<String, Object> toMap() {
		Map<String, Object> map = new HashMap<>();
		if (enableQualityProcessing != null) {
			map.put("enable_quality_processing", enableQualityProcessing);
		}
		if (ocr != null) {
			map.put("ocr", ocr.toMap());
		}
		if (forceOcr != null) {
			map.put("force_ocr", forceOcr);
		}
		if (forceOcrPages != null) {
			map.put("force_ocr_pages", forceOcrPages);
		}
		if (chunking != null) {
			map.put("chunking", chunking.toMap());
		}
		if (images != null) {
			map.put("images", images.toMap());
		}
		if (pdfOptions != null) {
			map.put("pdf_options", pdfOptions.toMap());
		}
		if (tokenReduction != null) {
			map.put("token_reduction", tokenReduction.toMap());
		}
		if (languageDetection != null) {
			map.put("language_detection", languageDetection.toMap());
		}
		if (pages != null) {
			map.put("pages", pages.toMap());
		}
		if (keywords != null) {
			map.put("keywords", keywords.toMap());
		}
		if (postprocessor != null) {
			map.put("postprocessor", postprocessor.toMap());
		}
		if (htmlOptions != null) {
			map.put("html_options", htmlOptions.toMap());
		}
		if (layout != null) {
			map.put("layout", layout.toMap());
		}
		if (includeDocumentStructure != null) {
			map.put("include_document_structure", includeDocumentStructure);
		}
		if (outputFormat != null) {
			map.put("output_format", outputFormat);
		}
		if (resultFormat != null) {
			map.put("result_format", resultFormat);
		}
		if (timeoutSecs != null) {
			map.put("timeout_secs", timeoutSecs);
		}
		return map;
	}

	/**
	 * Serialize to JSON string.
	 *
	 * @return JSON representation
	 */
	public String toJson() {
		try {
			return MAPPER.writeValueAsString(toMap());
		} catch (Exception e) {
			throw new RuntimeException("Failed to serialize FileExtractionConfig to JSON", e);
		}
	}

	public static final class Builder {
		private Boolean enableQualityProcessing;
		private OcrConfig ocr;
		private Boolean forceOcr;
		private List<Long> forceOcrPages;
		private ChunkingConfig chunking;
		private ImageExtractionConfig images;
		private PdfConfig pdfOptions;
		private TokenReductionConfig tokenReduction;
		private LanguageDetectionConfig languageDetection;
		private PageConfig pages;
		private KeywordConfig keywords;
		private PostProcessorConfig postprocessor;
		private HtmlOptions htmlOptions;
		private LayoutDetectionConfig layout;
		private Boolean includeDocumentStructure;
		private String outputFormat;
		private String resultFormat;
		private Long timeoutSecs;

		public Builder enableQualityProcessing(Boolean enableQualityProcessing) {
			this.enableQualityProcessing = enableQualityProcessing;
			return this;
		}

		public Builder ocr(OcrConfig ocr) {
			this.ocr = ocr;
			return this;
		}

		public Builder forceOcrPages(List<Long> forceOcrPages) {
			this.forceOcrPages = forceOcrPages;
			return this;
		}

		public Builder forceOcr(Boolean forceOcr) {
			this.forceOcr = forceOcr;
			return this;
		}

		public Builder chunking(ChunkingConfig chunking) {
			this.chunking = chunking;
			return this;
		}

		public Builder images(ImageExtractionConfig images) {
			this.images = images;
			return this;
		}

		public Builder pdfOptions(PdfConfig pdfOptions) {
			this.pdfOptions = pdfOptions;
			return this;
		}

		public Builder tokenReduction(TokenReductionConfig tokenReduction) {
			this.tokenReduction = tokenReduction;
			return this;
		}

		public Builder languageDetection(LanguageDetectionConfig languageDetection) {
			this.languageDetection = languageDetection;
			return this;
		}

		public Builder pages(PageConfig pages) {
			this.pages = pages;
			return this;
		}

		public Builder keywords(KeywordConfig keywords) {
			this.keywords = keywords;
			return this;
		}

		public Builder postprocessor(PostProcessorConfig postprocessor) {
			this.postprocessor = postprocessor;
			return this;
		}

		public Builder htmlOptions(HtmlOptions htmlOptions) {
			this.htmlOptions = htmlOptions;
			return this;
		}

		public Builder layout(LayoutDetectionConfig layout) {
			this.layout = layout;
			return this;
		}

		public Builder includeDocumentStructure(Boolean includeDocumentStructure) {
			this.includeDocumentStructure = includeDocumentStructure;
			return this;
		}

		public Builder outputFormat(String outputFormat) {
			this.outputFormat = outputFormat;
			return this;
		}

		public Builder resultFormat(String resultFormat) {
			this.resultFormat = resultFormat;
			return this;
		}

		/**
		 * Set the per-file extraction timeout in seconds.
		 *
		 * <p>
		 * When the timeout is exceeded, the extraction for this file is cancelled and an error is returned.
		 *
		 * @param timeoutSecs
		 *            timeout in seconds
		 * @return this builder for chaining
		 */
		public Builder timeoutSecs(Long timeoutSecs) {
			this.timeoutSecs = timeoutSecs;
			return this;
		}

		public FileExtractionConfig build() {
			return new FileExtractionConfig(this);
		}
	}
}
