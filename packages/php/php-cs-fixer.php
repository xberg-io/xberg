<?php

declare(strict_types=1);

$finder = (new PhpCsFixer\Finder())
    ->in(array_filter([
        __DIR__ . '/src',
        is_dir(__DIR__ . '/tests') ? __DIR__ . '/tests' : null,
    ]));

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
