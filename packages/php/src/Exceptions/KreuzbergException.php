<?php

declare(strict_types=1);

namespace Kreuzberg\Exceptions;

use Exception;

/**
 * Base exception class for all Kreuzberg errors.
 *
 * This exception is thrown when document extraction fails due to various reasons
 * such as invalid files, unsupported formats, OCR errors, parsing failures, etc.
 */
class KreuzbergException extends Exception
{
    /**
     * Panic context from Rust, if available.
     */
    public readonly ?PanicContext $panicContext;

    /**
     * Create a new KreuzbergException.
     *
     * @param string $message Error message
     * @param int $code Error code (0 for generic errors)
     * @param Exception|null $previous Previous exception for chaining
     * @param PanicContext|null $panicContext Panic context from Rust
     */
    public function __construct(
        string $message = '',
        int $code = 0,
        ?Exception $previous = null,
        ?PanicContext $panicContext = null,
    ) {
        parent::__construct($message, $code, $previous);
        $this->panicContext = $panicContext;
    }

    /**
     * Create exception for validation errors.
     */
    public static function validation(string $message): self
    {
        return new self("Validation error: {$message}", 1);
    }

    /**
     * Create exception for parsing errors.
     */
    public static function parsing(string $message): self
    {
        return new self("Parsing error: {$message}", 2);
    }

    /**
     * Create exception for OCR errors.
     */
    public static function ocr(string $message): self
    {
        return new self("OCR error: {$message}", 3);
    }

    /**
     * Create exception for missing dependencies.
     */
    public static function missingDependency(string $message): self
    {
        return new self("Missing dependency: {$message}", 4);
    }

    /**
     * Create exception for I/O errors.
     */
    public static function io(string $message): self
    {
        return new self("I/O error: {$message}", 5);
    }

    /**
     * Create exception for plugin errors.
     */
    public static function plugin(string $message): self
    {
        return new self("Plugin error: {$message}", 6);
    }

    /**
     * Create exception for unsupported format errors.
     */
    public static function unsupportedFormat(string $message): self
    {
        return new self("Unsupported format: {$message}", 7);
    }

    /**
     * Create exception for embedding errors.
     */
    public static function embedding(string $message): self
    {
        return new self("Embedding error: {$message}", 8);
    }
}
