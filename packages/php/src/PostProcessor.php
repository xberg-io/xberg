<?php

declare(strict_types=1);

namespace Kreuzberg;

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

     * @param mixed $result
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function process(, mixed $result, mixed $config): mixed;

    /**
     * Get the processing stage for this post-processor.
     *

     * @return mixed Return value from the plugin method
     */
    public function processing_stage(): mixed;

    /**
     * Optional: Check if this processor should run for a given result.
     *

     * @param mixed $_result
     * @param mixed $_config
     * @return mixed Return value from the plugin method
     */
    public function should_process(, mixed $_result, mixed $_config): mixed;

    /**
     * Optional: Estimate processing time in milliseconds.
     *

     * @param mixed $_result
     * @return mixed Return value from the plugin method
     */
    public function estimated_duration_ms(, mixed $_result): mixed;

    /**
     * Execution priority within the processing stage.
     *

     * @return mixed Return value from the plugin method
     */
    public function priority(): mixed;

}
