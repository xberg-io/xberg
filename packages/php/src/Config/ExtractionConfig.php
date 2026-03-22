<?php

declare(strict_types=1);

namespace Kreuzberg\Config;

/**
 * Configuration for document extraction.
 *
 * @example
 * ```php
 * use Kreuzberg\Config\ExtractionConfig;
 * use Kreuzberg\Config\OcrConfig;
 * use Kreuzberg\Config\PdfConfig;
 * use Kreuzberg\Config\ChunkingConfig;
 *
 * $config = new ExtractionConfig(
 *     ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
 *     pdf: new PdfConfig(extractImages: true),
 *     chunking: new ChunkingConfig(maxChunkSize: 1000),
 * );
 * ```
 */
readonly class ExtractionConfig
{
    public function __construct(
        /**
         * Enable caching of extraction results.
         *
         * When enabled, extraction results are cached to avoid redundant processing
         * of the same documents. Cache key is based on document content hash.
         *
         * @var bool
         * @default true
         */
        public bool $useCache = true,

        /**
         * Enable quality processing enhancements.
         *
         * When enabled, applies advanced quality improvement techniques including
         * text smoothing, error correction, and content validation to improve
         * extraction output quality.
         *
         * @var bool
         * @default true
         */
        public bool $enableQualityProcessing = true,

        /**
         * OCR configuration.
         *
         * Configures Optical Character Recognition settings for scanned documents
         * and image-based PDFs. Includes backend selection, language, and Tesseract options.
         *
         * @var OcrConfig|null
         * @default null
         */
        public ?OcrConfig $ocr = null,

        /**
         * Force OCR on all documents regardless of document type.
         *
         * When enabled, OCR will be applied even to documents that typically
         * have machine-readable text. Useful for ensuring consistent text extraction
         * quality across heterogeneous document collections.
         *
         * @var bool
         * @default false
         */
        public bool $forceOcr = false,

        /**
         * Text chunking configuration.
         *
         * Configures how extracted text is split into chunks for processing,
         * including chunk size, overlap, and boundary preservation options.
         *
         * @var ChunkingConfig|null
         * @default null
         */
        public ?ChunkingConfig $chunking = null,

        /**
         * Image extraction configuration.
         *
         * Configures image extraction parameters such as minimum dimensions
         * and whether to perform OCR on extracted images.
         *
         * @var ImageExtractionConfig|null
         * @default null
         */
        public ?ImageExtractionConfig $images = null,

        /**
         * PDF extraction configuration.
         *
         * Configures PDF-specific extraction options like image extraction,
         * metadata extraction, OCR fallback, and page range selection.
         *
         * @var PdfConfig|null
         * @default null
         */
        public ?PdfConfig $pdfOptions = null,

        /**
         * Token reduction configuration.
         *
         * Configures token reduction/optimization for extracted content.
         *
         * @var TokenReductionConfig|null
         * @default null
         */
        public ?TokenReductionConfig $tokenReduction = null,

        /**
         * Language detection configuration.
         *
         * Configures automatic language detection for document content,
         * including confidence thresholds and maximum languages to detect.
         *
         * @var LanguageDetectionConfig|null
         * @default null
         */
        public ?LanguageDetectionConfig $languageDetection = null,

        /**
         * Page extraction configuration.
         *
         * Configures page-level extraction options including page markers and format.
         *
         * @var PageConfig|null
         * @default null
         */
        public ?PageConfig $pages = null,

        /**
         * Keyword extraction configuration.
         *
         * Configures keyword extraction parameters such as maximum number of keywords
         * and minimum relevance score thresholds.
         *
         * @var KeywordConfig|null
         * @default null
         */
        public ?KeywordConfig $keywords = null,

        /**
         * Post-processor configuration.
         *
         * Configures post-processing options for extracted content.
         *
         * @var PostProcessorConfig|null
         * @default null
         */
        public ?PostProcessorConfig $postprocessor = null,

        /**
         * HTML to Markdown conversion options.
         *
         * Configures how HTML documents are converted to Markdown, including heading styles,
         * list formatting, code block styles, and preprocessing options.
         *
         * @var HtmlConversionOptions|null
         * @default null
         */
        public ?HtmlConversionOptions $htmlOptions = null,

        /**
         * Security limits for archive extraction.
         *
         * Controls maximum archive size, compression ratio, file count, and other
         * security thresholds to prevent decompression bomb attacks.
         * When null, default limits are used.
         *
         * @var SecurityLimitsConfig|null
         * @default null
         */
        public ?SecurityLimitsConfig $securityLimits = null,

        /**
         * Maximum number of concurrent extraction operations.
         *
         * Controls the degree of parallelism for batch extraction operations.
         * Higher values allow more documents to be processed concurrently but consume more resources.
         * When null, uses the Rust default.
         *
         * @var int|null
         * @default null
         */
        public ?int $maxConcurrentExtractions = null,

        /**
         * Result format for structured output.
         *
         * Specifies how results are formatted when structured output is requested.
         * Common values:
         * - 'unified': Single unified format combining all extraction results (default)
         * - 'element_based': Semantic elements for Unstructured compatibility
         *
         * @var string
         * @default 'unified'
         */
        public string $resultFormat = 'unified',

        /**
         * Output format for extracted content.
         *
         * Specifies the format for the extracted content. Common values:
         * - 'plain': Plain text format (default)
         * - 'markdown': Markdown format with basic formatting
         * - 'djot': Djot markup format
         * - 'html': HTML format with rich formatting
         *
         * @var string
         * @default 'plain'
         */
        public string $outputFormat = 'plain',

        /**
         * Include hierarchical document structure.
         *
         * When enabled, the extraction result will include a DocumentStructure
         * with a hierarchical tree of document nodes representing the semantic structure.
         *
         * @var bool
         * @default false
         */
        public bool $includeDocumentStructure = false,

        /**
         * Hardware acceleration configuration for ONNX Runtime models.
         *
         * Configures which execution provider to use for ONNX model inference,
         * enabling hardware acceleration on GPU devices (CUDA, TensorRT, CoreML).
         *
         * @var AccelerationConfig|null
         * @default null
         */
        public ?AccelerationConfig $acceleration = null,

        /**
         * Email extraction configuration.
         *
         * Configures email-specific extraction settings such as the fallback
         * code page for MSG email body decoding.
         *
         * @var EmailConfig|null
         * @default null
         */
        public ?EmailConfig $email = null,

        /**
         * Concurrency configuration for thread pool management.
         *
         * Controls the maximum number of threads used for parallel processing
         * during document extraction.
         *
         * @var ConcurrencyConfig|null
         * @default null
         */
        public ?ConcurrencyConfig $concurrency = null,

        /**
         * Cache namespace for tenant isolation.
         *
         * When set, cache keys are scoped to this namespace, enabling tenant
         * isolation so that different tenants' cached results do not collide.
         *
         * @var string|null
         * @default null
         */
        public ?string $cacheNamespace = null,

        /**
         * Per-request cache TTL in seconds.
         *
         * Overrides the default cache time-to-live for this extraction request.
         * When null, the server default TTL is used.
         *
         * @var int|null
         * @default null
         */
        public ?int $cacheTtlSecs = null,
    ) {
    }

    /**
     * Create configuration from array data.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var bool $useCache */
        $useCache = $data['use_cache'] ?? true;
        if (!is_bool($useCache)) {
            /** @var bool $useCache */
            $useCache = (bool) $useCache;
        }

        /** @var bool $enableQualityProcessing */
        $enableQualityProcessing = $data['enable_quality_processing'] ?? true;
        if (!is_bool($enableQualityProcessing)) {
            /** @var bool $enableQualityProcessing */
            $enableQualityProcessing = (bool) $enableQualityProcessing;
        }

        /** @var bool $forceOcr */
        $forceOcr = $data['force_ocr'] ?? false;
        if (!is_bool($forceOcr)) {
            /** @var bool $forceOcr */
            $forceOcr = (bool) $forceOcr;
        }

        /** @var int|null $maxConcurrentExtractions */
        $maxConcurrentExtractions = $data['max_concurrent_extractions'] ?? null;
        if ($maxConcurrentExtractions !== null && !is_int($maxConcurrentExtractions)) {
            /** @var int $maxConcurrentExtractions */
            $maxConcurrentExtractions = (int) $maxConcurrentExtractions;
        }

        /** @var string $resultFormat */
        $resultFormat = $data['result_format'] ?? 'unified';
        if (!is_string($resultFormat)) {
            /** @var string $resultFormat */
            $resultFormat = (string) $resultFormat;
        }

        /** @var string $outputFormat */
        $outputFormat = $data['output_format'] ?? 'plain';
        if (!is_string($outputFormat)) {
            /** @var string $outputFormat */
            $outputFormat = (string) $outputFormat;
        }

        $ocr = null;
        if (isset($data['ocr']) && is_array($data['ocr'])) {
            /** @var array<string, mixed> $ocrData */
            $ocrData = $data['ocr'];
            $ocr = OcrConfig::fromArray($ocrData);
        }

        $pdf = null;
        if (isset($data['pdf_options']) && is_array($data['pdf_options'])) {
            /** @var array<string, mixed> $pdfData */
            $pdfData = $data['pdf_options'];
            $pdf = PdfConfig::fromArray($pdfData);
        } elseif (isset($data['pdf']) && is_array($data['pdf'])) {
            /** @var array<string, mixed> $pdfData */
            $pdfData = $data['pdf'];
            $pdf = PdfConfig::fromArray($pdfData);
        }

        $chunking = null;
        if (isset($data['chunking']) && is_array($data['chunking'])) {
            /** @var array<string, mixed> $chunkingData */
            $chunkingData = $data['chunking'];
            $chunking = ChunkingConfig::fromArray($chunkingData);
        }

        $imageExtraction = null;
        if (isset($data['images']) && is_array($data['images'])) {
            /** @var array<string, mixed> $imageExtractionData */
            $imageExtractionData = $data['images'];
            $imageExtraction = ImageExtractionConfig::fromArray($imageExtractionData);
        } elseif (isset($data['image_extraction']) && is_array($data['image_extraction'])) {
            /** @var array<string, mixed> $imageExtractionData */
            $imageExtractionData = $data['image_extraction'];
            $imageExtraction = ImageExtractionConfig::fromArray($imageExtractionData);
        }

        $page = null;
        if (isset($data['pages']) && is_array($data['pages'])) {
            /** @var array<string, mixed> $pageData */
            $pageData = $data['pages'];
            $page = PageConfig::fromArray($pageData);
        } elseif (isset($data['page']) && is_array($data['page'])) {
            /** @var array<string, mixed> $pageData */
            $pageData = $data['page'];
            $page = PageConfig::fromArray($pageData);
        }

        $languageDetection = null;
        if (isset($data['language_detection']) && is_array($data['language_detection'])) {
            /** @var array<string, mixed> $languageDetectionData */
            $languageDetectionData = $data['language_detection'];
            $languageDetection = LanguageDetectionConfig::fromArray($languageDetectionData);
        }

        $keywords = null;
        if (isset($data['keywords']) && is_array($data['keywords'])) {
            /** @var array<string, mixed> $keywordsData */
            $keywordsData = $data['keywords'];
            $keywords = KeywordConfig::fromArray($keywordsData);
        }

        $htmlOptions = null;
        if (isset($data['html_options']) && is_array($data['html_options'])) {
            /** @var array<string, mixed> $htmlOptionsData */
            $htmlOptionsData = $data['html_options'];
            $htmlOptions = HtmlConversionOptions::fromArray($htmlOptionsData);
        }

        $postprocessor = null;
        if (isset($data['postprocessor']) && is_array($data['postprocessor'])) {
            /** @var array<string, mixed> $postprocessorData */
            $postprocessorData = $data['postprocessor'];
            $postprocessor = PostProcessorConfig::fromArray($postprocessorData);
        }

        $tokenReduction = null;
        if (isset($data['token_reduction']) && is_array($data['token_reduction'])) {
            /** @var array<string, mixed> $tokenReductionData */
            $tokenReductionData = $data['token_reduction'];
            $tokenReduction = TokenReductionConfig::fromArray($tokenReductionData);
        }

        /** @var bool $includeDocumentStructure */
        $includeDocumentStructure = $data['include_document_structure'] ?? false;
        if (!is_bool($includeDocumentStructure)) {
            /** @var bool $includeDocumentStructure */
            $includeDocumentStructure = (bool) $includeDocumentStructure;
        }

        $acceleration = null;
        if (isset($data['acceleration']) && is_array($data['acceleration'])) {
            /** @var array<string, mixed> $accelerationData */
            $accelerationData = $data['acceleration'];
            $acceleration = AccelerationConfig::fromArray($accelerationData);
        }

        $email = null;
        if (isset($data['email']) && is_array($data['email'])) {
            /** @var array<string, mixed> $emailData */
            $emailData = $data['email'];
            $email = EmailConfig::fromArray($emailData);
        }

        $concurrency = null;
        if (isset($data['concurrency']) && is_array($data['concurrency'])) {
            /** @var array<string, mixed> $concurrencyData */
            $concurrencyData = $data['concurrency'];
            $concurrency = ConcurrencyConfig::fromArray($concurrencyData);
        }

        /** @var string|null $cacheNamespace */
        $cacheNamespace = $data['cache_namespace'] ?? null;
        if ($cacheNamespace !== null && !is_string($cacheNamespace)) {
            /** @var string $cacheNamespace */
            $cacheNamespace = (string) $cacheNamespace;
        }

        /** @var int|null $cacheTtlSecs */
        $cacheTtlSecs = $data['cache_ttl_secs'] ?? null;
        if ($cacheTtlSecs !== null && !is_int($cacheTtlSecs)) {
            /** @var int $cacheTtlSecs */
            $cacheTtlSecs = (int) $cacheTtlSecs;
        }

        $securityLimits = null;
        if (isset($data['security_limits']) && is_array($data['security_limits'])) {
            /** @var array<string, mixed> $securityLimitsData */
            $securityLimitsData = $data['security_limits'];
            $securityLimits = SecurityLimitsConfig::fromArray($securityLimitsData);
        }

        return new self(
            useCache: $useCache,
            enableQualityProcessing: $enableQualityProcessing,
            ocr: $ocr,
            forceOcr: $forceOcr,
            chunking: $chunking,
            images: $imageExtraction,
            pdfOptions: $pdf,
            tokenReduction: $tokenReduction,
            languageDetection: $languageDetection,
            pages: $page,
            keywords: $keywords,
            postprocessor: $postprocessor,
            htmlOptions: $htmlOptions,
            securityLimits: $securityLimits,
            maxConcurrentExtractions: $maxConcurrentExtractions,
            resultFormat: $resultFormat,
            outputFormat: $outputFormat,
            includeDocumentStructure: $includeDocumentStructure,
            acceleration: $acceleration,
            email: $email,
            concurrency: $concurrency,
            cacheNamespace: $cacheNamespace,
            cacheTtlSecs: $cacheTtlSecs,
        );
    }

    /**
     * Create configuration from JSON string.
     */
    public static function fromJson(string $json): self
    {
        $data = json_decode($json, true);
        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new \InvalidArgumentException('Invalid JSON: ' . json_last_error_msg());
        }
        if (!is_array($data)) {
            throw new \InvalidArgumentException('JSON must decode to an object/array');
        }
        /** @var array<string, mixed> $data */
        return self::fromArray($data);
    }

    /**
     * Simple TOML parser for kreuzberg configuration format.
     *
     * @param string $toml TOML content
     * @return array<string, mixed> Parsed configuration
     */
    private static function parseTOML(string $toml): array
    {
        $result = [];
        $lines = explode("\n", $toml);
        $currentSection = null;

        foreach ($lines as $line) {
            // Remove comments and trim
            if (($commentPos = strpos($line, '#')) !== false) {
                $line = substr($line, 0, $commentPos);
            }
            $line = trim($line);

            if (empty($line)) {
                continue;
            }

            // Parse section header [section_name]
            if (preg_match('/^\[([^\]]+)\]$/', $line, $matches)) {
                $currentSection = $matches[1];
                if (!isset($result[$currentSection])) {
                    $result[$currentSection] = [];
                }
                continue;
            }

            // Parse key = value
            if (preg_match('/^([^=]+)=(.+)$/', $line, $matches)) {
                $key = trim($matches[1]);
                $value = trim($matches[2]);

                // Convert value type
                if (strtolower($value) === 'true') {
                    $value = true;
                } elseif (strtolower($value) === 'false') {
                    $value = false;
                } elseif (is_numeric($value)) {
                    $value = strpos($value, '.') !== false ? (float) $value : (int) $value;
                } else {
                    // Remove quotes if present
                    $value = preg_replace('/^["\']|["\']$/', '', $value);
                }

                if ($currentSection !== null) {
                    /** @var array<string, mixed> $sectionArray */
                    $sectionArray = $result[$currentSection];
                    $sectionArray[$key] = $value;
                    $result[$currentSection] = $sectionArray;
                } else {
                    $result[$key] = $value;
                }
            }
        }

        return $result;
    }

    /**
     * Create configuration from JSON file.
     */
    public static function fromFile(string $path): self
    {
        if (!file_exists($path)) {
            throw new \InvalidArgumentException("File not found: {$path}");
        }
        $contents = file_get_contents($path);
        if ($contents === false) {
            throw new \InvalidArgumentException("Unable to read file: {$path}");
        }

        // Detect format from file extension
        if (str_ends_with($path, '.toml')) {
            $data = self::parseTOML($contents);
        } else {
            // Default to JSON
            $data = json_decode($contents, true);
            if (json_last_error() !== JSON_ERROR_NONE) {
                throw new \InvalidArgumentException('Invalid JSON: ' . json_last_error_msg());
            }
            if (!is_array($data)) {
                throw new \InvalidArgumentException('JSON must decode to an object/array');
            }
        }

        /** @var array<string, mixed> $data */
        return self::fromArray($data);
    }

    /**
     * Discover and load configuration from current or parent directories.
     *
     * Searches for kreuzberg.toml configuration file in the current working
     * directory and parent directories up the filesystem tree. Returns null
     * if no configuration file is found.
     *
     * @return self|null Loaded configuration, or null if not found
     * @throws \InvalidArgumentException If configuration file is invalid
     */
    public static function discover(): ?self
    {
        $cwd = getcwd();
        if ($cwd === false) {
            return null;
        }

        $current = $cwd;
        while (true) {
            $configPath = $current . '/kreuzberg.toml';
            if (file_exists($configPath)) {
                return self::fromFile($configPath);
            }

            $parent = dirname($current);
            if ($parent === $current) {
                // Reached filesystem root
                break;
            }
            $current = $parent;
        }

        return null;
    }

    /**
     * Convert configuration to array for FFI.
     *
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        $result = [
            'ocr' => $this->ocr?->toArray(),
            'pdf_options' => $this->pdfOptions?->toArray(),
            'chunking' => $this->chunking?->toArray(),
            'images' => $this->images?->toArray(),
            'pages' => $this->pages?->toArray(),
            'language_detection' => $this->languageDetection?->toArray(),
            'keywords' => $this->keywords?->toArray(),
            'html_options' => $this->htmlOptions?->toArray(),
            'security_limits' => $this->securityLimits?->toArray(),
            'postprocessor' => $this->postprocessor?->toArray(),
            'token_reduction' => $this->tokenReduction?->toArray(),
            'acceleration' => $this->acceleration?->toArray(),
            'email' => $this->email?->toArray(),
            'concurrency' => $this->concurrency?->toArray(),
        ];

        // Add simple boolean/string fields only if explicitly set to non-default values
        // useCache defaults to true, so only add if false
        if (!$this->useCache) {
            $result['use_cache'] = false;
        }
        // enableQualityProcessing defaults to true, so only add if false
        if (!$this->enableQualityProcessing) {
            $result['enable_quality_processing'] = false;
        }
        // forceOcr defaults to false, so only add if true
        if ($this->forceOcr) {
            $result['force_ocr'] = true;
        }
        // maxConcurrentExtractions defaults to null, so only add if set
        if ($this->maxConcurrentExtractions !== null) {
            $result['max_concurrent_extractions'] = $this->maxConcurrentExtractions;
        }
        // resultFormat defaults to 'unified', so only add if different
        if ($this->resultFormat !== 'unified') {
            $result['result_format'] = $this->resultFormat;
        }
        // outputFormat defaults to 'plain', so only add if different
        if ($this->outputFormat !== 'plain') {
            $result['output_format'] = $this->outputFormat;
        }
        // includeDocumentStructure defaults to false, so only add if true
        if ($this->includeDocumentStructure) {
            $result['include_document_structure'] = true;
        }
        // cacheNamespace defaults to null, so only add if set
        if ($this->cacheNamespace !== null) {
            $result['cache_namespace'] = $this->cacheNamespace;
        }
        // cacheTtlSecs defaults to null, so only add if set
        if ($this->cacheTtlSecs !== null) {
            $result['cache_ttl_secs'] = $this->cacheTtlSecs;
        }

        return array_filter($result, static fn ($value): bool => $value !== null);
    }

    /**
     * Convert configuration to JSON string.
     */
    public function toJson(): string
    {
        $json = json_encode($this->toArray(), JSON_PRETTY_PRINT);
        if ($json === false) {
            throw new \RuntimeException('Failed to encode configuration to JSON');
        }
        return $json;
    }

    /**
     * Create a new configuration builder instance.
     *
     * @return ExtractionConfigBuilder A builder for creating ExtractionConfig instances
     */
    public static function builder(): ExtractionConfigBuilder
    {
        return new ExtractionConfigBuilder();
    }
}
