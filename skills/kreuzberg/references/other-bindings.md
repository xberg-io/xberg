# Language Bindings Reference

Kreuzberg provides native bindings for multiple programming languages, each with precompiled binaries for x86_64 and aarch64 on Linux and macOS. This reference covers installation and basic usage for each binding.

## Go

**Installation:**
```bash
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4
```

**Basic Extraction:**
```go
package main

import (
    "context"
    "fmt"
    "github.com/kreuzberg-dev/kreuzberg/packages/go/v4/kreuzberg"
)

func main() {
    ctx := context.Background()
    result, err := kreuzberg.ExtractFile(ctx, "document.pdf", nil)
    if err != nil {
        panic(err)
    }
    fmt.Println(result.Content)
}
```

See the [Go binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go) for complete API reference.

## Ruby

**Installation:**
```bash
gem install kreuzberg
```

Or in your Gemfile:
```ruby
gem 'kreuzberg'
```

**Basic Extraction:**
```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')
puts result.content
```

See the [Ruby binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/ruby) for complete API reference.

## Java

**Installation:**
Add to your Maven `pom.xml`:
```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.2.x</version>
</dependency>
```

**Basic Extraction:**
```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;

public class Example {
    public static void main(String[] args) throws Exception {
        ExtractionResult result = Kreuzberg.extractFile("document.pdf");
        System.out.println(result.getContent());
    }
}
```

See the [Java binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/java) for complete API reference.

## C#

**Installation:**
```bash
dotnet add package Kreuzberg
```

**Basic Extraction:**
```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
```

See the [C# binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/csharp) for complete API reference.

## PHP

**Installation:**
```bash
composer require kreuzberg/kreuzberg
```

**Basic Extraction:**
```php
<?php
require 'vendor/autoload.php';

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');
echo $result->content;
```

See the [PHP binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/php) for complete API reference.

## Elixir

**Installation:**
Add to your `mix.exs` dependencies:
```elixir
def deps do
  [
    kreuzberg: "~> 4.2"
  ]
end
```

**Basic Extraction:**
```elixir
{:ok, result} = Kreuzberg.extract_file("document.pdf")
IO.puts(result.content)
```

See the [Elixir binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/elixir) for complete API reference.

## WebAssembly (WASM)

**Installation:**
```bash
npm install @kreuzberg/wasm
```

**Basic Extraction:**
```typescript
import { extractBytes } from '@kreuzberg/wasm';

const fileData = await fs.promises.readFile('document.pdf');
const result = await extractBytes(fileData, 'application/pdf');
console.log(result.content);
```

Supports browsers, Deno, and Cloudflare Workers.

See the [WASM binding documentation](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/typescript) for complete API reference.

## Docker

**Installation:**
Pull the official image from GitHub Container Registry:
```bash
docker pull ghcr.io/kreuzberg-dev/kreuzberg
```

**API Server Mode:**
```bash
docker run -p 8000:8000 ghcr.io/kreuzberg-dev/kreuzberg serve --host 0.0.0.0
```

**CLI Mode:**
```bash
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg extract /data/document.pdf
```

**MCP Server Mode:**
```bash
docker run -i ghcr.io/kreuzberg-dev/kreuzberg mcp
```

Image sizes:
- Core image: 1.0-1.3GB
- Full image (with LibreOffice): 1.5-2.1GB

See the [Docker guide](https://docs.kreuzberg.dev/guides/docker/) for deployment details.

## Platform Support

All language bindings include precompiled binaries for x86_64 and aarch64 on Linux and macOS. Windows support varies by binding. Refer to the main [README](https://github.com/kreuzberg-dev/kreuzberg) for platform compatibility matrix.
