# Code Intelligence

Kreuzberg integrates [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) (TSLP) to parse and analyze source code files. When you extract a source code file, Kreuzberg automatically detects the programming language and produces structured analysis alongside the raw text content.

## What You Get

When extracting source code, the `metadata.format` field contains a `ProcessResult` (format type `"code"`) with:

- **Structure** -- functions, classes, structs, methods, modules, and their nesting hierarchy
- **Imports** -- import/include/require statements with source paths and imported items
- **Exports** -- exported symbols with their kinds (function, class, variable, type, default)
- **Comments** -- inline and block comments with their positions
- **Docstrings** -- documentation comments with parsed sections (params, returns, etc.)
- **Symbols** -- variable, constant, and type alias definitions
- **Diagnostics** -- parse errors and warnings from tree-sitter
- **Chunks** -- semantically meaningful code chunks for RAG and embedding pipelines
- **Metrics** -- file-level statistics (lines of code, comment lines, empty lines, node count)

Language support covers **248 programming languages** via tree-sitter grammars. See the [TSLP documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev) for the full language list.

## Getting Started

Code intelligence is enabled by default when the `tree-sitter` feature flag is active. Simply extract a source code file:

=== "Rust"

    ```rust title="basic.rs"
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    let config = ExtractionConfig::default();
    let result = extract_file_sync("app.py", None, &config)?;

    // The content field has the raw source text
    println!("{}", result.content);

    // Code intelligence is in metadata.format
    if let Some(kreuzberg::types::FormatMetadata::Code(ref code)) = result.metadata.format {
        println!("Language: {}", code.language);
        println!("Structures: {}", code.structure.len());
        println!("Imports: {}", code.imports.len());
    }
    ```

=== "Python"

    ```python title="basic.py"
    import kreuzberg

    result = kreuzberg.extract_file_sync("app.py")

    # The content field has the raw source text
    print(result.content)

    # Code intelligence is in metadata["format"]
    fmt = result.metadata.get("format")
    if fmt and fmt.get("format_type") == "code":
        print(f"Language: {fmt['language']}")
        print(f"Structures: {len(fmt['structure'])}")
        print(f"Imports: {len(fmt['imports'])}")
    ```

=== "TypeScript"

    ```typescript title="basic.ts"
    import { extractFileSync } from "@kreuzberg/node";

    const result = extractFileSync("app.ts");

    console.log(result.content);

    const fmt = result.metadata?.format;
    if (fmt?.formatType === "code") {
      console.log(`Language: ${fmt.language}`);
      console.log(`Structures: ${fmt.structure.length}`);
      console.log(`Imports: ${fmt.imports.length}`);
    }
    ```

=== "Go"

    ```go title="basic.go"
    result, err := kreuzberg.ExtractFileSync("app.py", nil)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Println(result.Content)
    // Code intelligence is available in result.Metadata.Format
    // when Format.Type == "code"
    ```

## Configuration

Use `TreeSitterConfig` to control which analysis features are enabled. Set `enabled: false` to disable code intelligence entirely. By default, `structure`, `imports`, and `exports` are enabled; `comments`, `docstrings`, `symbols`, and `diagnostics` are disabled.

=== "Rust"

    ```rust title="config.rs"
    use kreuzberg::{ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig};

    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig {
            process: TreeSitterProcessConfig {
                structure: true,      // functions, classes, etc. (default: true)
                imports: true,        // import statements (default: true)
                exports: true,        // export statements (default: true)
                comments: true,       // comments (default: false)
                docstrings: true,     // docstrings (default: false)
                symbols: true,        // variables, constants (default: false)
                diagnostics: true,    // parse errors/warnings (default: false)
                chunk_max_size: Some(4096),  // max chunk size in bytes
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Python"

    ```python title="config.py"
    import kreuzberg

    config = kreuzberg.ExtractionConfig(
        tree_sitter={
            "process": {
                "structure": True,
                "imports": True,
                "exports": True,
                "comments": True,
                "docstrings": True,
                "symbols": True,
                "diagnostics": True,
                "chunk_max_size": 4096,
            }
        }
    )
    ```

=== "TypeScript"

    ```typescript title="config.ts"
    import { ExtractionConfig } from "@kreuzberg/node";

    const config: ExtractionConfig = {
      treeSitter: {
        process: {
          structure: true,
          imports: true,
          exports: true,
          comments: true,
          docstrings: true,
          symbols: true,
          diagnostics: true,
          chunkMaxSize: 4096,
        },
      },
    };
    ```

=== "TOML"

    ```toml title="kreuzberg.toml"
    [tree_sitter.process]
    structure = true
    imports = true
    exports = true
    comments = true
    docstrings = true
    symbols = true
    diagnostics = true
    chunk_max_size = 4096
    ```

### Configuration Fields

| Field            | Type              | Default  | Description                                                              |
| ---------------- | ----------------- | -------- | ------------------------------------------------------------------------ |
| `structure`      | `bool`            | `true`   | Extract structural items (functions, classes, structs, etc.)             |
| `imports`        | `bool`            | `true`   | Extract import/include/require statements                                |
| `exports`        | `bool`            | `true`   | Extract export statements                                                |
| `comments`       | `bool`            | `false`  | Extract comments                                                         |
| `docstrings`     | `bool`            | `false`  | Extract docstrings with parsed sections                                  |
| `symbols`        | `bool`            | `false`  | Extract symbol definitions (variables, constants, type aliases)          |
| `diagnostics`    | `bool`            | `false`  | Include parse diagnostics (errors and warnings)                          |
| `chunk_max_size` | `usize?`          | `None`   | Maximum chunk size in bytes. `None` uses the default chunk size          |
| `content_mode`   | `CodeContentMode` | `chunks` | Controls how code content is rendered in the `content` field (see below) |

### CodeContentMode

Controls how the extracted `content` field is populated for code files:

| Value       | Description                                                                  |
| ----------- | ---------------------------------------------------------------------------- |
| `chunks`    | Semantic chunks joined as content (default). Best for RAG and embeddings.    |
| `raw`       | Raw source code as-is. Best when you need the original file content.         |
| `structure` | Function/class headings with docstrings, no code bodies. Best for summaries. |

## ProcessResult Fields

The `ProcessResult` (accessed via `FormatMetadata::Code` in Rust, or `metadata.format` in other languages) contains:

### `language`

The detected programming language name (for example, `"python"`, `"rust"`, `"typescript"`).

### `metrics`

File-level statistics:

| Field           | Type    | Description                     |
| --------------- | ------- | ------------------------------- |
| `total_lines`   | `usize` | Total number of lines           |
| `code_lines`    | `usize` | Lines containing code           |
| `comment_lines` | `usize` | Lines containing comments       |
| `blank_lines`   | `usize` | Empty or whitespace-only lines  |
| `total_bytes`   | `usize` | Total file size in bytes        |
| `node_count`    | `usize` | Number of tree-sitter AST nodes |
| `error_count`   | `usize` | Number of parse error nodes     |
| `max_depth`     | `usize` | Maximum AST nesting depth       |

### `structure`

A tree of structural items (functions, classes, methods, modules, etc.). Each `StructureItem` has:

- `kind` -- the type of structure (`function`, `class`, `struct`, `method`, `module`, `interface`, `enum`, `trait`, `impl`, etc.)
- `name` -- the identifier name (if available)
- `visibility` -- visibility modifier (`public`, `private`, `protected`, etc.)
- `span` -- source location (start/end line, column, byte offset)
- `children` -- nested structural items (for example, methods inside a class)
- `decorators` -- decorator/attribute names
- `doc_comment` -- associated documentation comment text
- `signature` -- function/method signature string
- `body_span` -- span of the body (excluding signature)

### `imports`

Import/include/require statements. Each `ImportInfo` has:

- `source` -- the module path or file being imported
- `items` -- specific items imported (for example, `["foo", "bar"]`)
- `alias` -- import alias (for example, `import numpy as np` yields alias `"np"`)
- `is_wildcard` -- whether this is a wildcard import (`import *`)
- `span` -- source location

### `exports`

Exported symbols. Each `ExportInfo` has:

- `name` -- the exported symbol name
- `kind` -- export kind (`function`, `class`, `variable`, `type`, `default`)
- `span` -- source location

### `chunks`

Semantically meaningful code chunks, suitable for RAG and embedding pipelines. Each `CodeChunk` has:

- `content` -- the chunk text
- `language` -- the programming language
- `span` -- source location
- `context` -- optional parent context (`parent_name`, `parent_kind`)

Chunks are split at structural boundaries (function/class boundaries) rather than arbitrary line counts, preserving semantic coherence.

### `comments`, `docstrings`, `symbols`, `diagnostics`

These fields are populated only when their corresponding configuration flags are enabled.

## Semantic Chunking for RAG

Code chunks produced by tree-sitter are semantically aware -- they split at function, class, and module boundaries rather than fixed line counts. This makes them ideal for retrieval-augmented generation (RAG) pipelines:

```python title="rag_chunking.py"
import kreuzberg

config = kreuzberg.ExtractionConfig(
    tree_sitter={"process": {"chunk_max_size": 2048}}
)

result = kreuzberg.extract_file_sync("large_module.py", config=config)

fmt = result.metadata.get("format")
if fmt and fmt.get("format_type") == "code":
    for chunk in fmt.get("chunks", []):
        # Each chunk is a semantically coherent piece of code
        embedding = your_embedding_model(chunk["content"])
        store_in_vector_db(
            text=chunk["content"],
            embedding=embedding,
            metadata={
                "language": chunk["language"],
                "start_line": chunk["span"]["start_line"],
                "parent": chunk.get("context", {}).get("parent_name"),
            },
        )
```

## Language Detection

Kreuzberg detects the programming language in two ways:

1. **File extension** (fast path) -- when using `extract_file`, the extension is matched against 248 known language extensions
2. **Shebang line** (fallback) -- when using `extract_bytes` or when the extension is ambiguous, the first line is checked for `#!/usr/bin/env python`, `#!/bin/bash`, and so on.

If neither method identifies the language, extraction returns an `UnsupportedFormat` error.

## Language Support

Tree-sitter-language-pack supports 248 programming languages. For the full list, see the [TSLP language reference](https://docs.tree-sitter-language-pack.kreuzberg.dev).

Common languages with full structural analysis:

| Language   | Structure | Imports | Exports | Docstrings |
| ---------- | --------- | ------- | ------- | ---------- |
| Python     | Yes       | Yes     | Yes     | Yes        |
| Rust       | Yes       | Yes     | Yes     | Yes        |
| TypeScript | Yes       | Yes     | Yes     | Yes        |
| JavaScript | Yes       | Yes     | Yes     | Yes        |
| Go         | Yes       | Yes     | Yes     | Yes        |
| Java       | Yes       | Yes     | Yes     | Yes        |
| C/C++      | Yes       | Yes     | Yes     | Yes        |
| Ruby       | Yes       | Yes     | Yes     | Yes        |
| PHP        | Yes       | Yes     | Yes     | Yes        |
| C#         | Yes       | Yes     | Yes     | Yes        |
| Swift      | Yes       | Yes     | Yes     | Yes        |
| Kotlin     | Yes       | Yes     | Yes     | Yes        |
| Elixir     | Yes       | Yes     | Yes     | Yes        |

## Related Documentation

- [Configuration Reference](../reference/configuration.md#treesitterconfig) -- TreeSitterConfig and TreeSitterProcessConfig fields
- [Types Reference](../reference/types.md) -- ProcessResult, StructureItem, CodeChunk, and related type definitions
- [tree-sitter-language-pack documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev) -- Full language support reference
