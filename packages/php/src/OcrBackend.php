<?php

declare(strict_types=1);

namespace Kreuzberg;

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
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function process_image(mixed $image_bytes, mixed $config): mixed;

    /**
     * Process a file and extract text via OCR.
     *

     * @param mixed $path
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function process_image_file(mixed $path, mixed $config): mixed;

    /**
     * Check if this backend supports a given language code.
     *

     * @param string $lang
     * @return mixed Return value from the plugin method
     */
    public function supports_language(string $lang): mixed;

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

     * @return mixed Return value from the plugin method
     */
    public function supports_table_detection(): mixed;

    /**
     * Check if the backend supports direct document-level processing (e.g. for PDFs).
     *

     * @return mixed Return value from the plugin method
     */
    public function supports_document_processing(): mixed;

    /**
     * Declare that this backend emits structured markdown directly (tables, headings, lists)
     *

     * @return mixed Return value from the plugin method
     */
    public function emits_structured_markdown(): mixed;

    /**
     * Process a document file directly via OCR.
     *

     * @param mixed $_path
     * @param mixed $_config
     * @return mixed Return value from the plugin method
     */
    public function process_document(mixed $_path, mixed $_config): mixed;

}
