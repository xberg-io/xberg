<?php

declare(strict_types=1);

namespace Kreuzberg;

/**
 * Plugin interface for DocumentExtractor.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface DocumentExtractor
{

    /**
     * Extract content from a byte array.
     *

     * @param mixed $content
     * @param string $mime_type
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function extract_bytes(, mixed $content, string $mime_type, mixed $config): mixed;

    /**
     * Extract content from a file.
     *

     * @param mixed $path
     * @param string $mime_type
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function extract_file(, mixed $path, string $mime_type, mixed $config): mixed;

    /**
     * Get the list of MIME types supported by this extractor.
     *

     * @return mixed Return value from the plugin method
     */
    public function supported_mime_types(): mixed;

    /**
     * Get the priority of this extractor.
     *

     * @return mixed Return value from the plugin method
     */
    public function priority(): mixed;

    /**
     * Optional: Check if this extractor can handle a specific file.
     *

     * @param mixed $_path
     * @param string $_mime_type
     * @return mixed Return value from the plugin method
     */
    public function can_handle(, mixed $_path, string $_mime_type): mixed;

    /**
     * Attempt to get a reference to this extractor as a SyncExtractor.
     *

     * @return mixed Return value from the plugin method
     */
    public function as_sync_extractor(): mixed;

}
