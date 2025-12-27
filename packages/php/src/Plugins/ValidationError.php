<?php

declare(strict_types=1);

namespace Kreuzberg\Plugins;

use Kreuzberg\Exceptions\KreuzbergException;

/**
 * Exception thrown when validation fails.
 *
 * This exception should be thrown by validators when extraction results
 * do not meet validation criteria. It provides detailed error information
 * about why validation failed.
 *
 * @example
 * ```php
 * use Kreuzberg\Plugins\ValidationError;
 *
 * class ContentValidator
 * {
 *     public function validate(array $result): bool
 *     {
 *         if (strlen($result['content']) < 100) {
 *             throw new ValidationError(
 *                 message: 'Content too short',
 *                 details: [
 *                     'actual_length' => strlen($result['content']),
 *                     'required_length' => 100,
 *                     'validator' => 'min_length',
 *                 ]
 *             );
 *         }
 *
 *         return true;
 *     }
 * }
 * ```
 */
class ValidationError extends KreuzbergException
{
    /**
     * Additional validation error details.
     *
     * @var array<string, mixed>
     */
    private array $details;

    /**
     * Create a new ValidationError.
     *
     * @param string $message Error message describing why validation failed
     * @param array<string, mixed> $details Additional error details (field names, expected values, etc.)
     * @param int $code Error code (default: 1 for validation errors)
     * @param \Exception|null $previous Previous exception for chaining
     *
     * @example
     * ```php
     * throw new ValidationError(
     *     message: 'Invalid document metadata',
     *     details: [
     *         'field' => 'title',
     *         'error' => 'Title is required but missing',
     *         'validator' => 'metadata_validator',
     *     ]
     * );
     * ```
     */
    public function __construct(
        string $message = '',
        array $details = [],
        int $code = 1,
        ?\Exception $previous = null,
    ) {
        parent::__construct($message, $code, $previous);
        $this->details = $details;
    }

    /**
     * Get additional validation error details.
     *
     * @return array<string, mixed> Associative array with error details
     *
     * @example
     * ```php
     * try {
     *     // ... validation that fails
     * } catch (ValidationError $e) {
     *     $details = $e->getDetails();
     *     echo "Field: " . $details['field'] . "\n";
     *     echo "Error: " . $details['error'] . "\n";
     * }
     * ```
     */
    public function getDetails(): array
    {
        return $this->details;
    }

    /**
     * Get a specific detail value.
     *
     * @param string $key Detail key to retrieve
     * @param mixed $default Default value if key doesn't exist
     * @return mixed Detail value or default
     *
     * @example
     * ```php
     * try {
     *     // ... validation that fails
     * } catch (ValidationError $e) {
     *     $field = $e->getDetail('field', 'unknown');
     *     $validator = $e->getDetail('validator', 'unknown');
     * }
     * ```
     */
    public function getDetail(string $key, mixed $default = null): mixed
    {
        return $this->details[$key] ?? $default;
    }

    /**
     * Convert exception to array format.
     *
     * @return array<string, mixed> Exception data as array
     *
     * @example
     * ```php
     * try {
     *     // ... validation that fails
     * } catch (ValidationError $e) {
     *     $errorData = $e->toArray();
     *     // [
     *     //     'message' => 'Content too short',
     *     //     'code' => 1,
     *     //     'details' => [...],
     *     // ]
     * }
     * ```
     */
    public function toArray(): array
    {
        return [
            'message' => $this->getMessage(),
            'code' => $this->getCode(),
            'details' => $this->details,
        ];
    }

    /**
     * Create ValidationError for missing required field.
     *
     * @param string $field Field name that is missing
     * @param string $validator Validator name that detected the error
     * @return self
     *
     * @example
     * ```php
     * if (empty($result['metadata']['title'])) {
     *     throw ValidationError::missingField('title', 'metadata_validator');
     * }
     * ```
     */
    public static function missingField(string $field, string $validator = 'unknown'): self
    {
        return new self(
            message: "Missing required field: {$field}",
            details: [
                'field' => $field,
                'error' => 'required',
                'validator' => $validator,
            ],
        );
    }

    /**
     * Create ValidationError for invalid field value.
     *
     * @param string $field Field name with invalid value
     * @param mixed $actual Actual value received
     * @param mixed $expected Expected value or description
     * @param string $validator Validator name that detected the error
     * @return self
     *
     * @example
     * ```php
     * if (strlen($content) < 100) {
     *     throw ValidationError::invalidValue(
     *         field: 'content.length',
     *         actual: strlen($content),
     *         expected: '>= 100',
     *         validator: 'min_length'
     *     );
     * }
     * ```
     */
    public static function invalidValue(
        string $field,
        mixed $actual,
        mixed $expected,
        string $validator = 'unknown',
    ): self {
        $expectedStr = self::valueToString($expected);
        $actualStr = self::valueToString($actual);

        return new self(
            message: "Invalid value for field '{$field}': expected {$expectedStr}, got {$actualStr}",
            details: [
                'field' => $field,
                'actual' => $actual,
                'expected' => $expected,
                'error' => 'invalid_value',
                'validator' => $validator,
            ],
        );
    }

    /**
     * Convert mixed value to string representation.
     *
     * @param mixed $value Value to convert
     * @return string String representation of the value
     */
    private static function valueToString(mixed $value): string
    {
        return match (true) {
            is_string($value) => $value,
            $value === null => 'null',
            is_bool($value) => $value ? 'true' : 'false',
            is_array($value) => 'array',
            is_object($value) => get_class($value),
            is_int($value) => (string) $value,
            is_float($value) => (string) $value,
            default => 'unknown',
        };
    }

    /**
     * Create ValidationError for content that's too short.
     *
     * @param int $actual Actual content length
     * @param int $minimum Minimum required length
     * @param string $validator Validator name that detected the error
     * @return self
     *
     * @example
     * ```php
     * $length = strlen($result['content']);
     * if ($length < 100) {
     *     throw ValidationError::contentTooShort($length, 100, 'min_length');
     * }
     * ```
     */
    public static function contentTooShort(
        int $actual,
        int $minimum,
        string $validator = 'min_length',
    ): self {
        return new self(
            message: "Content too short: {$actual} < {$minimum} characters",
            details: [
                'field' => 'content',
                'actual_length' => $actual,
                'minimum_length' => $minimum,
                'error' => 'too_short',
                'validator' => $validator,
            ],
        );
    }

    /**
     * Create ValidationError for content that's too long.
     *
     * @param int $actual Actual content length
     * @param int $maximum Maximum allowed length
     * @param string $validator Validator name that detected the error
     * @return self
     *
     * @example
     * ```php
     * $length = strlen($result['content']);
     * if ($length > 10000) {
     *     throw ValidationError::contentTooLong($length, 10000, 'max_length');
     * }
     * ```
     */
    public static function contentTooLong(
        int $actual,
        int $maximum,
        string $validator = 'max_length',
    ): self {
        return new self(
            message: "Content too long: {$actual} > {$maximum} characters",
            details: [
                'field' => 'content',
                'actual_length' => $actual,
                'maximum_length' => $maximum,
                'error' => 'too_long',
                'validator' => $validator,
            ],
        );
    }
}
