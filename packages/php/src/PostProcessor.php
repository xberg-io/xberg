<?php

declare(strict_types=1);

namespace Xberg;

/**
 * Plugin interface for PostProcessor.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface PostProcessor
{

    /**
     * Process an extraction result.
     *

     * @param ExtractedDocument $result
     * @param ExtractionConfig $config
     * @return mixed Return value from the plugin method
     */
    public function process(ExtractedDocument $result, ExtractionConfig $config): mixed;

    /**
     * Get the processing stage for this post-processor.
     *

     * @return mixed Return value from the plugin method
     */
    public function processing_stage(): mixed;

    /**
     * Optional: Check if this processor should run for a given result.
     *

     * @param ExtractedDocument $_result
     * @param ExtractionConfig $_config
     * @return bool Return value from the plugin method
     */
    public function should_process(ExtractedDocument $_result, ExtractionConfig $_config): bool;

    /**
     * Optional: Estimate processing time in milliseconds.
     *

     * @param ExtractedDocument $_result
     * @return int Return value from the plugin method
     */
    public function estimated_duration_ms(ExtractedDocument $_result): int;

    /**
     * Execution priority within the processing stage.
     *

     * @return int Return value from the plugin method
     */
    public function priority(): int;

}
