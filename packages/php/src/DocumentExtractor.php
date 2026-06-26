<?php

declare(strict_types=1);

namespace Xberg;

/**
 * Plugin interface for DocumentExtractor.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface DocumentExtractor
{

    /**
     * Binding-safe extraction entry point for foreign-language plugin bridges.
     *

     * @param ExtractInput $input
     * @param ExtractionConfig $config
     * @return ExtractedDocument Return value from the plugin method
     */
    public function extract(ExtractInput $input, ExtractionConfig $config): ExtractedDocument;

    /**
     * Get the list of MIME types supported by this extractor.
     *

     * @return mixed Return value from the plugin method
     */
    public function supported_mime_types(): mixed;

    /**
     * Get the priority of this extractor.
     *

     * @return int Return value from the plugin method
     */
    public function priority(): int;

    /**
     * Optional: Check if this extractor can handle a specific file.
     *

     * @param mixed $_path
     * @param string $_mime_type
     * @return bool Return value from the plugin method
     */
    public function can_handle(mixed $_path, string $_mime_type): bool;

}
