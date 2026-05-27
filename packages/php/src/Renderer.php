<?php

declare(strict_types=1);

namespace Kreuzberg;

/**
 * Plugin interface for Renderer.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface Renderer
{

    /**
     * Render an `InternalDocument` to the output format.
     *

     * @param mixed $doc
     * @return mixed Return value from the plugin method
     */
    public function render(, mixed $doc): mixed;

}
