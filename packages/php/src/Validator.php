<?php

declare(strict_types=1);

namespace Kreuzberg;

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

     * @param mixed $result
     * @param mixed $config
     * @return mixed Return value from the plugin method
     */
    public function validate(, mixed $result, mixed $config): mixed;

    /**
     * Optional: Check if this validator should run for a given result.
     *

     * @param mixed $_result
     * @param mixed $_config
     * @return mixed Return value from the plugin method
     */
    public function should_validate(, mixed $_result, mixed $_config): mixed;

    /**
     * Optional: Get the validation priority.
     *

     * @return mixed Return value from the plugin method
     */
    public function priority(): mixed;

}
