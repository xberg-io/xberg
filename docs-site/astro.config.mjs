// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";
import { xbergStarlightConfig } from "@xberg-io/docs-theme";

const API_LANGUAGES = [
  { label: "Python", slug: "reference/api-python" },
  { label: "TypeScript / Node.js", slug: "reference/api-typescript" },
  { label: "WebAssembly", slug: "reference/api-wasm" },
  { label: "Rust", slug: "reference/api-rust" },
  { label: "Go", slug: "reference/api-go" },
  { label: "Java", slug: "reference/api-java" },
  { label: "Kotlin (Android)", slug: "reference/api-kotlin-android" },
  { label: "C#", slug: "reference/api-csharp" },
  { label: "Swift", slug: "reference/api-swift" },
  { label: "Ruby", slug: "reference/api-ruby" },
  { label: "PHP", slug: "reference/api-php" },
  { label: "Elixir", slug: "reference/api-elixir" },
  { label: "Dart", slug: "reference/api-dart" },
  { label: "Zig", slug: "reference/api-zig" },
  { label: "C", slug: "reference/api-c" },
];

export default defineConfig({
  site: "https://docs.xberg.io",
  integrations: [
    starlight(
      xbergStarlightConfig({
        title: "Xberg",
        description:
          "Full content intelligence engine: extract text, tables, entities, and embeddings from " +
          "96+ formats with OCR, transcription, code intelligence, and LLM integration. Native " +
          "bindings for 15 languages.",
        githubUrl: "https://github.com/xberg-io/xberg",
        editBaseUrl: "https://github.com/xberg-io/xberg/edit/main/docs-site/",
        plugins: [
          starlightLlmsTxt({
            customSets: [
              {
                label: "Get Started",
                description: "Installation and quick-start guides.",
                paths: ["getting-started/**"],
              },
              {
                label: "Guides",
                description:
                  "Task-oriented guides: extraction, configuration, OCR, chunking, embeddings, " +
                  "transcription, structured extraction, code intelligence, and deployment.",
                paths: ["guides/**", "cli/**"],
              },
              {
                label: "Concepts",
                description: "Architecture, the extraction pipeline, and the plugin system.",
                paths: ["concepts/**", "features"],
              },
              {
                label: "Integrations",
                description: "Connecting Xberg to Open WebUI, SurrealDB, and other tools.",
                paths: ["integrations/**"],
              },
              {
                label: "Reference",
                description:
                  "Per-language API docs, configuration schema, types, errors, formats, and CLI/MCP " + "reference.",
                paths: ["reference/**"],
              },
              {
                label: "More",
                description: "Migration, changelog, contributing, and ecosystem.",
                paths: ["migration/**", "changelog", "contributing", "ecosystem"],
              },
            ],
            optionalLinks: [
              {
                label: "GitHub",
                url: "https://github.com/xberg-io/xberg",
                description: "Source code and issues",
              },
            ],
          }),
        ],
        sidebar: [
          { label: "Home", link: "/" },
          {
            label: "Get Started",
            items: [
              { label: "Installation", slug: "getting-started/installation" },
              { label: "Quick Start", slug: "getting-started/quickstart" },
              { label: "Live Demo", link: "/demo.html", attrs: { target: "_blank" } },
            ],
          },
          {
            label: "Guides",
            items: [
              {
                label: "Core",
                items: [
                  { label: "Extraction Basics", slug: "guides/extraction" },
                  { label: "Configuration", slug: "guides/configuration" },
                  { label: "Rust Core API", slug: "guides/rust-core-api" },
                  { label: "Output Formats", slug: "guides/output-formats" },
                  { label: "OCR", slug: "guides/ocr" },
                  { label: "HTML Output", slug: "guides/html-output" },
                ],
              },
              {
                label: "Advanced",
                items: [
                  { label: "Chunking", slug: "guides/chunking" },
                  { label: "Embeddings", slug: "guides/embeddings" },
                  { label: "Reranking", slug: "guides/reranking" },
                  { label: "Keyword Extraction", slug: "guides/keywords" },
                  { label: "Language Detection", slug: "guides/language-detection" },
                  { label: "Token Reduction", slug: "guides/token-reduction" },
                  { label: "Quality Processing", slug: "guides/quality-processing" },
                  { label: "URL Extraction", slug: "guides/url-extraction" },
                  { label: "PDF Form Fields", slug: "guides/pdf-form-fields" },
                  { label: "Document Splitting", slug: "guides/document-splitting" },
                  { label: "Layout Detection", slug: "guides/layout-detection" },
                  { label: "Audio and Video Transcription", slug: "guides/transcription" },
                  { label: "Presets", slug: "guides/presets" },
                  { label: "Structured Extraction", slug: "guides/structured-extraction" },
                  { label: "LLM Integration", slug: "guides/llm-integration" },
                  { label: "Code Intelligence", slug: "guides/code-intelligence" },
                  { label: "Named-Entity Recognition", slug: "guides/ner" },
                  { label: "Redaction & Anonymisation", slug: "guides/redaction" },
                  { label: "Document Summarisation", slug: "guides/summarization" },
                  { label: "Document Translation", slug: "guides/translation" },
                  { label: "Page Classification", slug: "guides/page-classification" },
                  { label: "VLM Image Captions", slug: "guides/image-captions" },
                  { label: "QR-Code Detection", slug: "guides/qr-codes" },
                ],
              },
              {
                label: "Deployment",
                items: [
                  { label: "Docker", slug: "guides/docker" },
                  { label: "API Server", slug: "guides/api-server" },
                  { label: "MCP Integration", slug: "guides/mcp-integration" },
                  { label: "AI Coding Assistants", slug: "guides/ai-coding-assistants" },
                ],
              },
              {
                label: "Integrations",
                items: [
                  { label: "Overview", slug: "integrations" },
                  { label: "Open WebUI", slug: "integrations/openwebui" },
                  { label: "SurrealDB", slug: "integrations/surrealdb" },
                ],
              },
              { label: "CLI", slug: "cli/usage" },
              {
                label: "Development",
                items: [
                  { label: "Creating Plugins", slug: "guides/plugins" },
                  { label: "Development Workflow", slug: "guides/development" },
                ],
              },
            ],
          },
          {
            label: "Concepts",
            items: [
              { label: "Architecture", slug: "concepts/architecture" },
              { label: "Extraction Pipeline", slug: "concepts/extraction-pipeline" },
              { label: "Plugin System", slug: "concepts/plugin-system" },
              { label: "Platform Support", slug: "concepts/platform-support" },
              { label: "Features", slug: "features" },
            ],
          },
          {
            label: "Reference",
            items: [
              { label: "API", items: API_LANGUAGES },
              { label: "Types", slug: "reference/types" },
              { label: "Configuration", slug: "reference/configuration" },
              { label: "Errors", slug: "reference/errors" },
              { label: "Environment Variables", slug: "reference/environment-variables" },
              { label: "File Size Limits", slug: "reference/file-size-limits" },
              { label: "Format Support", slug: "reference/formats" },
              { label: "OCR Languages", slug: "reference/ocr-languages" },
              { label: "Model Sources", slug: "reference/model-sources" },
              { label: "HTML Styling Contract", slug: "reference/html-styling-contract" },
              { label: "CLI Reference", slug: "reference/cli" },
              { label: "MCP Reference", slug: "reference/mcp" },
              { label: "Benchmarks", link: "https://xberg.io/benchmarks" },
            ],
          },
          {
            label: "More",
            items: [
              {
                label: "Migration",
                items: [{ label: "From Unstructured", slug: "migration/from-unstructured" }],
              },
              { label: "Changelog", slug: "changelog" },
              { label: "Contributing", slug: "contributing" },
              { label: "Ecosystem", slug: "ecosystem" },
            ],
          },
        ],
      }),
    ),
  ],
});
