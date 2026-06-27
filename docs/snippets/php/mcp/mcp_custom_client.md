```php title="PHP"
<?php

declare(strict_types=1);

$descriptors = [
    0 => ['pipe', 'r'],
    1 => ['pipe', 'w'],
    2 => ['pipe', 'w'],
];

$process = proc_open(['xberg', 'mcp'], $descriptors, $pipes);
if (!is_resource($process)) {
    throw new RuntimeException('failed to spawn xberg mcp');
}

$request = [
    'method' => 'tools/call',
    'params' => [
        'name' => 'extract',
        'arguments' => [
            'path' => 'document.pdf',
            'async' => true,
        ],
    ],
];

fwrite($pipes[0], json_encode($request, JSON_THROW_ON_ERROR) . "\n");
fclose($pipes[0]);

$response = fgets($pipes[1]);
if ($response !== false) {
    echo $response;
}

fclose($pipes[1]);
fclose($pipes[2]);
proc_close($process);
```
