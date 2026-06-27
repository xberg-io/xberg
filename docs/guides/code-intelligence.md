# Code Intelligence

Xberg integrates [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.xberg.io) (TSLP) to parse and analyze source code files. When you extract a source code file, Xberg automatically detects the programming language and produces structured analysis alongside the raw text content.

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

Language support covers **306 programming languages** via tree-sitter grammars. See the [TSLP documentation](https://docs.tree-sitter-language-pack.xberg.io) for the full language list.

## Getting Started

Code intelligence is enabled by default when the `tree-sitter` feature flag is active. Simply extract a source code file:

=== "Rust"

    ```rust title="basic.rs"
    use xberg::{extract, ExtractionConfig};

    let config = ExtractionConfig::default();
    let result = extract("app.py", None, &config)?;

    // The content field has the raw source text
    println!("{}", result.content);

    // Code intelligence is in metadata.format
    if let Some(xberg::types::FormatMetadata::Code(ref code)) = result.metadata.format {
        println!("Language: {}", code.language);
        println!("Structures: {}", code.structure.len());
        println!("Imports: {}", code.imports.len());
    }
    ```

=== "Python"

    ```python title="basic.py"
    import xberg

    config = xberg.ExtractionConfig()
    result = xberg.extract("app.py", config=config)

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
    import { extractFileSync } from "@xberg-io/xberg";

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
    result, err := xberg.ExtractFileSync("app.py", nil)
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
    use xberg::{ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig};

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
    import xberg

    config = xberg.ExtractionConfig(
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
    import { ExtractionConfig } from "@xberg-io/xberg";

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

    ```toml title="xberg.toml"
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

See [`TreeSitterConfig`](../reference/configuration.md#treesitterconfig) and [`TreeSitterProcessConfig`](../reference/configuration.md#treesitterprocessconfig) for all fields.

## ProcessResult Fields

Code intelligence results are returned as a `ProcessResult` from the upstream [`tree-sitter-language-pack`](https://docs.rs/tree-sitter-language-pack) crate. Top-level fields: `language`, `metrics`, `structure`, `imports`, `exports`, `chunks`, plus `comments` / `docstrings` / `symbols` / `diagnostics` (populated only when their `TreeSitterProcessConfig` flag is on). See the upstream crate docs for full field shapes.

## Semantic Chunking for RAG

Code chunks produced by tree-sitter are semantically aware -- they split at function, class, and module boundaries rather than fixed line counts. This makes them ideal for retrieval-augmented generation (RAG) pipelines:

```python title="rag_chunking.py"
import xberg

config = xberg.ExtractionConfig(
    tree_sitter={"process": {"chunk_max_size": 2048}}
)

result = xberg.extract("large_module.py", config=config)

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

Xberg detects the programming language in two ways:

1. **File extension** (fast path) -- when using `extract`, the extension is matched against 248 known language extensions
2. **Shebang line** (fallback) -- when using `extract` or when the extension is ambiguous, the first line is checked for `#!/usr/bin/env python`, `#!/bin/bash`, and so on.

If neither method identifies the language, extraction returns an `UnsupportedFormat` error.

## Language Support

Tree-sitter-language-pack supports 306 programming languages. For the full list, see the [TSLP language reference](https://docs.tree-sitter-language-pack.xberg.io).

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
- [tree-sitter-language-pack documentation](https://docs.tree-sitter-language-pack.xberg.io) -- Full language support reference
