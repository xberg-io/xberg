---
title: "n8n"
---

The `@xberg-io/n8n-nodes-xberg` package is a community node that runs Xberg document extraction inside your [n8n](https://n8n.io/) workflows. It uses the native `@xberg-io/xberg` binding, so extraction happens in-process — no API key, no external call, and no data leaves your n8n instance.

[![npm](https://img.shields.io/npm/v/@xberg-io/n8n-nodes-xberg)](https://www.npmjs.com/package/@xberg-io/n8n-nodes-xberg)
[![License](https://img.shields.io/npm/l/@xberg-io/n8n-nodes-xberg)](https://github.com/xberg-io/xberg/blob/main/LICENSE)

Because it ships a native N-API addon, the node runs on **self-hosted n8n only** — n8n Cloud does not load native addons, so this package is not among the Cloud-verified nodes.

## How it works

1. **Feed** — A workflow passes an uploaded file (binary) or a URL/path to the node.
2. **Extract** — Xberg parses the document in-process, running OCR where needed.
3. **Emit** — The node returns extracted content, metadata, tables, and optional chunks as item JSON, ready for the rest of your workflow.

## Installation

In your self-hosted n8n instance, open **Settings → Community Nodes → Install**, enter the package name, and confirm you accept the risks of running community code:

```text
@xberg-io/n8n-nodes-xberg
```

Or install it manually:

```bash
npm install @xberg-io/n8n-nodes-xberg
```

Node.js 22+ is required. Prebuilt binaries ship for Linux (x64/arm64, glibc or musl), macOS (arm64), and Windows (x64/arm64).

## Operations

The node has one resource, **Document**, with three operations.

| Operation | Description |
| ------------- | ------------------------------------------------------------------------------------------ |
| Extract       | Extract text, tables, and metadata from one document per item.                             |
| Extract Batch | Extract every incoming item in a single batch call instead of looping.                     |
| Map URL       | List the URLs reachable from a web page or sitemap without extracting them.                 |

**Extract** and **Extract Batch** accept either uploaded binary data or a URL/local path via the **Input Source** parameter, and expose the key `ExtractionConfig` options: output format (Markdown, plain, HTML, Djot, JSON tree, or DocTags), OCR (enable, force, languages), chunking (size and overlap), image extraction, and quality processing. Per-item failures respect n8n's **Continue On Fail** setting.

## Output

Each Extract output item's JSON holds the extracted content (default field `text`), plus `mimeType`, `extractionMethod`, `detectedLanguages`, and `counts` (pages, tables, images). `metadata`, `tables`, and `chunks` are attached when their toggles are set, and `formattedContent`, `qualityScore`, `entities`, and `summary` appear when populated. Enabling **Return As Binary** also attaches the content as a binary property.

For the full parameter reference and workflow examples, see the [package readme](https://github.com/xberg-io/xberg/tree/main/integrations/node/n8n-nodes-xberg). For the list of supported file types, see [format support](/reference/formats/).
