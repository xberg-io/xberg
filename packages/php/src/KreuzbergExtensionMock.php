<?php

declare(strict_types=1);

/**
 * Mock implementation of Kreuzberg extension functions for testing.
 *
 * This provides PHP implementations of the extension functions when the
 * Rust extension is not available, allowing tests to run.
 */

/**
 * Static storage class for mock implementations.
 * Using a class with static properties instead of $GLOBALS for better test isolation.
 */
class KreuzbergMockStorage
{
    /** @var array<string, callable> */
    public static array $postProcessors = [];
    /** @var array<string, callable> */
    public static array $validators = [];
    /** @var array<string, callable> */
    public static array $extractors = [];
    /** @var array<string, callable> */
    public static array $ocrBackends = [];
    /** @var array<string, callable> */
    public static array $documentExtractors = [];

    public static function reset(): void
    {
        self::$postProcessors = [];
        self::$validators = [];
        self::$extractors = [];
        self::$ocrBackends = [];
        self::$documentExtractors = [];
    }
}

// Only define if not already defined by the extension
if (!function_exists('kreuzberg_extract_file')) {
    /**
     * @param string|null $config_json JSON-encoded configuration
     * @return array<string, mixed>
     */
    function kreuzberg_extract_file(
        string $filePath,
        ?string $mimeType = null,
        ?string $config_json = null,
    ): array {
        // Decode JSON config to array (matching native extension which accepts JSON string)
        /** @var array<string, mixed>|null $config */
        $config = $config_json !== null ? json_decode($config_json, true) : null;

        // Validate configuration
        if ($config !== null) {
            // Validate chunking config
            if (isset($config['chunking']) && is_array($config['chunking'])) {
                if (isset($config['chunking']['max_chunk_size']) && $config['chunking']['max_chunk_size'] < 0) {
                    $maxChunkSizeValue = $config['chunking']['max_chunk_size'];
                    throw new \Kreuzberg\Exceptions\KreuzbergException('[Validation] Invalid maxChunkSize: must be positive, got ' . (is_scalar($maxChunkSizeValue) ? (string)$maxChunkSizeValue : 'unknown'));
                }
            }
        }

        // Check if it's a directory instead of a file
        if (is_dir($filePath)) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("Path is a directory, not a file: $filePath");
        }

        // Mock implementation - return basic extraction result
        if (!file_exists($filePath)) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("File not found: $filePath");
        }

        $content = file_get_contents($filePath);
        if ($content === false) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("Failed to read file: $filePath");
        }

        // If MIME type is not provided, detect it from the file
        if ($mimeType === null) {
            $ext = strtolower(pathinfo($filePath, PATHINFO_EXTENSION));
            // If file has .pdf extension, treat it as PDF regardless of content
            if ($ext === 'pdf') {
                $mimeType = 'application/pdf';
            } else {
                $mimeType = kreuzberg_detect_mime_type_from_path($filePath);
                // If the detected MIME type is just plain text and the content is very short,
                // it might be a corrupted file without proper format markers
                if ($mimeType === 'text/plain' && strlen($content) < 100) {
                    // Check if it has any format-like content
                    if (!preg_match('/^(<?xml|<!DOCTYPE|{|\[|%|PK|\xFF\xD8\xFF|\x89PNG|GIF|BM)/i', $content)) {
                        throw new \Kreuzberg\Exceptions\KreuzbergException('Unsupported or corrupted file format: could not detect document type');
                    }
                }
            }
        }

        // Validate the MIME type (already set by this point)
        try {
            kreuzberg_validate_mime_type($mimeType);
        } catch (\Exception $e) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("Invalid MIME type: $mimeType");
        }

        // Validate PDF files have valid signatures (allow leading content/comments)
        if ($mimeType === 'application/pdf') {
            // PDFs can have comments before the "%PDF" header
            if (strpos($content, '%PDF') === false && strpos($content, '%!PS-Adobe') === false) {
                throw new \Kreuzberg\Exceptions\KreuzbergException("Corrupted or invalid PDF file: $filePath");
            }
        }

        // Validate mismatched MIME types - if the detected type doesn't match the provided one
        if ($mimeType !== kreuzberg_detect_mime_type_from_path($filePath)) {
            // Only throw if provided MIME type doesn't match detected one
            $detectedType = kreuzberg_detect_mime_type_from_path($filePath);
            if ($mimeType !== $detectedType) {
                // Check if the content actually matches what it claims to be
                if ($mimeType === 'application/pdf' && !str_starts_with($content, '%PDF')) {
                    throw new \Kreuzberg\Exceptions\KreuzbergException("MIME type mismatch: provided '$mimeType' but file appears to be '$detectedType'");
                }
            }
        }

        // Mock metadata with page count for PDFs
        $metadata = [];
        if (strpos($mimeType, 'pdf') !== false || $mimeType === 'application/pdf') {
            $metadata['page_count'] = 1;
        }

        // Check if image extraction is enabled
        $images = [];
        $extractImages = false;
        if ($config !== null) {
            if (isset($config['extract_images'])) {
                $extractImages = (bool) $config['extract_images'];
            } elseif (isset($config['images']) && is_array($config['images']) && isset($config['images']['extract_images'])) {
                $extractImages = (bool) $config['images']['extract_images'];
            }
        }

        // If image extraction is enabled, return mock images with format data
        if ($extractImages) {
            $images = [
                [
                    'data' => 'PNG image data',
                    'format' => 'PNG',
                    'image_index' => 0,
                    'page_number' => 1,
                    'width' => 100,
                    'height' => 100,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock PNG image',
                ],
                [
                    'data' => 'JPEG image data',
                    'format' => 'JPEG',
                    'image_index' => 1,
                    'page_number' => 1,
                    'width' => 150,
                    'height' => 150,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock JPEG image',
                ],
            ];
        }

        return [
            'content' => "Mock extraction result from $filePath",
            'mime_type' => $mimeType,
            'metadata' => $metadata,
            'tables' => [],
            'detected_languages' => ['en'],
            'chunks' => [],
            'images' => $images,
            'pages' => [
                [
                    'page_number' => 1,
                    'content' => "Mock extraction result from $filePath",
                    'tables' => [],
                    'images' => $images,
                ],
            ],
            'keywords' => [],
        ];
    }
}

if (!function_exists('kreuzberg_extract_bytes')) {
    /**
     * @param string|null $config_json JSON-encoded configuration
     * @return array<string, mixed>
     */
    function kreuzberg_extract_bytes(
        string $data,
        string $mimeType,
        ?string $config_json = null,
    ): array {
        // Decode JSON config to array (matching native extension which accepts JSON string)
        /** @var array<string, mixed>|null $config */
        $config = $config_json !== null ? json_decode($config_json, true) : null;

        // Validate input - only require non-empty for PDFs
        if (empty($data) && $mimeType === 'application/pdf') {
            throw new \Kreuzberg\Exceptions\KreuzbergException('Empty data provided');
        }

        // Validate MIME type
        try {
            kreuzberg_validate_mime_type($mimeType);
        } catch (\Exception $e) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("Invalid MIME type: $mimeType");
        }

        // Validate PDF files have valid signatures (allow leading content/comments)
        if ($mimeType === 'application/pdf') {
            // PDFs can have comments before the "%PDF" header
            if (strpos($data, '%PDF') === false && strpos($data, '%!PS-Adobe') === false) {
                throw new \Kreuzberg\Exceptions\KreuzbergException('Corrupted or invalid PDF file');
            }
        }

        // Validate mismatched MIME types
        $detectedType = kreuzberg_detect_mime_type($data);
        if ($mimeType !== $detectedType && $detectedType !== 'application/octet-stream') {
            // Only throw if the detected type is clearly different and is not a generic type
            if (($mimeType === 'application/pdf' && $detectedType !== 'application/pdf') ||
                ($mimeType !== 'application/pdf' && $detectedType === 'application/pdf')) {
                throw new \Kreuzberg\Exceptions\KreuzbergException("MIME type mismatch: provided '$mimeType' but data appears to be '$detectedType'");
            }
        }

        // Validate configuration
        if ($config !== null) {
            // Validate chunking config
            if (isset($config['chunking']) && is_array($config['chunking'])) {
                if (isset($config['chunking']['max_chunk_size']) && $config['chunking']['max_chunk_size'] < 0) {
                    $maxChunkSizeValue = $config['chunking']['max_chunk_size'];
                    throw new \Kreuzberg\Exceptions\KreuzbergException('[Validation] Invalid maxChunkSize: must be positive, got ' . (is_scalar($maxChunkSizeValue) ? (string)$maxChunkSizeValue : 'unknown'));
                }
            }
        }

        // Mock implementation
        $content = 'Mock extraction result from bytes';
        $pages = [];

        // Handle pages configuration
        if ($config !== null && isset($config['page']) && is_array($config['page'])) {
            /** @var array<string, mixed> $pageConfig */
            $pageConfig = $config['page'];
            if (isset($pageConfig['extract_pages']) && $pageConfig['extract_pages']) {
                // Return mock pages
                $pages = [
                    [
                        'content' => 'Page 1 content',
                        'page_number' => 1,
                        'tables' => [],
                        'images' => [],
                    ],
                ];
            }

            // Handle page markers
            if (isset($pageConfig['insert_page_markers']) && $pageConfig['insert_page_markers']) {
                /** @var string $markerFormat */
                $markerFormat = $pageConfig['marker_format'] ?? '--- PAGE {page_num} ---';
                $marker = str_replace('{page_num}', '1', $markerFormat);
                $content = $marker . "\nMock extraction result from bytes";
            }
        }

        // Mock metadata with page count for PDFs
        $metadata = [];
        if ($mimeType === 'application/pdf') {
            $metadata['page_count'] = 1;
        }

        // Check if image extraction is enabled
        $images = [];
        $extractImages = false;
        if ($config !== null) {
            if (isset($config['extract_images'])) {
                $extractImages = (bool) $config['extract_images'];
            } elseif (isset($config['images']) && is_array($config['images']) && isset($config['images']['extract_images'])) {
                $extractImages = (bool) $config['images']['extract_images'];
            }
        }

        // If image extraction is enabled, return mock images with format data
        if ($extractImages) {
            $images = [
                [
                    'data' => 'PNG image data',
                    'format' => 'PNG',
                    'image_index' => 0,
                    'page_number' => 1,
                    'width' => 100,
                    'height' => 100,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock PNG image',
                ],
                [
                    'data' => 'JPEG image data',
                    'format' => 'JPEG',
                    'image_index' => 1,
                    'page_number' => 1,
                    'width' => 150,
                    'height' => 150,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock JPEG image',
                ],
            ];
        }

        return [
            'content' => $content,
            'mime_type' => $mimeType,
            'metadata' => $metadata,
            'tables' => [],
            'detected_languages' => ['en'],
            'chunks' => [],
            'images' => $images,
            'pages' => $pages ?: [[
                'page_number' => 1,
                'content' => $content,
                'tables' => [],
                'images' => $images,
            ]],
            'keywords' => [],
        ];
    }
}

if (!function_exists('kreuzberg_batch_extract_files')) {
    /**
     * @param array<int, string> $paths
     * @param string|null $config_json JSON-encoded configuration
     * @return array<int, array<string, mixed>>
     */
    function kreuzberg_batch_extract_files(
        array $paths,
        ?string $config_json = null,
    ): array {
        // Decode JSON config to array (matching native extension which accepts JSON string)
        /** @var array<string, mixed>|null $config */
        $config = $config_json !== null ? json_decode($config_json, true) : null;

        // Validate configuration
        if ($config !== null) {
            // Validate chunking config
            if (isset($config['chunking']) && is_array($config['chunking'])) {
                if (isset($config['chunking']['max_chunk_size']) && $config['chunking']['max_chunk_size'] < 0) {
                    $maxChunkSizeValue = $config['chunking']['max_chunk_size'];
                    throw new \Kreuzberg\Exceptions\KreuzbergException('[Validation] Invalid maxChunkSize: must be positive, got ' . (is_scalar($maxChunkSizeValue) ? (string)$maxChunkSizeValue : 'unknown'));
                }
            }
        }

        // Mock implementation
        $results = [];

        // Check if image extraction is enabled
        $extractImages = false;
        if ($config !== null) {
            if (isset($config['extract_images'])) {
                $extractImages = (bool) $config['extract_images'];
            } elseif (isset($config['images']) && is_array($config['images']) && isset($config['images']['extract_images'])) {
                $extractImages = (bool) $config['images']['extract_images'];
            }
        }

        // Create mock images if extraction is enabled
        $images = [];
        if ($extractImages) {
            $images = [
                [
                    'data' => 'PNG image data',
                    'format' => 'PNG',
                    'image_index' => 0,
                    'page_number' => 1,
                    'width' => 100,
                    'height' => 100,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock PNG image',
                ],
                [
                    'data' => 'JPEG image data',
                    'format' => 'JPEG',
                    'image_index' => 1,
                    'page_number' => 1,
                    'width' => 150,
                    'height' => 150,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock JPEG image',
                ],
            ];
        }

        foreach ($paths as $path) {
            // Check if file exists
            if (!file_exists($path)) {
                throw new \Kreuzberg\Exceptions\KreuzbergException("File not found: $path");
            }

            $mimeType = kreuzberg_detect_mime_type_from_path($path);

            // Mock metadata with page count for PDFs
            $metadata = [];
            if ($mimeType === 'application/pdf') {
                $metadata['page_count'] = 1;
            }

            $results[] = [
                'content' => "Mock extraction from $path",
                'mime_type' => $mimeType,
                'metadata' => $metadata,
                'tables' => [],
                'detected_languages' => ['en'],
                'chunks' => [],
                'images' => $images,
                'pages' => [[
                    'page_number' => 1,
                    'content' => "Mock extraction from $path",
                    'tables' => [],
                    'images' => $images,
                ]],
                'keywords' => [],
            ];
        }
        return $results;
    }
}

if (!function_exists('kreuzberg_batch_extract_bytes')) {
    /**
     * @param array<int, string> $dataList
     * @param array<int, string> $mimeTypes
     * @param string|null $config_json JSON-encoded configuration
     * @return array<int, array<string, mixed>>
     */
    function kreuzberg_batch_extract_bytes(
        array $dataList,
        array $mimeTypes,
        ?string $config_json = null,
    ): array {
        // Decode JSON config to array (matching native extension which accepts JSON string)
        /** @var array<string, mixed>|null $config */
        $config = $config_json !== null ? json_decode($config_json, true) : null;

        // Validate that array lengths match
        if (count($dataList) !== count($mimeTypes)) {
            throw new \Kreuzberg\Exceptions\KreuzbergException('data_list and mime_types must have the same length (got ' . count($dataList) . ' and ' . count($mimeTypes) . ')');
        }

        // Validate configuration
        if ($config !== null) {
            // Validate chunking config
            if (isset($config['chunking']) && is_array($config['chunking'])) {
                if (isset($config['chunking']['max_chunk_size']) && $config['chunking']['max_chunk_size'] < 0) {
                    $maxChunkSizeValue = $config['chunking']['max_chunk_size'];
                    throw new \Kreuzberg\Exceptions\KreuzbergException('[Validation] Invalid maxChunkSize: must be positive, got ' . (is_scalar($maxChunkSizeValue) ? (string)$maxChunkSizeValue : 'unknown'));
                }
            }
        }

        // Mock implementation
        $results = [];

        // Check if image extraction is enabled
        $extractImages = false;
        if ($config !== null) {
            if (isset($config['extract_images'])) {
                $extractImages = (bool) $config['extract_images'];
            } elseif (isset($config['images']) && is_array($config['images']) && isset($config['images']['extract_images'])) {
                $extractImages = (bool) $config['images']['extract_images'];
            }
        }

        // Create mock images if extraction is enabled
        $images = [];
        if ($extractImages) {
            $images = [
                [
                    'data' => 'PNG image data',
                    'format' => 'PNG',
                    'image_index' => 0,
                    'page_number' => 1,
                    'width' => 100,
                    'height' => 100,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock PNG image',
                ],
                [
                    'data' => 'JPEG image data',
                    'format' => 'JPEG',
                    'image_index' => 1,
                    'page_number' => 1,
                    'width' => 150,
                    'height' => 150,
                    'colorspace' => 'RGB',
                    'bits_per_component' => 8,
                    'is_mask' => false,
                    'description' => 'Mock JPEG image',
                ],
            ];
        }

        foreach ($dataList as $index => $data) {
            if (empty($data)) {
                throw new \Kreuzberg\Exceptions\KreuzbergException("Empty data at index $index");
            }

            $mimeType = $mimeTypes[$index] ?? 'application/octet-stream';

            // Validate MIME type
            try {
                kreuzberg_validate_mime_type($mimeType);
            } catch (\Exception $e) {
                throw new \Kreuzberg\Exceptions\KreuzbergException("Invalid MIME type at index $index: $mimeType");
            }

            // Mock metadata with page count for PDFs
            $metadata = [];
            if ($mimeType === 'application/pdf') {
                $metadata['page_count'] = 1;
            }

            $results[] = [
                'content' => "Mock extraction result $index",
                'mime_type' => $mimeType,
                'metadata' => $metadata,
                'tables' => [],
                'detected_languages' => ['en'],
                'chunks' => [],
                'images' => $images,
                'pages' => [[
                    'page_number' => 1,
                    'content' => "Mock extraction result $index",
                    'tables' => [],
                    'images' => $images,
                ]],
                'keywords' => [],
            ];
        }
        return $results;
    }
}

if (!function_exists('kreuzberg_detect_mime_type')) {
    function kreuzberg_detect_mime_type(string $data): string
    {
        // Mock implementation - simple magic number detection
        if (str_starts_with($data, '%PDF')) {
            return 'application/pdf';
        }
        if (str_starts_with($data, "\x89PNG")) {
            return 'image/png';
        }
        if (str_starts_with($data, "\xFF\xD8\xFF")) {
            return 'image/jpeg';
        }
        if (str_starts_with($data, 'PK')) {
            // Could be .docx, .xlsx, .pptx, or other ZIP-based formats
            // For now, detect based on content or return generic zip
            if (strpos($data, 'word/') !== false) {
                return 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
            }
            if (strpos($data, 'xl/') !== false) {
                return 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
            }
            if (strpos($data, 'ppt/') !== false) {
                return 'application/vnd.openxmlformats-officedocument.presentationml.presentation';
            }
            return 'application/zip';
        }
        if (str_starts_with($data, 'GIF8')) {
            return 'image/gif';
        }
        if (str_starts_with($data, 'BM')) {
            return 'image/bmp';
        }
        if (str_starts_with($data, 'II\x2a\x00') || str_starts_with($data, 'MM\x00\x2a')) {
            return 'image/tiff';
        }
        // Check for text-based formats
        if (strlen($data) > 0 && (ctype_print($data[0]) || $data[0] === "\n" || $data[0] === "\r" || $data[0] === "\t")) {
            if (str_starts_with($data, '<?xml') || str_starts_with($data, '<')) {
                return 'application/xml';
            }
            if (str_starts_with($data, '{') || str_starts_with($data, '[')) {
                return 'application/json';
            }
            return 'text/plain';
        }
        return 'application/octet-stream';
    }
}

if (!function_exists('kreuzberg_detect_mime_type_from_bytes')) {
    function kreuzberg_detect_mime_type_from_bytes(string $data): string
    {
        return kreuzberg_detect_mime_type($data);
    }
}

if (!function_exists('kreuzberg_detect_mime_type_from_path')) {
    function kreuzberg_detect_mime_type_from_path(string $path): string
    {
        // First try to detect by file extension
        $ext = strtolower(pathinfo($path, PATHINFO_EXTENSION));

        $mimeMap = [
            'pdf' => 'application/pdf',
            'txt' => 'text/plain',
            'md' => 'text/markdown',
            'markdown' => 'text/markdown',
            'html' => 'text/html',
            'htm' => 'text/html',
            'xml' => 'application/xml',
            'json' => 'application/json',
            'csv' => 'text/csv',
            'xlsx' => 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
            'xls' => 'application/vnd.ms-excel',
            'docx' => 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
            'doc' => 'application/msword',
            'pptx' => 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
            'ppt' => 'application/vnd.ms-powerpoint',
            'odt' => 'application/vnd.oasis.opendocument.text',
            'ods' => 'application/vnd.oasis.opendocument.spreadsheet',
            'odp' => 'application/vnd.oasis.opendocument.presentation',
            'png' => 'image/png',
            'jpg' => 'image/jpeg',
            'jpeg' => 'image/jpeg',
            'gif' => 'image/gif',
            'bmp' => 'image/bmp',
            'tiff' => 'image/tiff',
            'tif' => 'image/tiff',
            'webp' => 'image/webp',
            'svg' => 'image/svg+xml',
            'zip' => 'application/zip',
            'tar' => 'application/x-tar',
            'gz' => 'application/gzip',
            'tgz' => 'application/x-tar',
            'rtf' => 'application/rtf',
            'epub' => 'application/epub+zip',
            'yml' => 'application/x-yaml',
            'yaml' => 'application/x-yaml',
            'toml' => 'application/toml',
            'eml' => 'message/rfc822',
            'msg' => 'application/vnd.ms-outlook',
            'rst' => 'text/x-rst',
            'org' => 'text/x-org',
            'ipynb' => 'application/x-ipynb+json',
            'tex' => 'application/x-latex',
            'latex' => 'application/x-latex',
            'typst' => 'application/x-typst',
        ];

        if (isset($mimeMap[$ext])) {
            return $mimeMap[$ext];
        }

        // Fall back to magic number detection
        $data = file_get_contents($path, false, null, 0, 512);
        if ($data === false) {
            return 'application/octet-stream';
        }
        return kreuzberg_detect_mime_type($data);
    }
}

if (!function_exists('kreuzberg_validate_mime_type')) {
    function kreuzberg_validate_mime_type(string $mimeType): string
    {
        // List of supported MIME types
        $supportedMimes = [
            'text/plain',
            'text/markdown',
            'text/x-markdown',
            'text/html',
            'application/pdf',
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
            'application/msword',
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
            'application/vnd.ms-excel',
            'application/vnd.openxmlformats-officedocument.presentationml.presentation',
            'application/vnd.ms-powerpoint',
            'application/vnd.oasis.opendocument.text',
            'application/vnd.oasis.opendocument.spreadsheet',
            'application/vnd.oasis.opendocument.presentation',
            'application/xml',
            'text/xml',
            'application/json',
            'text/csv',
            'text/tab-separated-values',
            'application/x-yaml',
            'text/yaml',
            'application/toml',
            'text/toml',
            'application/x-tar',
            'application/gzip',
            'application/x-7z-compressed',
            'application/zip',
            'application/rtf',
            'application/epub+zip',
            'application/x-ipynb+json',
            'application/x-latex',
            'application/x-typst',
            'text/x-rst',
            'text/x-org',
            'text/x-commonmark',
            'text/troff',
            'text/x-pod',
            'text/x-dokuwiki',
            'application/x-bibtex',
            'application/x-biblatex',
            'application/x-fictionbook+xml',
            'application/x-jats+xml',
            'application/docbook+xml',
            'application/x-opml+xml',
            'application/xml+opml',
            'text/x-opml',
            'application/x-research-info-systems',
            'application/csl+json',
            'message/rfc822',
            'application/vnd.ms-outlook',
        ];

        // Check if MIME type is in supported list or is an image type
        if (in_array($mimeType, $supportedMimes, true) || strpos($mimeType, 'image/') === 0) {
            return $mimeType;
        }

        throw new \Exception("Unsupported MIME type: $mimeType");
    }
}

if (!function_exists('kreuzberg_register_post_processor')) {
    function kreuzberg_register_post_processor(string $name, callable $callback): void
    {
        KreuzbergMockStorage::$postProcessors[$name] = $callback;
    }
}

if (!function_exists('kreuzberg_unregister_post_processor')) {
    function kreuzberg_unregister_post_processor(string $name): void
    {
        unset(KreuzbergMockStorage::$postProcessors[$name]);
    }
}

if (!function_exists('kreuzberg_list_post_processors')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_post_processors(): array
    {
        return array_keys(KreuzbergMockStorage::$postProcessors);
    }
}

if (!function_exists('kreuzberg_clear_post_processors')) {
    function kreuzberg_clear_post_processors(): void
    {
        KreuzbergMockStorage::$postProcessors = [];
    }
}

if (!function_exists('kreuzberg_run_post_processors')) {
    function kreuzberg_run_post_processors(mixed &$result): void
    {
        foreach (KreuzbergMockStorage::$postProcessors as $callback) {
            $callback($result);
        }
    }
}

if (!function_exists('kreuzberg_register_validator')) {
    function kreuzberg_register_validator(string $name, callable $callback): void
    {
        KreuzbergMockStorage::$validators[$name] = $callback;
    }
}

if (!function_exists('kreuzberg_unregister_validator')) {
    function kreuzberg_unregister_validator(string $name): void
    {
        unset(KreuzbergMockStorage::$validators[$name]);
    }
}

if (!function_exists('kreuzberg_list_validators')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_validators(): array
    {
        return array_keys(KreuzbergMockStorage::$validators);
    }
}

if (!function_exists('kreuzberg_clear_validators')) {
    function kreuzberg_clear_validators(): void
    {
        KreuzbergMockStorage::$validators = [];
    }
}

if (!function_exists('kreuzberg_run_validators')) {
    function kreuzberg_run_validators(mixed &$result): void
    {
        foreach (KreuzbergMockStorage::$validators as $callback) {
            $callback($result);
        }
    }
}

if (!function_exists('kreuzberg_register_extractor')) {
    function kreuzberg_register_extractor(string $mimeType, callable $callback): void
    {
        KreuzbergMockStorage::$extractors[$mimeType] = $callback;
    }
}

if (!function_exists('kreuzberg_unregister_extractor')) {
    function kreuzberg_unregister_extractor(string $mimeType): void
    {
        unset(KreuzbergMockStorage::$extractors[$mimeType]);
    }
}

if (!function_exists('kreuzberg_list_extractors')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_extractors(): array
    {
        return array_keys(KreuzbergMockStorage::$extractors);
    }
}

if (!function_exists('kreuzberg_clear_extractors')) {
    function kreuzberg_clear_extractors(): void
    {
        KreuzbergMockStorage::$extractors = [];
    }
}

if (!function_exists('kreuzberg_test_plugin')) {
    function kreuzberg_test_plugin(string $pluginType, string $pluginName, mixed &$testData): bool
    {
        return true;
    }
}

if (!function_exists('kreuzberg_list_embedding_presets')) {
    /**
     * @return array<string, array{model: string, dimensions: int}>
     */
    function kreuzberg_list_embedding_presets(): array
    {
        return ['default' => ['model' => 'default', 'dimensions' => 384]];
    }
}

if (!function_exists('kreuzberg_get_embedding_preset')) {
    /**
     * @return array{model: string, dimensions: int}|null
     */
    function kreuzberg_get_embedding_preset(string $name): ?array
    {
        $presets = kreuzberg_list_embedding_presets();
        return $presets[$name] ?? null;
    }
}

if (!function_exists('kreuzberg_get_extensions_for_mime')) {
    /**
     * @return array<string>
     */
    function kreuzberg_get_extensions_for_mime(string $mimeType): array
    {
        $extensionMap = [
            'application/pdf' => ['pdf'],
            'text/plain' => ['txt', 'text'],
            'text/html' => ['html', 'htm'],
            'application/xml' => ['xml'],
            'text/xml' => ['xml'],
            'application/json' => ['json'],
            'image/png' => ['png'],
            'image/jpeg' => ['jpg', 'jpeg'],
            'image/gif' => ['gif'],
            'image/bmp' => ['bmp'],
            'image/tiff' => ['tiff', 'tif'],
            'application/zip' => ['zip'],
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document' => ['docx'],
            'application/msword' => ['doc'],
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' => ['xlsx'],
            'application/vnd.ms-excel' => ['xls'],
            'application/vnd.openxmlformats-officedocument.presentationml.presentation' => ['pptx'],
            'application/vnd.ms-powerpoint' => ['ppt'],
            'application/x-yaml' => ['yaml', 'yml'],
            'application/toml' => ['toml'],
        ];

        return $extensionMap[$mimeType] ?? [];
    }
}

if (!function_exists('kreuzberg_clear_document_extractors')) {
    function kreuzberg_clear_document_extractors(): void
    {
        KreuzbergMockStorage::$documentExtractors = [];
    }
}

if (!function_exists('kreuzberg_list_document_extractors')) {
    /**
     * @return array<string>
     */
    function kreuzberg_list_document_extractors(): array
    {
        return array_keys(KreuzbergMockStorage::$documentExtractors);
    }
}

if (!function_exists('kreuzberg_register_document_extractor')) {
    function kreuzberg_register_document_extractor(string $name, callable $callback): void
    {
        KreuzbergMockStorage::$documentExtractors[$name] = $callback;
    }
}

if (!function_exists('kreuzberg_unregister_document_extractor')) {
    function kreuzberg_unregister_document_extractor(string $name): void
    {
        unset(KreuzbergMockStorage::$documentExtractors[$name]);
    }
}

if (!function_exists('kreuzberg_register_ocr_backend')) {
    function kreuzberg_register_ocr_backend(string $name, callable $callback): void
    {
        KreuzbergMockStorage::$ocrBackends[$name] = $callback;
    }
}

if (!function_exists('kreuzberg_clear_ocr_backends')) {
    function kreuzberg_clear_ocr_backends(): void
    {
        KreuzbergMockStorage::$ocrBackends = [];
    }
}

if (!function_exists('kreuzberg_list_ocr_backends')) {
    /**
     * @return array<string>
     */
    function kreuzberg_list_ocr_backends(): array
    {
        return array_keys(KreuzbergMockStorage::$ocrBackends);
    }
}

if (!function_exists('kreuzberg_unregister_ocr_backend')) {
    function kreuzberg_unregister_ocr_backend(string $name): void
    {
        unset(KreuzbergMockStorage::$ocrBackends[$name]);
    }
}
