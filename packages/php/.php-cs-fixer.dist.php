<?php

declare(strict_types=1);

// Stub files declare classes the native extension provides at runtime.
// They contain ext-php-rs-style scaffolding that php-cs-fixer's @PHP82Migration
// rule would otherwise rewrite into constructor-promoted properties, deleting
// the explicit class-level property declarations phpstan needs to see.
// Excluding stubs/ keeps the stub structure intact for static analysis.
$finder = (new PhpCsFixer\Finder())
    ->in(array_filter([
        __DIR__ . '/src',
        is_dir(__DIR__ . '/tests') ? __DIR__ . '/tests' : null,
    ]))
    ->notPath('stubs');

return (new PhpCsFixer\Config())
    ->setRules([
        '@PSR12' => true,
        '@PHP82Migration' => true,
        'array_syntax' => ['syntax' => 'short'],
        'single_quote' => true,
        'trailing_comma_in_multiline' => [
            'elements' => ['arrays', 'arguments', 'parameters'],
        ],
        'declare_strict_types' => true,
        'ordered_imports' => ['sort_algorithm' => 'alpha'],
        'no_unused_imports' => true,
    ])
    ->setFinder($finder)
    ->setRiskyAllowed(true);
