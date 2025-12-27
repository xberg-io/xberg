<?php

declare(strict_types=1);

namespace Kreuzberg\Plugins;

use Kreuzberg\Exceptions\KreuzbergException;

/**
 * Registry for managing validator plugins.
 *
 * This class provides a high-level object-oriented API for registering and managing
 * validators. It wraps the low-level procedural functions (`kreuzberg_register_validator`,
 * `kreuzberg_unregister_validator`, etc.) with a more convenient interface.
 *
 * @example
 * ```php
 * use Kreuzberg\Plugins\ValidatorRegistry;
 * use Kreuzberg\Plugins\ValidatorInterface;
 * use Kreuzberg\Plugins\ValidationError;
 *
 * // Create a validator
 * class MinLengthValidator implements ValidatorInterface
 * {
 *     public function __construct(private int $minLength = 100) {}
 *
 *     public function validate(array $result): bool
 *     {
 *         if (strlen($result['content']) < $this->minLength) {
 *             throw ValidationError::contentTooShort(
 *                 strlen($result['content']),
 *                 $this->minLength
 *             );
 *         }
 *         return true;
 *     }
 * }
 *
 * // Register using the registry
 * $registry = ValidatorRegistry::getInstance();
 * $validator = new MinLengthValidator(minLength: 200);
 * $registry->register('min_length', $validator);
 *
 * // List validators
 * $validators = $registry->list();
 * print_r($validators); // ["min_length"]
 *
 * // Unregister
 * $registry->unregister('min_length');
 *
 * // Clear all
 * $registry->clear();
 * ```
 */
final class ValidatorRegistry
{
    private static ?self $instance = null;

    /**
     * Private constructor to enforce singleton pattern.
     */
    private function __construct()
    {
    }

    /**
     * Get the singleton instance.
     *
     * @return self
     */
    public static function getInstance(): self
    {
        if (self::$instance === null) {
            self::$instance = new self();
        }

        return self::$instance;
    }

    /**
     * Register a validator.
     *
     * The validator can be:
     * - An instance of ValidatorInterface
     * - A callable that accepts an array and returns bool
     * - An array like [$object, 'methodName']
     *
     * @param string $name Unique validator name
     * @param ValidatorInterface|callable $validator Validator instance or callable
     * @return self For method chaining
     * @throws KreuzbergException If registration fails
     *
     * @example
     * ```php
     * // Register a validator object
     * $registry = ValidatorRegistry::getInstance();
     * $validator = new MinLengthValidator();
     * $registry->register('min_length', $validator);
     *
     * // Register a closure
     * $registry->register('custom', function (array $result): bool {
     *     return strlen($result['content']) > 0;
     * });
     *
     * // Method chaining
     * $registry
     *     ->register('validator1', $v1)
     *     ->register('validator2', $v2);
     * ```
     */
    public function register(string $name, ValidatorInterface|callable $validator): self
    {
        if ($validator instanceof ValidatorInterface) {
            $callback = [$validator, 'validate'];
        } elseif (is_callable($validator)) {
            $callback = $validator;
        } else {
            throw new KreuzbergException(
                'Validator must be an instance of ValidatorInterface or a callable',
            );
        }

        \kreuzberg_register_validator($name, $callback);

        return $this;
    }

    /**
     * Unregister a validator by name.
     *
     * @param string $name Validator name to unregister
     * @return self For method chaining
     * @throws KreuzbergException If validator is not registered
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * $registry->unregister('min_length');
     * ```
     */
    public function unregister(string $name): self
    {
        \kreuzberg_unregister_validator($name);

        return $this;
    }

    /**
     * Check if a validator is registered.
     *
     * @param string $name Validator name to check
     * @return bool True if registered, false otherwise
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * if ($registry->has('min_length')) {
     *     echo "Validator is registered\n";
     * }
     * ```
     */
    public function has(string $name): bool
    {
        return in_array($name, $this->list(), true);
    }

    /**
     * List all registered validator names.
     *
     * @return array<string> List of validator names
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * $validators = $registry->list();
     * foreach ($validators as $name) {
     *     echo "Validator: {$name}\n";
     * }
     * ```
     */
    public function list(): array
    {
        return \kreuzberg_list_validators();
    }

    /**
     * Get the count of registered validators.
     *
     * @return int Number of registered validators
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * echo "Validators registered: " . $registry->count() . "\n";
     * ```
     */
    public function count(): int
    {
        return count($this->list());
    }

    /**
     * Clear all registered validators.
     *
     * @return self For method chaining
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * $registry->clear();
     * ```
     */
    public function clear(): self
    {
        \kreuzberg_clear_validators();

        return $this;
    }

    /**
     * Register multiple validators at once.
     *
     * @param array<string, ValidatorInterface|callable> $validators Map of name => validator
     * @return self For method chaining
     * @throws KreuzbergException If any registration fails
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * $registry->registerMany([
     *     'min_length' => new MinLengthValidator(),
     *     'max_length' => new MaxLengthValidator(),
     *     'custom' => fn($r) => strlen($r['content']) > 0,
     * ]);
     * ```
     */
    public function registerMany(array $validators): self
    {
        foreach ($validators as $name => $validator) {
            $this->register($name, $validator);
        }

        return $this;
    }

    /**
     * Unregister multiple validators at once.
     *
     * @param array<string> $names Validator names to unregister
     * @return self For method chaining
     * @throws KreuzbergException If any unregistration fails
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     * $registry->unregisterMany(['min_length', 'max_length']);
     * ```
     */
    public function unregisterMany(array $names): self
    {
        foreach ($names as $name) {
            $this->unregister($name);
        }

        return $this;
    }

    /**
     * Create a scoped validator registration.
     *
     * Registers validators for the duration of a callback, then automatically
     * unregisters them afterwards. Useful for testing or temporary validation rules.
     *
     * @param array<string, ValidatorInterface|callable> $validators Validators to register temporarily
     * @param callable $callback Callback to execute with validators active
     * @return mixed Return value of the callback
     *
     * @example
     * ```php
     * $registry = ValidatorRegistry::getInstance();
     *
     * $result = $registry->withValidators(
     *     validators: [
     *         'min_length' => new MinLengthValidator(100),
     *         'max_length' => new MaxLengthValidator(1000),
     *     ],
     *     callback: function () {
     *         // These validators are active only within this callback
     *         return extract_file('document.pdf');
     *     }
     * );
     * // Validators are automatically unregistered here
     * ```
     */
    public function withValidators(array $validators, callable $callback): mixed
    {
        $previousValidators = $this->list();

        try {
            $this->registerMany($validators);

            return $callback();
        } finally {
            foreach (array_keys($validators) as $name) {
                if (!in_array($name, $previousValidators, true)) {
                    try {
                        $this->unregister($name);
                    } catch (\Exception $e) {
                    }
                }
            }
        }
    }
}
