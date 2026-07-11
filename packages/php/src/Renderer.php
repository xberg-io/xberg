<?php

declare(strict_types=1);

namespace Xberg;

/**
 * Plugin interface for Renderer.
 *
 * Implement this interface and register an instance with the corresponding
 * registration function to provide custom behavior for extraction.
 */
interface Renderer
{

    /**
     * Binding-safe rendering entry point for foreign-language plugin bridges.
     *

     * @param ExtractedDocument $result
     * @return string Return value from the plugin method
     */
    public function render_result(ExtractedDocument $result): string;

}
