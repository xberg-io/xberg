<?php

declare(strict_types=1);

namespace Xberg;

/**
 * Plugin interface for OcrBackend.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface OcrBackend
{

    /**
     * Process an image and extract text via OCR.
     *

     * @param mixed $image_bytes
     * @param OcrConfig $config
     * @return ExtractedDocument Return value from the plugin method
     */
    public function process_image(mixed $image_bytes, OcrConfig $config): ExtractedDocument;

    /**
     * Process a file and extract text via OCR.
     *

     * @param mixed $path
     * @param OcrConfig $config
     * @return ExtractedDocument Return value from the plugin method
     */
    public function process_image_file(mixed $path, OcrConfig $config): ExtractedDocument;

    /**
     * Check if this backend supports a given language code.
     *

     * @param string $lang
     * @return bool Return value from the plugin method
     */
    public function supports_language(string $lang): bool;

    /**
     * Get the backend type identifier.
     *

     * @return mixed Return value from the plugin method
     */
    public function backend_type(): mixed;

    /**
     * Optional: Get a list of all supported languages.
     *

     * @return mixed Return value from the plugin method
     */
    public function supported_languages(): mixed;

    /**
     * Optional: Check if the backend supports table detection.
     *

     * @return bool Return value from the plugin method
     */
    public function supports_table_detection(): bool;

    /**
     * Check if the backend supports direct document-level processing (e.g. for PDFs).
     *

     * @return bool Return value from the plugin method
     */
    public function supports_document_processing(): bool;

    /**
     * Declare that this backend emits structured markdown directly (tables, headings, lists)
     *

     * @return bool Return value from the plugin method
     */
    public function emits_structured_markdown(): bool;

    /**
     * Process a document file directly via OCR.
     *

     * @param mixed $_path
     * @param OcrConfig $_config
     * @return ExtractedDocument Return value from the plugin method
     */
    public function process_document(mixed $_path, OcrConfig $_config): ExtractedDocument;

}
