<?php

declare(strict_types=1);

namespace Kreuzberg\Config;

/**
 * Builder class for constructing ExtractionConfig instances with a fluent interface.
 *
 * This builder pattern addresses the 16-parameter constructor issue in ExtractionConfig,
 * providing a clean, readable way to configure extraction behavior through method chaining.
 *
 * @example
 * ```php
 * use Kreuzberg\Config\ExtractionConfigBuilder;
 * use Kreuzberg\Config\OcrConfig;
 * use Kreuzberg\Config\ChunkingConfig;
 *
 * $config = ExtractionConfig::builder()
 *     ->withOcr(new OcrConfig(backend: 'tesseract', language: 'eng'))
 *     ->withChunking(new ChunkingConfig(maxChunkSize: 1000))
 *     ->withUseCache(true)
 *     ->withMaxConcurrentExtractions(8)
 *     ->build();
 * ```
 */
class ExtractionConfigBuilder
{
    private bool $useCache = true;
    private bool $enableQualityProcessing = true;
    private ?OcrConfig $ocr = null;
    private bool $forceOcr = false;
    /** @var int[]|null */
    private ?array $forceOcrPages = null;
    private ?ChunkingConfig $chunking = null;
    private ?ImageExtractionConfig $images = null;
    private ?PdfConfig $pdfOptions = null;
    private ?TokenReductionConfig $tokenReduction = null;
    private ?LanguageDetectionConfig $languageDetection = null;
    private ?PageConfig $pages = null;
    private ?KeywordConfig $keywords = null;
    private ?PostProcessorConfig $postprocessor = null;
    private ?HtmlConversionOptions $htmlOptions = null;
    private ?int $maxConcurrentExtractions = null;
    private ?ConcurrencyConfig $concurrency = null;
    private string $resultFormat = 'unified';
    private string $outputFormat = 'plain';
    private ?string $cacheNamespace = null;
    private ?int $cacheTtlSecs = null;
    private ?int $extractionTimeoutSecs = null;

    /**
     * Set whether to enable caching of extraction results.
     *
     * @param bool $useCache Whether to cache extraction results
     * @return self For method chaining
     */
    public function withUseCache(bool $useCache): self
    {
        $this->useCache = $useCache;
        return $this;
    }

    /**
     * Set whether to enable quality processing enhancements.
     *
     * @param bool $enableQualityProcessing Whether to apply quality processing
     * @return self For method chaining
     */
    public function withEnableQualityProcessing(bool $enableQualityProcessing): self
    {
        $this->enableQualityProcessing = $enableQualityProcessing;
        return $this;
    }

    /**
     * Set the OCR configuration.
     *
     * @param OcrConfig|null $ocr OCR backend configuration
     * @return self For method chaining
     */
    public function withOcr(?OcrConfig $ocr): self
    {
        $this->ocr = $ocr;
        return $this;
    }

    /**
     * Set whether to force OCR on all documents.
     *
     * @param bool $forceOcr Whether to force OCR processing
     * @return self For method chaining
     */
    public function withForceOcr(bool $forceOcr): self
    {
        $this->forceOcr = $forceOcr;
        return $this;
    }

    /**
     * Set the list of page numbers to force OCR on.
     *
     * @param int[]|null $forceOcrPages 1-indexed page numbers to force OCR on
     * @return self For method chaining
     */
    public function withForceOcrPages(?array $forceOcrPages): self
    {
        $this->forceOcrPages = $forceOcrPages;
        return $this;
    }

    /**
     * Set the chunking configuration.
     *
     * @param ChunkingConfig|null $chunking Text chunking settings
     * @return self For method chaining
     */
    public function withChunking(?ChunkingConfig $chunking): self
    {
        $this->chunking = $chunking;
        return $this;
    }

    /**
     * Set the image extraction configuration.
     *
     * @param ImageExtractionConfig|null $images Image extraction settings
     * @return self For method chaining
     */
    public function withImages(?ImageExtractionConfig $images): self
    {
        $this->images = $images;
        return $this;
    }

    /**
     * Set the PDF configuration.
     *
     * @param PdfConfig|null $pdfOptions PDF extraction settings
     * @return self For method chaining
     */
    public function withPdfOptions(?PdfConfig $pdfOptions): self
    {
        $this->pdfOptions = $pdfOptions;
        return $this;
    }

    /**
     * Set the token reduction configuration.
     *
     * @param TokenReductionConfig|null $tokenReduction Token reduction settings
     * @return self For method chaining
     */
    public function withTokenReduction(?TokenReductionConfig $tokenReduction): self
    {
        $this->tokenReduction = $tokenReduction;
        return $this;
    }

    /**
     * Set the language detection configuration.
     *
     * @param LanguageDetectionConfig|null $languageDetection Language detection settings
     * @return self For method chaining
     */
    public function withLanguageDetection(?LanguageDetectionConfig $languageDetection): self
    {
        $this->languageDetection = $languageDetection;
        return $this;
    }

    /**
     * Set the pages configuration.
     *
     * @param PageConfig|null $pages Page-specific settings
     * @return self For method chaining
     */
    public function withPages(?PageConfig $pages): self
    {
        $this->pages = $pages;
        return $this;
    }

    /**
     * Set the keyword extraction configuration.
     *
     * @param KeywordConfig|null $keywords Keyword extraction settings
     * @return self For method chaining
     */
    public function withKeywords(?KeywordConfig $keywords): self
    {
        $this->keywords = $keywords;
        return $this;
    }

    /**
     * Set the postprocessor configuration.
     *
     * @param PostProcessorConfig|null $postprocessor Postprocessor settings
     * @return self For method chaining
     */
    public function withPostprocessor(?PostProcessorConfig $postprocessor): self
    {
        $this->postprocessor = $postprocessor;
        return $this;
    }

    /**
     * Set the HTML to Markdown conversion options.
     *
     * @param HtmlConversionOptions|array<string, mixed>|null $htmlOptions HTML conversion configuration
     * @return self For method chaining
     */
    public function withHtmlOptions(HtmlConversionOptions|array|null $htmlOptions = null): self
    {
        if (is_array($htmlOptions)) {
            $this->htmlOptions = HtmlConversionOptions::fromArray($htmlOptions);
        } else {
            $this->htmlOptions = $htmlOptions;
        }
        return $this;
    }

    /**
     * Set the maximum number of concurrent extraction operations.
     *
     * @param int|null $maxConcurrentExtractions Maximum concurrent operations
     * @return self For method chaining
     */
    public function withMaxConcurrentExtractions(?int $maxConcurrentExtractions): self
    {
        $this->maxConcurrentExtractions = $maxConcurrentExtractions;
        return $this;
    }

    /**
     * Set the concurrency configuration.
     *
     * @param ConcurrencyConfig|null $concurrency Concurrency settings
     * @return self For method chaining
     */
    public function withConcurrency(?ConcurrencyConfig $concurrency): self
    {
        $this->concurrency = $concurrency;
        return $this;
    }

    /**
     * Set the result format for structured output.
     *
     * @param string $resultFormat Result format (e.g., 'unified', 'element_based')
     * @return self For method chaining
     */
    public function withResultFormat(string $resultFormat): self
    {
        $this->resultFormat = $resultFormat;
        return $this;
    }

    /**
     * Set the output format for extracted content.
     *
     * @param string $outputFormat Output format (e.g., 'plain', 'markdown', 'djot', 'html')
     * @return self For method chaining
     */
    public function withOutputFormat(string $outputFormat): self
    {
        $this->outputFormat = $outputFormat;
        return $this;
    }

    /**
     * Set the cache namespace for tenant isolation.
     *
     * @param string|null $namespace Cache namespace string
     * @return self For method chaining
     */
    public function cacheNamespace(?string $namespace): self
    {
        $this->cacheNamespace = $namespace;
        return $this;
    }

    /**
     * Set the per-request cache TTL in seconds.
     *
     * @param int|null $secs Cache TTL in seconds
     * @return self For method chaining
     */
    public function cacheTtlSecs(?int $secs): self
    {
        $this->cacheTtlSecs = $secs;
        return $this;
    }

    /**
     * Set the default per-file extraction timeout in seconds for batch operations.
     *
     * When set, each file in a batch will be canceled after this duration
     * unless overridden by a per-file timeout. Null means no timeout.
     *
     * @param int|null $secs Extraction timeout in seconds
     * @return self For method chaining
     */
    public function withExtractionTimeoutSecs(?int $secs): self
    {
        $this->extractionTimeoutSecs = $secs;
        return $this;
    }

    /**
     * Build and return the configured ExtractionConfig instance.
     *
     * @return ExtractionConfig The constructed configuration object
     */
    public function build(): ExtractionConfig
    {
        return new ExtractionConfig(
            useCache: $this->useCache,
            enableQualityProcessing: $this->enableQualityProcessing,
            ocr: $this->ocr,
            forceOcr: $this->forceOcr,
            forceOcrPages: $this->forceOcrPages,
            chunking: $this->chunking,
            images: $this->images,
            pdfOptions: $this->pdfOptions,
            tokenReduction: $this->tokenReduction,
            languageDetection: $this->languageDetection,
            pages: $this->pages,
            keywords: $this->keywords,
            postprocessor: $this->postprocessor,
            htmlOptions: $this->htmlOptions,
            maxConcurrentExtractions: $this->maxConcurrentExtractions,
            resultFormat: $this->resultFormat,
            outputFormat: $this->outputFormat,
            concurrency: $this->concurrency,
            cacheNamespace: $this->cacheNamespace,
            cacheTtlSecs: $this->cacheTtlSecs,
            extractionTimeoutSecs: $this->extractionTimeoutSecs,
        );
    }
}
