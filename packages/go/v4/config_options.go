package kreuzberg

// This file implements the functional options pattern for all Kreuzberg configuration types.
// Instead of using pointer helper functions (BoolPtr, StringPtr, etc.), use the option
// constructors defined below with NewXxxConfig functions.
//
// Example usage:
//
//	config := NewExtractionConfig(
//		WithUseCache(false),
//		WithEnableQualityProcessing(true),
//		WithOCR(
//			WithOCRBackend("tesseract"),
//			WithOCRLanguage("eng"),
//		),
//	)

// ============================================================================
// ExtractionConfig Options
// ============================================================================

// NewExtractionConfig creates a new ExtractionConfig with the given options.
// This is the idiomatic way to build ExtractionConfig instances.
func NewExtractionConfig(opts ...ExtractionOption) *ExtractionConfig {
	cfg := &ExtractionConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithUseCache sets whether caching is enabled.
func WithUseCache(enabled bool) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.UseCache = &enabled
	}
}

// WithEnableQualityProcessing sets whether quality processing is enabled.
func WithEnableQualityProcessing(enabled bool) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.EnableQualityProcessing = &enabled
	}
}

// WithOCR sets the OCR configuration with functional options.
func WithOCR(opts ...OCROption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Ocr = NewOCRConfig(opts...)
	}
}

// WithForceOCR sets whether OCR should be forced.
func WithForceOCR(enabled bool) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.ForceOcr = &enabled
	}
}

// WithDisableOCR sets whether OCR should be disabled entirely.
// When enabled, image files that would normally require OCR return empty content
// instead of errors.
func WithDisableOCR(disabled bool) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.DisableOcr = &disabled
	}
}

// WithChunking sets the chunking configuration with functional options.
func WithChunking(opts ...ChunkingOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Chunking = NewChunkingConfig(opts...)
	}
}

// WithImages sets the image extraction configuration with functional options.
func WithImages(opts ...ImageExtractionOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Images = NewImageExtractionConfig(opts...)
	}
}

// WithPdfOptions sets the PDF configuration with functional options.
func WithPdfOptions(opts ...PdfOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.PdfOptions = NewPdfConfig(opts...)
	}
}

// WithTokenReduction sets the token reduction configuration with functional options.
func WithTokenReduction(opts ...TokenReductionOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.TokenReduction = NewTokenReductionConfig(opts...)
	}
}

// WithLanguageDetection sets the language detection configuration with functional options.
func WithLanguageDetection(opts ...LanguageDetectionOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.LanguageDetection = NewLanguageDetectionConfig(opts...)
	}
}

// WithKeywords sets the keyword configuration with functional options.
func WithKeywords(opts ...KeywordOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Keywords = NewKeywordConfig(opts...)
	}
}

// WithPostprocessor sets the postprocessor configuration with functional options.
func WithPostprocessor(opts ...PostProcessorOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Postprocessor = NewPostProcessorConfig(opts...)
	}
}

// WithHTMLOptions sets the HTML conversion configuration with functional options.
func WithHTMLOptions(opts ...HTMLConversionOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.HTMLOptions = NewHTMLConversionOptions(opts...)
	}
}

// WithLayoutDetection sets the layout detection configuration with functional options.
func WithLayoutDetection(opts ...LayoutDetectionOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Layout = NewLayoutDetectionConfig(opts...)
	}
}

// WithContentFilter sets the content filter configuration with functional options.
func WithContentFilter(opts ...ContentFilterOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.ContentFilter = NewContentFilterConfig(opts...)
	}
}

// WithHTMLOutput sets the HTML output configuration with functional options.
func WithHTMLOutput(opts ...HTMLOutputOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.HTMLOutput = NewHTMLOutputConfig(opts...)
	}
}

// WithPages sets the page configuration with functional options.
func WithPages(opts ...PageOption) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.Pages = NewPageConfig(opts...)
	}
}

// WithMaxConcurrentExtractions sets the maximum concurrent extractions.
func WithMaxConcurrentExtractions(limit int) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.MaxConcurrentExtractions = &limit
	}
}

// WithIncludeDocumentStructure sets whether to include the document structure tree.
func WithIncludeDocumentStructure(include bool) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.IncludeDocumentStructure = &include
	}
}

// WithOutputFormat sets the content output format.
// Options: "plain", "markdown", "djot", "html"
func WithOutputFormat(format string) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.OutputFormat = format
	}
}

// WithResultFormat sets the result structure format.
// Options: "unified", "element_based"
func WithResultFormat(format string) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.ResultFormat = format
	}
}

// WithCacheNamespace sets the cache namespace for tenant isolation.
// When set, cache keys are scoped to this namespace so that different
// tenants' cached results do not collide.
func WithCacheNamespace(ns string) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.CacheNamespace = &ns
	}
}

// WithCacheTTLSecs sets the per-request cache TTL in seconds.
// Overrides the server default TTL for this extraction request.
func WithCacheTTLSecs(secs uint64) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.CacheTTLSecs = &secs
	}
}

// WithExtractionTimeoutSecs sets the per-request extraction timeout in seconds.
// When the timeout is exceeded, the extraction is canceled and an error is returned.
func WithExtractionTimeoutSecs(secs uint64) ExtractionOption {
	return func(c *ExtractionConfig) {
		c.ExtractionTimeoutSecs = &secs
	}
}

// ============================================================================
// OCRConfig Options
// ============================================================================

// NewOCRConfig creates a new OCRConfig with the given options.
func NewOCRConfig(opts ...OCROption) *OCRConfig {
	cfg := &OCRConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithOCRBackend sets the OCR backend (e.g., "tesseract").
func WithOCRBackend(backend string) OCROption {
	return func(c *OCRConfig) {
		c.Backend = backend
	}
}

// WithOCRLanguage sets the OCR language code.
func WithOCRLanguage(lang string) OCROption {
	return func(c *OCRConfig) {
		c.Language = &lang
	}
}

// WithTesseract sets the Tesseract configuration with functional options.
func WithTesseract(opts ...TesseractOption) OCROption {
	return func(c *OCRConfig) {
		c.Tesseract = NewTesseractConfig(opts...)
	}
}

// ============================================================================
// TesseractConfig Options
// ============================================================================

// NewTesseractConfig creates a new TesseractConfig with the given options.
func NewTesseractConfig(opts ...TesseractOption) *TesseractConfig {
	cfg := &TesseractConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithTesseractLanguage sets the Tesseract language code.
func WithTesseractLanguage(lang string) TesseractOption {
	return func(c *TesseractConfig) {
		c.Language = lang
	}
}

// WithTesseractPSM sets the Tesseract page segmentation mode.
func WithTesseractPSM(psm int) TesseractOption {
	return func(c *TesseractConfig) {
		c.PSM = &psm
	}
}

// WithTesseractOutputFormat sets the output format.
func WithTesseractOutputFormat(format string) TesseractOption {
	return func(c *TesseractConfig) {
		c.OutputFormat = format
	}
}

// WithTesseractOEM sets the OCR engine mode.
func WithTesseractOEM(oem int) TesseractOption {
	return func(c *TesseractConfig) {
		c.OEM = &oem
	}
}

// WithTesseractMinConfidence sets the minimum confidence threshold.
func WithTesseractMinConfidence(confidence float64) TesseractOption {
	return func(c *TesseractConfig) {
		c.MinConfidence = &confidence
	}
}

// WithTesseractPreprocessing sets the image preprocessing configuration with functional options.
func WithTesseractPreprocessing(opts ...ImagePreprocessingOption) TesseractOption {
	return func(c *TesseractConfig) {
		c.Preprocessing = NewImagePreprocessingConfig(opts...)
	}
}

// WithTesseractEnableTableDetection enables table detection.
func WithTesseractEnableTableDetection(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.EnableTableDetection = &enabled
	}
}

// WithTesseractTableMinConfidence sets the table minimum confidence.
func WithTesseractTableMinConfidence(confidence float64) TesseractOption {
	return func(c *TesseractConfig) {
		c.TableMinConfidence = &confidence
	}
}

// WithTesseractTableColumnThreshold sets the table column threshold.
func WithTesseractTableColumnThreshold(threshold int) TesseractOption {
	return func(c *TesseractConfig) {
		c.TableColumnThreshold = &threshold
	}
}

// WithTesseractTableRowThresholdRatio sets the table row threshold ratio.
func WithTesseractTableRowThresholdRatio(ratio float64) TesseractOption {
	return func(c *TesseractConfig) {
		c.TableRowThresholdRatio = &ratio
	}
}

// WithTesseractUseCache enables caching for Tesseract.
func WithTesseractUseCache(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.UseCache = &enabled
	}
}

// WithTesseractClassifyUsePreAdaptedTemplates enables pre-adapted templates for classification.
func WithTesseractClassifyUsePreAdaptedTemplates(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.ClassifyUsePreAdaptedTemplates = &enabled
	}
}

// WithTesseractLanguageModelNgramOn enables language model n-gram.
func WithTesseractLanguageModelNgramOn(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.LanguageModelNgramOn = &enabled
	}
}

// WithTesseractTesseditDontBlkrejGoodWds sets the tessedit_dont_blkrej_good_wds parameter.
func WithTesseractTesseditDontBlkrejGoodWds(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditDontBlkrejGoodWds = &enabled
	}
}

// WithTesseractTesseditDontRowrejGoodWds sets the tessedit_dont_rowrej_good_wds parameter.
func WithTesseractTesseditDontRowrejGoodWds(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditDontRowrejGoodWds = &enabled
	}
}

// WithTesseractTesseditEnableDictCorrection enables dictionary correction.
func WithTesseractTesseditEnableDictCorrection(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditEnableDictCorrection = &enabled
	}
}

// WithTesseractTesseditCharWhitelist sets the character whitelist.
func WithTesseractTesseditCharWhitelist(whitelist string) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditCharWhitelist = whitelist
	}
}

// WithTesseractTesseditCharBlacklist sets the character blacklist.
func WithTesseractTesseditCharBlacklist(blacklist string) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditCharBlacklist = blacklist
	}
}

// WithTesseractTesseditUsePrimaryParamsModel enables primary params model.
func WithTesseractTesseditUsePrimaryParamsModel(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.TesseditUsePrimaryParamsModel = &enabled
	}
}

// WithTesseractTextordSpaceSizeIsVariable sets the textord_space_size_is_variable parameter.
func WithTesseractTextordSpaceSizeIsVariable(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.TextordSpaceSizeIsVariable = &enabled
	}
}

// WithTesseractThresholdingMethod sets the thresholding method.
func WithTesseractThresholdingMethod(enabled bool) TesseractOption {
	return func(c *TesseractConfig) {
		c.ThresholdingMethod = &enabled
	}
}

// ============================================================================
// ImagePreprocessingConfig Options
// ============================================================================

// NewImagePreprocessingConfig creates a new ImagePreprocessingConfig with the given options.
func NewImagePreprocessingConfig(opts ...ImagePreprocessingOption) *ImagePreprocessingConfig {
	cfg := &ImagePreprocessingConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithTargetDPI sets the target DPI for image preprocessing.
func WithTargetDPI(dpi int) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.TargetDPI = &dpi
	}
}

// WithAutoRotate enables automatic rotation.
func WithAutoRotate(enabled bool) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.AutoRotate = &enabled
	}
}

// WithDeskew enables deskewing.
func WithDeskew(enabled bool) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.Deskew = &enabled
	}
}

// WithDenoise enables denoising.
func WithDenoise(enabled bool) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.Denoise = &enabled
	}
}

// WithContrastEnhance enables contrast enhancement.
func WithContrastEnhance(enabled bool) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.ContrastEnhance = &enabled
	}
}

// WithBinarizationMode sets the binarization method.
func WithBinarizationMode(mode string) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.BinarizationMode = mode
	}
}

// WithInvertColors enables color inversion.
func WithInvertColors(enabled bool) ImagePreprocessingOption {
	return func(c *ImagePreprocessingConfig) {
		c.InvertColors = &enabled
	}
}

// ============================================================================
// ChunkingConfig Options
// ============================================================================

// NewChunkingConfig creates a new ChunkingConfig with the given options.
func NewChunkingConfig(opts ...ChunkingOption) *ChunkingConfig {
	cfg := &ChunkingConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithMaxChars sets the maximum number of characters per chunk.
func WithMaxChars(limit int) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.MaxChars = &limit
	}
}

// WithMaxOverlap sets the maximum overlap between chunks.
func WithMaxOverlap(overlap int) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.MaxOverlap = &overlap
	}
}

// WithChunkSize sets the chunk size.
func WithChunkSize(size int) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.ChunkSize = &size
	}
}

// WithChunkOverlap sets the chunk overlap.
func WithChunkOverlap(overlap int) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.ChunkOverlap = &overlap
	}
}

// WithChunkingPreset sets the chunking preset.
func WithChunkingPreset(preset string) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.Preset = &preset
	}
}

// WithChunkerType sets the chunker type: "text" (default), "markdown", "yaml", or "semantic".
// Set to "semantic" for topic-aware chunking that works out of the box with sensible defaults
// (max_chars=1000, overlap=200, topic_threshold=0.75). No other parameters needed.
func WithChunkerType(t string) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.ChunkerType = &t
	}
}

// WithTopicThreshold sets the cosine similarity threshold for semantic topic
// detection. Optional, defaults to 0.75. Only used with chunker_type "semantic"
// and embeddings enabled. Range: 0.0-1.0. Rarely needs tuning.
func WithTopicThreshold(threshold float64) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.TopicThreshold = &threshold
	}
}

// WithChunkingEnabled sets whether chunking is enabled.
func WithChunkingEnabled(enabled bool) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.Enabled = &enabled
	}
}

// WithChunkSizingCharacters sets chunk sizing to character-based (default).
func WithChunkSizingCharacters() ChunkingOption {
	return func(c *ChunkingConfig) {
		c.Sizing = &ChunkSizingConfig{Type: "characters"}
	}
}

// WithChunkSizingTokenizer sets chunk sizing to token-based using a HuggingFace tokenizer.
func WithChunkSizingTokenizer(model string) ChunkingOption {
	return func(c *ChunkingConfig) {
		c.Sizing = &ChunkSizingConfig{Type: "tokenizer", Model: model}
	}
}

// ============================================================================
// ImageExtractionConfig Options
// ============================================================================

// NewImageExtractionConfig creates a new ImageExtractionConfig with the given options.
func NewImageExtractionConfig(opts ...ImageExtractionOption) *ImageExtractionConfig {
	cfg := &ImageExtractionConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithExtractImages enables image extraction.
func WithExtractImages(enabled bool) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.ExtractImages = &enabled
	}
}

// WithImageTargetDPI sets the target DPI for extracted images.
func WithImageTargetDPI(dpi int) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.TargetDPI = &dpi
	}
}

// WithMaxImageDimension sets the maximum image dimension.
func WithMaxImageDimension(dim int) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.MaxImageDimension = &dim
	}
}

// WithAutoAdjustDPI enables automatic DPI adjustment.
func WithAutoAdjustDPI(enabled bool) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.AutoAdjustDPI = &enabled
	}
}

// WithMinDPI sets the minimum DPI.
func WithMinDPI(dpi int) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.MinDPI = &dpi
	}
}

// WithMaxDPI sets the maximum DPI.
func WithMaxDPI(dpi int) ImageExtractionOption {
	return func(c *ImageExtractionConfig) {
		c.MaxDPI = &dpi
	}
}

// ============================================================================
// FontConfig Options
// ============================================================================

// NewFontConfig creates a new FontConfig with the given options.
func NewFontConfig(opts ...FontConfigOption) *FontConfig {
	cfg := &FontConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithFontConfigEnabled sets whether font config is enabled.
func WithFontConfigEnabled(enabled bool) FontConfigOption {
	return func(c *FontConfig) {
		c.Enabled = enabled
	}
}

// WithCustomFontDirs sets custom font directories.
func WithCustomFontDirs(dirs []string) FontConfigOption {
	return func(c *FontConfig) {
		c.CustomFontDirs = dirs
	}
}

// ============================================================================
// PdfConfig Options
// ============================================================================

// NewPdfConfig creates a new PdfConfig with the given options.
func NewPdfConfig(opts ...PdfOption) *PdfConfig {
	cfg := &PdfConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithPdfExtractImages enables image extraction in PDFs.
func WithPdfExtractImages(enabled bool) PdfOption {
	return func(c *PdfConfig) {
		c.ExtractImages = &enabled
	}
}

// WithPdfPasswords sets PDF passwords for encrypted documents.
func WithPdfPasswords(passwords []string) PdfOption {
	return func(c *PdfConfig) {
		c.Passwords = passwords
	}
}

// WithPdfExtractMetadata enables metadata extraction.
func WithPdfExtractMetadata(enabled bool) PdfOption {
	return func(c *PdfConfig) {
		c.ExtractMetadata = &enabled
	}
}

// WithPdfFontConfig sets the font configuration with functional options.
func WithPdfFontConfig(opts ...FontConfigOption) PdfOption {
	return func(c *PdfConfig) {
		c.FontConfig = NewFontConfig(opts...)
	}
}

// ============================================================================
// TokenReductionConfig Options
// ============================================================================

// NewHierarchyConfig creates a new HierarchyConfig with the given options.
func NewHierarchyConfig(opts ...HierarchyOption) *HierarchyConfig {
	cfg := &HierarchyConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithHierarchyEnabled enables hierarchy extraction.
func WithHierarchyEnabled(enabled bool) HierarchyOption {
	return func(c *HierarchyConfig) {
		c.Enabled = &enabled
	}
}

// WithKClusters sets the number of font size clusters (2-10).
func WithKClusters(k int) HierarchyOption {
	return func(c *HierarchyConfig) {
		c.KClusters = &k
	}
}

// WithIncludeBbox sets whether to include bounding box information.
func WithIncludeBbox(include bool) HierarchyOption {
	return func(c *HierarchyConfig) {
		c.IncludeBbox = &include
	}
}

// WithOcrCoverageThreshold sets the OCR coverage threshold (0.0-1.0).
func WithOcrCoverageThreshold(threshold float64) HierarchyOption {
	return func(c *HierarchyConfig) {
		c.OcrCoverageThreshold = &threshold
	}
}

// ============================================================================
// TokenReductionConfig Options
// ============================================================================

// NewTokenReductionConfig creates a new TokenReductionConfig with the given options.
func NewTokenReductionConfig(opts ...TokenReductionOption) *TokenReductionConfig {
	cfg := &TokenReductionConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithTokenReductionMode sets the token reduction mode.
func WithTokenReductionMode(mode string) TokenReductionOption {
	return func(c *TokenReductionConfig) {
		c.Mode = mode
	}
}

// WithPreserveImportantWords enables preservation of important words.
func WithPreserveImportantWords(enabled bool) TokenReductionOption {
	return func(c *TokenReductionConfig) {
		c.PreserveImportantWords = &enabled
	}
}

// ============================================================================
// LanguageDetectionConfig Options
// ============================================================================

// NewLanguageDetectionConfig creates a new LanguageDetectionConfig with the given options.
func NewLanguageDetectionConfig(opts ...LanguageDetectionOption) *LanguageDetectionConfig {
	cfg := &LanguageDetectionConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithLanguageDetectionEnabled enables language detection.
func WithLanguageDetectionEnabled(enabled bool) LanguageDetectionOption {
	return func(c *LanguageDetectionConfig) {
		c.Enabled = &enabled
	}
}

// WithLanguageDetectionMinConfidence sets the minimum confidence threshold.
func WithLanguageDetectionMinConfidence(confidence float64) LanguageDetectionOption {
	return func(c *LanguageDetectionConfig) {
		c.MinConfidence = &confidence
	}
}

// WithDetectMultiple enables detection of multiple languages.
func WithDetectMultiple(enabled bool) LanguageDetectionOption {
	return func(c *LanguageDetectionConfig) {
		c.DetectMultiple = &enabled
	}
}

// ============================================================================
// LayoutDetectionConfig Options
// ============================================================================

// NewLayoutDetectionConfig creates a new LayoutDetectionConfig with the given options.
func NewLayoutDetectionConfig(opts ...LayoutDetectionOption) *LayoutDetectionConfig {
	cfg := &LayoutDetectionConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithLayoutConfidenceThreshold sets the confidence threshold for layout detection.
func WithLayoutConfidenceThreshold(threshold float32) LayoutDetectionOption {
	return func(c *LayoutDetectionConfig) {
		c.ConfidenceThreshold = &threshold
	}
}

// WithLayoutApplyHeuristics sets whether to apply heuristics for layout detection.
func WithLayoutApplyHeuristics(apply bool) LayoutDetectionOption {
	return func(c *LayoutDetectionConfig) {
		c.ApplyHeuristics = &apply
	}
}

// WithLayoutTableModel sets the table model for layout detection.
// Valid values: "tatr", "slanet_wired", "slanet_wireless", "slanet_plus", "slanet_auto".
func WithLayoutTableModel(m string) LayoutDetectionOption {
	return func(c *LayoutDetectionConfig) {
		c.TableModel = &m
	}
}

// WithLayoutAcceleration sets the ONNX Runtime execution provider for layout detection.
func WithLayoutAcceleration(config *AccelerationConfig) LayoutDetectionOption {
	return func(c *LayoutDetectionConfig) {
		c.Acceleration = config
	}
}

// ============================================================================
// PostProcessorConfig Options
// ============================================================================

// NewPostProcessorConfig creates a new PostProcessorConfig with the given options.
func NewPostProcessorConfig(opts ...PostProcessorOption) *PostProcessorConfig {
	cfg := &PostProcessorConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithPostProcessorEnabled enables post-processing.
func WithPostProcessorEnabled(enabled bool) PostProcessorOption {
	return func(c *PostProcessorConfig) {
		c.Enabled = &enabled
	}
}

// WithEnabledProcessors sets the list of enabled processors.
func WithEnabledProcessors(processors []string) PostProcessorOption {
	return func(c *PostProcessorConfig) {
		c.EnabledProcessors = processors
	}
}

// WithDisabledProcessors sets the list of disabled processors.
func WithDisabledProcessors(processors []string) PostProcessorOption {
	return func(c *PostProcessorConfig) {
		c.DisabledProcessors = processors
	}
}

// ============================================================================
// EmbeddingModelType Options
// ============================================================================

// NewEmbeddingModelType creates a new EmbeddingModelType with the given options.
func NewEmbeddingModelType(opts ...EmbeddingModelTypeOption) *EmbeddingModelType {
	cfg := &EmbeddingModelType{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithEmbeddingModelType sets the embedding model type.
func WithEmbeddingModelType(modelType string) EmbeddingModelTypeOption {
	return func(c *EmbeddingModelType) {
		c.Type = modelType
	}
}

// WithEmbeddingModelName sets the embedding model name.
func WithEmbeddingModelName(name string) EmbeddingModelTypeOption {
	return func(c *EmbeddingModelType) {
		c.Name = name
	}
}

// WithEmbeddingModelValue sets the embedding model identifier.
func WithEmbeddingModelValue(model string) EmbeddingModelTypeOption {
	return func(c *EmbeddingModelType) {
		c.Model = model
	}
}

// WithEmbeddingModelID sets the embedding model ID.
func WithEmbeddingModelID(id string) EmbeddingModelTypeOption {
	return func(c *EmbeddingModelType) {
		c.ModelID = id
	}
}

// WithEmbeddingDimensions sets the embedding dimensions.
func WithEmbeddingDimensions(dimensions int) EmbeddingModelTypeOption {
	return func(c *EmbeddingModelType) {
		c.Dimensions = &dimensions
	}
}

// ============================================================================
// EmbeddingConfig Options
// ============================================================================

// NewEmbeddingConfig creates a new EmbeddingConfig with the given options.
func NewEmbeddingConfig(opts ...EmbeddingOption) *EmbeddingConfig {
	// Provide default model (balanced preset) if not specified
	cfg := &EmbeddingConfig{
		Model: &EmbeddingModelType{
			Type: "preset",
			Name: "balanced",
		},
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithEmbeddingModel sets the embedding model configuration with functional options.
func WithEmbeddingModel(opts ...EmbeddingModelTypeOption) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.Model = NewEmbeddingModelType(opts...)
	}
}

// WithEmbeddingNormalize enables embedding normalization.
func WithEmbeddingNormalize(enabled bool) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.Normalize = &enabled
	}
}

// WithEmbeddingBatchSize sets the batch size for embedding generation.
func WithEmbeddingBatchSize(size int) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.BatchSize = &size
	}
}

// WithShowDownloadProgress enables download progress display.
func WithShowDownloadProgress(enabled bool) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.ShowDownloadProgress = &enabled
	}
}

// WithCacheDir sets the cache directory for embeddings.
func WithCacheDir(dir string) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.CacheDir = &dir
	}
}

// WithEmbeddingAcceleration sets the ONNX Runtime execution provider for embedding generation.
func WithEmbeddingAcceleration(config *AccelerationConfig) EmbeddingOption {
	return func(c *EmbeddingConfig) {
		c.Acceleration = config
	}
}

// ============================================================================
// KeywordConfig Options
// ============================================================================

// NewKeywordConfig creates a new KeywordConfig with the given options.
func NewKeywordConfig(opts ...KeywordOption) *KeywordConfig {
	// Provide default values matching Rust defaults
	minScore := 0.0
	maxKeywords := 10
	ngramRange := [2]int{1, 3}
	language := "en"

	cfg := &KeywordConfig{
		MinScore:    &minScore,
		MaxKeywords: &maxKeywords,
		NgramRange:  &ngramRange,
		Language:    &language,
	}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithKeywordAlgorithm sets the keyword extraction algorithm.
func WithKeywordAlgorithm(algorithm string) KeywordOption {
	return func(c *KeywordConfig) {
		c.Algorithm = algorithm
	}
}

// WithMaxKeywords sets the maximum number of keywords to extract.
func WithMaxKeywords(limit int) KeywordOption {
	return func(c *KeywordConfig) {
		c.MaxKeywords = &limit
	}
}

// WithKeywordMinScore sets the minimum score for keywords.
func WithKeywordMinScore(score float64) KeywordOption {
	return func(c *KeywordConfig) {
		c.MinScore = &score
	}
}

// WithNgramRange sets the n-gram range for keyword extraction.
func WithNgramRange(minVal, maxVal int) KeywordOption {
	return func(c *KeywordConfig) {
		c.NgramRange = &[2]int{minVal, maxVal}
	}
}

// WithKeywordLanguage sets the language for keyword extraction.
func WithKeywordLanguage(lang string) KeywordOption {
	return func(c *KeywordConfig) {
		c.Language = &lang
	}
}

// WithYakeParams sets the YAKE-specific parameters with functional options.
func WithYakeParams(opts ...YakeParamsOption) KeywordOption {
	return func(c *KeywordConfig) {
		c.Yake = NewYakeParams(opts...)
	}
}

// WithRakeParams sets the RAKE-specific parameters with functional options.
func WithRakeParams(opts ...RakeParamsOption) KeywordOption {
	return func(c *KeywordConfig) {
		c.Rake = NewRakeParams(opts...)
	}
}

// ============================================================================
// YakeParams Options
// ============================================================================

// NewYakeParams creates a new YakeParams with the given options.
func NewYakeParams(opts ...YakeParamsOption) *YakeParams {
	cfg := &YakeParams{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithYakeWindowSize sets the YAKE window size.
func WithYakeWindowSize(size int) YakeParamsOption {
	return func(c *YakeParams) {
		c.WindowSize = &size
	}
}

// ============================================================================
// RakeParams Options
// ============================================================================

// NewRakeParams creates a new RakeParams with the given options.
func NewRakeParams(opts ...RakeParamsOption) *RakeParams {
	cfg := &RakeParams{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithRakeMinWordLength sets the minimum word length for RAKE.
func WithRakeMinWordLength(length int) RakeParamsOption {
	return func(c *RakeParams) {
		c.MinWordLength = &length
	}
}

// WithRakeMaxWordsPerPhrase sets the maximum words per phrase for RAKE.
func WithRakeMaxWordsPerPhrase(maxWords int) RakeParamsOption {
	return func(c *RakeParams) {
		c.MaxWordsPerPhrase = &maxWords
	}
}

// ============================================================================
// HTMLPreprocessingOptions Options
// ============================================================================

// NewHTMLPreprocessingOptions creates a new HTMLPreprocessingOptions with the given options.
func NewHTMLPreprocessingOptions(opts ...HTMLPreprocessingOption) *HTMLPreprocessingOptions {
	cfg := &HTMLPreprocessingOptions{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithHTMLPreprocessingEnabled enables HTML preprocessing.
func WithHTMLPreprocessingEnabled(enabled bool) HTMLPreprocessingOption {
	return func(c *HTMLPreprocessingOptions) {
		c.Enabled = &enabled
	}
}

// WithHTMLPreprocessingPreset sets the HTML preprocessing preset.
func WithHTMLPreprocessingPreset(preset string) HTMLPreprocessingOption {
	return func(c *HTMLPreprocessingOptions) {
		c.Preset = &preset
	}
}

// WithRemoveNavigation enables removal of navigation elements.
func WithRemoveNavigation(enabled bool) HTMLPreprocessingOption {
	return func(c *HTMLPreprocessingOptions) {
		c.RemoveNavigation = &enabled
	}
}

// WithRemoveForms enables removal of form elements.
func WithRemoveForms(enabled bool) HTMLPreprocessingOption {
	return func(c *HTMLPreprocessingOptions) {
		c.RemoveForms = &enabled
	}
}

// ============================================================================
// HTMLConversionOptions Options
// ============================================================================

// NewHTMLConversionOptions creates a new HTMLConversionOptions with the given options.
func NewHTMLConversionOptions(opts ...HTMLConversionOption) *HTMLConversionOptions {
	cfg := &HTMLConversionOptions{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithHeadingStyle sets the heading style for HTML conversion.
func WithHeadingStyle(style string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.HeadingStyle = &style
	}
}

// WithListIndentType sets the list indent type.
func WithListIndentType(indentType string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.ListIndentType = &indentType
	}
}

// WithListIndentWidth sets the list indent width.
func WithListIndentWidth(width int) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.ListIndentWidth = &width
	}
}

// WithBullets sets the bullet style.
func WithBullets(bullets string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Bullets = &bullets
	}
}

// WithStrongEmSymbol sets the strong/emphasis symbol.
func WithStrongEmSymbol(symbol string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.StrongEmSymbol = &symbol
	}
}

// WithEscapeAsterisks enables asterisk escaping.
func WithEscapeAsterisks(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.EscapeAsterisks = &enabled
	}
}

// WithEscapeUnderscores enables underscore escaping.
func WithEscapeUnderscores(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.EscapeUnderscores = &enabled
	}
}

// WithEscapeMisc enables miscellaneous character escaping.
func WithEscapeMisc(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.EscapeMisc = &enabled
	}
}

// WithEscapeASCII enables ASCII character escaping.
func WithEscapeASCII(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.EscapeASCII = &enabled
	}
}

// WithCodeLanguage sets the code language for highlighting.
func WithCodeLanguage(lang string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.CodeLanguage = &lang
	}
}

// WithAutolinks enables automatic links.
func WithAutolinks(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Autolinks = &enabled
	}
}

// WithDefaultTitle enables default title generation.
func WithDefaultTitle(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.DefaultTitle = &enabled
	}
}

// WithBrInTables enables <br> tags in tables.
func WithBrInTables(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.BrInTables = &enabled
	}
}

// WithHocrSpatialTables enables HOCR spatial tables.
func WithHocrSpatialTables(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.HocrSpatialTables = &enabled
	}
}

// WithHighlightStyle sets the syntax highlight style.
func WithHighlightStyle(style string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.HighlightStyle = &style
	}
}

// WithExtractMetadata enables metadata extraction in HTML conversion.
func WithExtractMetadata(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.ExtractMetadata = &enabled
	}
}

// WithWhitespaceMode sets the whitespace handling mode.
func WithWhitespaceMode(mode string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.WhitespaceMode = &mode
	}
}

// WithStripNewlines enables newline stripping.
func WithStripNewlines(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.StripNewlines = &enabled
	}
}

// WithWrap enables text wrapping.
func WithWrap(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Wrap = &enabled
	}
}

// WithWrapWidth sets the text wrap width.
func WithWrapWidth(width int) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.WrapWidth = &width
	}
}

// WithConvertAsInline enables inline HTML conversion.
func WithConvertAsInline(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.ConvertAsInline = &enabled
	}
}

// WithSubSymbol sets the subscript symbol.
func WithSubSymbol(symbol string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.SubSymbol = &symbol
	}
}

// WithSupSymbol sets the superscript symbol.
func WithSupSymbol(symbol string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.SupSymbol = &symbol
	}
}

// WithNewlineStyle sets the newline style.
func WithNewlineStyle(style string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.NewlineStyle = &style
	}
}

// WithCodeBlockStyle sets the code block style.
func WithCodeBlockStyle(style string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.CodeBlockStyle = &style
	}
}

// WithKeepInlineImagesIn sets the formats to keep inline images in.
func WithKeepInlineImagesIn(formats []string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.KeepInlineImagesIn = formats
	}
}

// WithEncoding sets the encoding for HTML conversion.
func WithEncoding(encoding string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Encoding = &encoding
	}
}

// WithDebug enables debug mode for HTML conversion.
func WithDebug(enabled bool) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Debug = &enabled
	}
}

// WithStripTags sets the tags to strip.
func WithStripTags(tags []string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.StripTags = tags
	}
}

// WithPreserveTags sets the tags to preserve.
func WithPreserveTags(tags []string) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.PreserveTags = tags
	}
}

// WithHTMLPreprocessing sets the HTML preprocessing configuration with functional options.
func WithHTMLPreprocessing(opts ...HTMLPreprocessingOption) HTMLConversionOption {
	return func(c *HTMLConversionOptions) {
		c.Preprocessing = NewHTMLPreprocessingOptions(opts...)
	}
}

// ============================================================================
// PageConfig Options
// ============================================================================

// NewPageConfig creates a new PageConfig with the given options.
func NewPageConfig(opts ...PageOption) *PageConfig {
	cfg := &PageConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithExtractPages enables page extraction.
func WithExtractPages(enabled bool) PageOption {
	return func(c *PageConfig) {
		c.ExtractPages = &enabled
	}
}

// WithInsertPageMarkers enables insertion of page markers.
func WithInsertPageMarkers(enabled bool) PageOption {
	return func(c *PageConfig) {
		c.InsertPageMarkers = &enabled
	}
}

// WithMarkerFormat sets the page marker format.
func WithMarkerFormat(format string) PageOption {
	return func(c *PageConfig) {
		c.MarkerFormat = &format
	}
}

// ============================================================================
// ContentFilterConfig Options
// ============================================================================

// NewContentFilterConfig creates a new ContentFilterConfig with the given options.
func NewContentFilterConfig(opts ...ContentFilterOption) *ContentFilterConfig {
	cfg := &ContentFilterConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithIncludeHeaders sets whether to include running headers in extraction output.
func WithIncludeHeaders(enabled bool) ContentFilterOption {
	return func(c *ContentFilterConfig) {
		c.IncludeHeaders = &enabled
	}
}

// WithIncludeFooters sets whether to include running footers in extraction output.
func WithIncludeFooters(enabled bool) ContentFilterOption {
	return func(c *ContentFilterConfig) {
		c.IncludeFooters = &enabled
	}
}

// WithStripRepeatingText sets whether to enable cross-page repeating text detection and removal.
func WithStripRepeatingText(enabled bool) ContentFilterOption {
	return func(c *ContentFilterConfig) {
		c.StripRepeatingText = &enabled
	}
}

// WithIncludeWatermarks sets whether to include watermark text in extraction output.
func WithIncludeWatermarks(enabled bool) ContentFilterOption {
	return func(c *ContentFilterConfig) {
		c.IncludeWatermarks = &enabled
	}
}

// NewHTMLOutputConfig creates an HTMLOutputConfig from functional options.
func NewHTMLOutputConfig(opts ...HTMLOutputOption) *HTMLOutputConfig {
	cfg := &HTMLOutputConfig{}
	for _, opt := range opts {
		opt(cfg)
	}
	return cfg
}

// WithHTMLOutputCSS sets the inline CSS string for HTML output.
func WithHTMLOutputCSS(css string) HTMLOutputOption {
	return func(c *HTMLOutputConfig) {
		c.CSS = &css
	}
}

// WithHTMLOutputCSSFile sets the path to a CSS file for HTML output.
func WithHTMLOutputCSSFile(cssFile string) HTMLOutputOption {
	return func(c *HTMLOutputConfig) {
		c.CSSFile = &cssFile
	}
}

// WithHTMLOutputTheme sets the built-in HTML theme (default, github, dark, light, unstyled).
func WithHTMLOutputTheme(theme string) HTMLOutputOption {
	return func(c *HTMLOutputConfig) {
		c.Theme = theme
	}
}

// WithHTMLOutputClassPrefix sets the CSS class prefix for HTML output.
func WithHTMLOutputClassPrefix(prefix string) HTMLOutputOption {
	return func(c *HTMLOutputConfig) {
		c.ClassPrefix = prefix
	}
}

// WithHTMLOutputEmbedCSS sets whether to embed CSS in the HTML output.
func WithHTMLOutputEmbedCSS(embed bool) HTMLOutputOption {
	return func(c *HTMLOutputConfig) {
		c.EmbedCSS = &embed
	}
}
