<?php

declare(strict_types=1);

namespace Xberg;

/**
 * Plugin interface for Validator.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface Validator
{

    /**
     * Validate an extraction result.
     *

     * @param ExtractedDocument $result
     * @param ExtractionConfig $config
     * @return mixed Return value from the plugin method
     */
    public function validate(ExtractedDocument $result, ExtractionConfig $config): mixed;

    /**
     * Optional: Check if this validator should run for a given result.
     *

     * @param ExtractedDocument $_result
     * @param ExtractionConfig $_config
     * @return bool Return value from the plugin method
     */
    public function should_validate(ExtractedDocument $_result, ExtractionConfig $_config): bool;

    /**
     * Optional: Get the validation priority.
     *

     * @return int Return value from the plugin method
     */
    public function priority(): int;

}
