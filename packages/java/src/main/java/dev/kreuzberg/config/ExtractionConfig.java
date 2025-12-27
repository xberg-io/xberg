package dev.kreuzberg.config;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.KreuzbergFFI;
import java.io.IOException;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedHashMap;
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
  private static final TypeReference<Map<String, Object>> CONFIG_MAP_TYPE = new TypeReference<>() { };

  private final boolean useCache;
  private final boolean enableQualityProcessing;
  private final boolean forceOcr;
  private final boolean useCacheSet;
  private final boolean enableQualityProcessingSet;
  private final boolean forceOcrSet;
  private final OcrConfig ocr;
  private final ChunkingConfig chunking;
  private final LanguageDetectionConfig languageDetection;
  private final PdfConfig pdfOptions;
  private final ImageExtractionConfig imageExtraction;
  private final ImagePreprocessingConfig imagePreprocessing;
  private final PostProcessorConfig postprocessor;
  private final TokenReductionConfig tokenReduction;
  private final HtmlOptions htmlOptions;
  private final KeywordConfig keywords;
  private final PageConfig pages;
  private final Integer maxConcurrentExtractions;
  private final Map<String, Object> rawConfigOverride;

  private ExtractionConfig(Builder builder) {
    this.useCache = builder.useCache;
    this.enableQualityProcessing = builder.enableQualityProcessing;
    this.forceOcr = builder.forceOcr;
    this.useCacheSet = builder.useCacheSet;
    this.enableQualityProcessingSet = builder.enableQualityProcessingSet;
    this.forceOcrSet = builder.forceOcrSet;
    this.ocr = builder.ocr;
    this.chunking = builder.chunking;
    this.languageDetection = builder.languageDetection;
    this.pdfOptions = builder.pdfOptions;
    this.imageExtraction = builder.imageExtraction;
    this.imagePreprocessing = builder.imagePreprocessing;
    this.postprocessor = builder.postprocessor;
    this.tokenReduction = builder.tokenReduction;
    this.htmlOptions = builder.htmlOptions;
    this.keywords = builder.keywords;
    this.pages = builder.pages;
    this.maxConcurrentExtractions = builder.maxConcurrentExtractions;
    this.rawConfigOverride = builder.rawConfigOverride != null
        ? Collections.unmodifiableMap(new LinkedHashMap<>(builder.rawConfigOverride))
        : null;
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

  public ImagePreprocessingConfig getImagePreprocessing() {
    return imagePreprocessing;
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

  public Integer getMaxConcurrentExtractions() {
    return maxConcurrentExtractions;
  }

  /**
   * Parse configuration from JSON produced by the Rust core.
   *
   * @param json serialized configuration
   * @return parsed configuration
   * @throws KreuzbergException if parsing fails
   */
  public static ExtractionConfig fromJson(String json) throws KreuzbergException {
    try {
      Map<String, Object> raw = CONFIG_MAPPER.readValue(json, CONFIG_MAP_TYPE);
      Builder builder = builder();
      applyTopLevelOverrides(builder, raw);
      builder.rawConfigOverride(Collections.unmodifiableMap(new LinkedHashMap<>(raw)));
      return builder.build();
    } catch (IOException e) {
      throw new KreuzbergException("Failed to parse extraction config", e);
    }
  }

  /**
   * Load configuration from a file (TOML, YAML, or JSON).
   *
   * @param path path to the configuration file
   * @return parsed configuration
   * @throws KreuzbergException if loading or parsing fails
   * @since 4.0.0
   */
  public static ExtractionConfig fromFile(String path) throws KreuzbergException {
    return dev.kreuzberg.Kreuzberg.loadExtractionConfigFromFile(java.nio.file.Path.of(path));
  }

  /**
   * Discover configuration from current or parent directories.
   *
   * <p>Searches for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json
   * in the current directory and parent directories.</p>
   *
   * @return discovered configuration, or null if not found
   * @throws KreuzbergException if an error occurs during discovery
   * @since 4.0.0
   */
  public static ExtractionConfig discover() throws KreuzbergException {
    return dev.kreuzberg.Kreuzberg.discoverExtractionConfig();
  }

  /**
   * Serialize configuration to JSON string via FFI.
   *
   * Uses the Rust FFI backend for consistent serialization.
   *
   * @return JSON representation of this configuration
   * @throws KreuzbergException if serialization fails
   * @since 4.0.0
   */
  public String toJson() throws KreuzbergException {
    try (var arena = Arena.ofConfined()) {
      String jsonInput = CONFIG_MAPPER.writeValueAsString(toMap());
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
   * Supports dot notation for nested fields (e.g., "ocr.backend").
   *
   * @param fieldName the field name or path (e.g., "use_cache", "ocr.backend")
   * @return the field value as a JSON string, or empty if not found
   * @throws KreuzbergException if retrieval fails
   * @since 4.0.0
   */
  public Optional<String> getField(String fieldName) throws KreuzbergException {
    if (fieldName == null || fieldName.isEmpty()) {
      throw new IllegalArgumentException("fieldName cannot be null or empty");
    }

    try (var arena = Arena.ofConfined()) {
      String jsonInput = CONFIG_MAPPER.writeValueAsString(toMap());
      MemorySegment configJsonSeg = KreuzbergFFI.allocateCString(arena, jsonInput);

      MemorySegment configPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_CONFIG_FROM_JSON.invoke(configJsonSeg);
      if (configPtr == null || configPtr.address() == 0) {
        throw new KreuzbergException("Failed to create config for field retrieval");
      }

      try {
        MemorySegment fieldNameSeg = KreuzbergFFI.allocateCString(arena, fieldName);
        MemorySegment valueSeg = (MemorySegment)
            KreuzbergFFI.KREUZBERG_CONFIG_GET_FIELD.invoke(configPtr, fieldNameSeg);

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
   * Creates a new config with fields from the other config overriding this one.
   *
   * @param other the configuration to merge in
   * @return a new merged ExtractionConfig
   * @throws KreuzbergException if merging fails
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

  private Map<String, Object> toMap(boolean includeDefaults) {
    if (rawConfigOverride != null) {
      return rawConfigOverride;
    }
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
    if (imagePreprocessing != null) {
      map.put("image_preprocessing", imagePreprocessing.toMap());
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
    if (maxConcurrentExtractions != null) {
      map.put("max_concurrent_extractions", maxConcurrentExtractions);
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
      builder.enableQualityProcessing(asBoolean(raw.get("enable_quality_processing"), builder.enableQualityProcessing));
    }
    if (raw.containsKey("force_ocr")) {
      builder.forceOcr(asBoolean(raw.get("force_ocr"), builder.forceOcr));
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
    Map<String, Object> imagePreMap = asMap(raw.get("image_preprocessing"));
    if (imagePreMap != null) {
      builder.imagePreprocessing(ImagePreprocessingConfig.fromMap(imagePreMap));
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
    if (raw.containsKey("max_concurrent_extractions")) {
      builder.maxConcurrentExtractions(asInteger(raw.get("max_concurrent_extractions")));
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

  @SuppressWarnings("unchecked")
  private static Map<String, Object> asMap(Object value) {
    if (value instanceof Map) {
      return (Map<String, Object>) value;
    }
    return Collections.emptyMap();
  }

  public static final class Builder {
    private boolean useCache = true;
    private boolean enableQualityProcessing = false;
    private boolean forceOcr = false;
    private boolean useCacheSet = false;
    private boolean enableQualityProcessingSet = false;
    private boolean forceOcrSet = false;
    private OcrConfig ocr;
    private ChunkingConfig chunking;
    private LanguageDetectionConfig languageDetection;
    private PdfConfig pdfOptions;
    private ImageExtractionConfig imageExtraction;
    private ImagePreprocessingConfig imagePreprocessing;
    private PostProcessorConfig postprocessor;
    private TokenReductionConfig tokenReduction;
    private HtmlOptions htmlOptions;
    private KeywordConfig keywords;
    private PageConfig pages;
    private Integer maxConcurrentExtractions;
    private Map<String, Object> rawConfigOverride;

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

    public Builder imagePreprocessing(ImagePreprocessingConfig imagePreprocessing) {
      this.imagePreprocessing = imagePreprocessing;
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

    public Builder maxConcurrentExtractions(Integer maxConcurrentExtractions) {
      this.maxConcurrentExtractions = maxConcurrentExtractions;
      return this;
    }

    Builder rawConfigOverride(Map<String, Object> rawConfigOverride) {
      if (rawConfigOverride != null) {
        this.rawConfigOverride = new LinkedHashMap<>(rawConfigOverride);
      } else {
        this.rawConfigOverride = null;
      }
      return this;
    }

    public ExtractionConfig build() {
      return new ExtractionConfig(this);
    }
  }
}
