/* tslint:disable */
/* eslint-disable */

/**
 * Hardware acceleration configuration for ONNX Runtime models.
 *
 * Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
 * for inference in layout detection and embedding generation.
 *
 * # Example
 */
export class WasmAccelerationConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmAccelerationConfig;
    constructor(provider?: WasmExecutionProviderType | null, deviceId?: number | null);
    deviceId: number;
    get provider(): string;
    set provider(value: WasmExecutionProviderType);
}

/**
 * Types of inline text annotations.
 */
export class WasmAnnotationKind {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmAnnotationKind;
    constructor();
    annotationType: string;
    get name(): string | undefined;
    set name(value: string | null | undefined);
    get title(): string | undefined;
    set title(value: string | null | undefined);
    get url(): string | undefined;
    set url(value: string | null | undefined);
    get value(): string | undefined;
    set value(value: string | null | undefined);
}

/**
 * A single file extracted from an archive.
 *
 * When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
 * enabled, each processable file produces its own full `ExtractedDocument`.
 */
export class WasmArchiveEntry {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmArchiveEntry;
    constructor(path: string, mimeType: string, result: WasmExtractedDocument);
    mimeType: string;
    path: string;
    result: WasmExtractedDocument;
}

/**
 * Archive (ZIP/TAR/7Z) metadata.
 *
 * Extracted from compressed archive files containing file lists and size information.
 */
export class WasmArchiveMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmArchiveMetadata;
    constructor(format?: string | null, fileCount?: number | null, fileList?: string[] | null, totalSize?: bigint | null, compressedSize?: bigint | null);
    get compressedSize(): bigint | undefined;
    set compressedSize(value: bigint | null | undefined);
    fileCount: number;
    fileList: string[];
    format: string;
    totalSize: bigint;
}

/**
 * The category of a downloaded asset.
 */
export enum WasmAssetCategory {
    Document = 0,
    Image = 1,
    Audio = 2,
    Video = 3,
    Font = 4,
    Stylesheet = 5,
    Script = 6,
    Archive = 7,
    Data = 8,
    Other = 9,
}

/**
 * Authentication configuration.
 */
export class WasmAuthConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmAuthConfig;
    constructor();
    get name(): string | undefined;
    set name(value: string | null | undefined);
    get password(): string | undefined;
    set password(value: string | null | undefined);
    get token(): string | undefined;
    set token(value: string | null | undefined);
    type: string;
    get username(): string | undefined;
    set username(value: string | null | undefined);
    get value(): string | undefined;
    set value(value: string | null | undefined);
}

/**
 * A batch of pages ready for a single vision-LLM call.
 */
export class WasmBatch {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmBatch;
    constructor(pages: WasmPageImage[], userText?: string | null);
    pages: WasmPageImage[];
    get userText(): string | undefined;
    set userText(value: string | null | undefined);
}

/**
 * BibTeX bibliography metadata.
 */
export class WasmBibtexMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmBibtexMetadata;
    constructor(entryCount?: number | null, citationKeys?: string[] | null, authors?: string[] | null, yearRange?: WasmYearRange | null, entryTypes?: any | null);
    authors: string[];
    citationKeys: string[];
    entryCount: number;
    get entryTypes(): any | undefined;
    set entryTypes(value: any | null | undefined);
    get yearRange(): WasmYearRange | undefined;
    set yearRange(value: WasmYearRange | null | undefined);
}

/**
 * Types of block-level elements in Djot.
 */
export enum WasmBlockType {
    Paragraph = 0,
    Heading = 1,
    Blockquote = 2,
    CodeBlock = 3,
    ListItem = 4,
    OrderedList = 5,
    BulletList = 6,
    TaskList = 7,
    DefinitionList = 8,
    DefinitionTerm = 9,
    DefinitionDescription = 10,
    Div = 11,
    Section = 12,
    ThematicBreak = 13,
    RawBlock = 14,
    MathDisplay = 15,
}

/**
 * Bounding box coordinates for element positioning.
 */
export class WasmBoundingBox {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmBoundingBox;
    constructor(x0?: number | null, y0?: number | null, x1?: number | null, y1?: number | null);
    x0: number;
    x1: number;
    y0: number;
    y1: number;
}

/**
 * Browser backend used for JavaScript rendering.
 */
export enum WasmBrowserBackend {
    Chromiumoxide = 0,
    Native = 1,
}

/**
 * Browser fallback configuration.
 */
export class WasmBrowserConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmBrowserConfig;
    constructor(mode?: WasmBrowserMode | null, backend?: WasmBrowserBackend | null, timeout?: bigint | null, wait?: WasmBrowserWait | null, blockUrlPatterns?: string[] | null, captureNetworkEvents?: boolean | null, sessionAffinity?: boolean | null, endpoint?: string | null, waitSelector?: string | null, extraWait?: bigint | null, proxy?: WasmProxyConfig | null, evalScript?: string | null, robotsUserAgent?: string | null);
    get backend(): string;
    set backend(value: WasmBrowserBackend);
    blockUrlPatterns: string[];
    captureNetworkEvents: boolean;
    get endpoint(): string | undefined;
    set endpoint(value: string | null | undefined);
    get evalScript(): string | undefined;
    set evalScript(value: string | null | undefined);
    get extraWait(): bigint | undefined;
    set extraWait(value: bigint | null | undefined);
    get mode(): string;
    set mode(value: WasmBrowserMode);
    get proxy(): WasmProxyConfig | undefined;
    set proxy(value: WasmProxyConfig | null | undefined);
    get robotsUserAgent(): string | undefined;
    set robotsUserAgent(value: string | null | undefined);
    sessionAffinity: boolean;
    get timeout(): bigint | undefined;
    set timeout(value: bigint | null | undefined);
    get wait(): string;
    set wait(value: WasmBrowserWait);
    get waitSelector(): string | undefined;
    set waitSelector(value: string | null | undefined);
}

/**
 * When to use the headless browser fallback.
 */
export enum WasmBrowserMode {
    Auto = 0,
    Always = 1,
    Never = 2,
    Stealth = 3,
}

/**
 * Wait strategy for browser page rendering.
 */
export enum WasmBrowserWait {
    NetworkIdle = 0,
    Selector = 1,
    Fixed = 2,
}

/**
 * Built prompt components ready to send to the vision model.
 */
export class WasmBuiltPrompt {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmBuiltPrompt;
    constructor(system: string, userText?: string | null);
    system: string;
    get userText(): string | undefined;
    set userText(value: string | null | undefined);
}

/**
 * Aggregate statistics for a xberg cache directory.
 */
export class WasmCacheStats {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCacheStats;
    constructor(totalFiles: number, totalSizeMb: number, availableSpaceMb: number, oldestFileAgeDays: number, newestFileAgeDays: number);
    availableSpaceMb: number;
    newestFileAgeDays: number;
    oldestFileAgeDays: number;
    totalFiles: number;
    totalSizeMb: number;
}

/**
 * How a structured-extraction preset is dispatched to the model.
 *
 * This is the preset-facing call mode (the `preferred_call_mode` field of a
 * `Preset`). The structured pipeline has a richer
 * runtime-only decision enum with skip and fallback states; this 3-variant
 * type is the stable, serializable surface presets and bindings depend on.
 */
export enum WasmCallMode {
    TextOnly = 0,
    VisionOnly = 1,
    TextPlusVision = 2,
}

/**
 * Configuration for the VLM captioning post-processor.
 */
export class WasmCaptioningConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCaptioningConfig;
    constructor(llm: WasmLlmConfig, minImageArea: number, prompt?: string | null);
    llm: WasmLlmConfig;
    minImageArea: number;
    get prompt(): string | undefined;
    set prompt(value: string | null | undefined);
}

/**
 * A single changed cell within a table.
 *
 * Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
 * reference it unconditionally, without requiring the `diff` Cargo feature.
 * `crate.diff` re-exports this type verbatim.
 */
export class WasmCellChange {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCellChange;
    constructor(row: number, col: number, from: string, to: string);
    col: number;
    from: string;
    row: number;
    to: string;
}

/**
 * A text chunk with optional embedding and metadata.
 *
 * Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
 * contains the text content, optional embedding vector (if embedding generation
 * is configured), and metadata about its position in the document.
 */
export class WasmChunk {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmChunk;
    constructor(content: string, chunkType: WasmChunkType, metadata: WasmChunkMetadata, embedding?: Float32Array | null);
    get chunkType(): string;
    set chunkType(value: WasmChunkType);
    content: string;
    get embedding(): Float32Array | undefined;
    set embedding(value: Float32Array | null | undefined);
    metadata: WasmChunkMetadata;
}

/**
 * Metadata about a chunk's position in the original document.
 */
export class WasmChunkMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmChunkMetadata;
    constructor(byteStart: number, byteEnd: number, chunkIndex: number, totalChunks: number, headingPath: string[], imageIndices: Uint32Array, tokenCount?: number | null, firstPage?: number | null, lastPage?: number | null, headingContext?: WasmHeadingContext | null);
    byteEnd: number;
    byteStart: number;
    chunkIndex: number;
    get firstPage(): number | undefined;
    set firstPage(value: number | null | undefined);
    get headingContext(): WasmHeadingContext | undefined;
    set headingContext(value: WasmHeadingContext | null | undefined);
    headingPath: string[];
    imageIndices: Uint32Array;
    get lastPage(): number | undefined;
    set lastPage(value: number | null | undefined);
    get tokenCount(): number | undefined;
    set tokenCount(value: number | null | undefined);
    totalChunks: number;
}

/**
 * How chunk size is measured.
 *
 * Defaults to `Characters` (Unicode character count). When using token-based sizing,
 * chunks are sized by token count according to the specified tokenizer.
 *
 * Token-based sizing uses HuggingFace tokenizers loaded at runtime. Any tokenizer
 * available on HuggingFace Hub can be used, including OpenAI-compatible tokenizers
 * (e.g., `Xenova/gpt-4o`, `Xenova/cl100k_base`).
 */
export class WasmChunkSizing {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmChunkSizing;
    constructor();
    get cacheDir(): string | undefined;
    set cacheDir(value: string | null | undefined);
    get model(): string | undefined;
    set model(value: string | null | undefined);
    type: string;
}

/**
 * Semantic structural classification of a text chunk.
 *
 * Assigned by the heuristic classifier in `chunking.classifier`.
 * Defaults to `Unknown` when no rule matches.
 * Designed to be extended in future versions without breaking changes.
 */
export enum WasmChunkType {
    Heading = 0,
    PartyList = 1,
    Definitions = 2,
    OperativeClause = 3,
    SignatureBlock = 4,
    Schedule = 5,
    TableLike = 6,
    Formula = 7,
    CodeBlock = 8,
    Image = 9,
    OrgChart = 10,
    Diagram = 11,
    Unknown = 12,
}

/**
 * Type of text chunker to use.
 *
 * # Variants
 *
 * * `Text` - Generic text splitter, splits on whitespace and punctuation
 * * `Markdown` - Markdown-aware splitter, preserves formatting and structure
 * * `Yaml` - YAML-aware splitter, creates one chunk per top-level key
 * * `Semantic` - Topic-aware chunker. With an `EmbeddingConfig`, splits at
 *   embedding-based topic shifts tuned by `topic_threshold` (default 0.75,
 *   lower = more splits). Without an embedding, falls back to a
 *   structural-boundary heuristic (ALL-CAPS headers, numbered sections,
 *   blank-line paragraphs) and merges groups into chunks capped at
 *   `max_characters` (default 1000). `topic_threshold` has no effect in the
 *   fallback path. For best results, pair with an embedding model.
 */
export enum WasmChunkerType {
    Text = 0,
    Markdown = 1,
    Yaml = 2,
    Semantic = 3,
}

/**
 * Chunking configuration.
 *
 * Configures text chunking for document content, including chunk size,
 * overlap, trimming behavior, and optional embeddings.
 *
 * Use `..Default.default()` when constructing to allow for future field additions:
 */
export class WasmChunkingConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmChunkingConfig;
    constructor(maxCharacters?: number | null, overlap?: number | null, trim?: boolean | null, chunkerType?: WasmChunkerType | null, sizing?: any | null, prependHeadingContext?: boolean | null, tableChunking?: WasmTableChunkingMode | null, embedding?: WasmEmbeddingConfig | null, preset?: string | null, topicThreshold?: number | null);
    get chunkerType(): string;
    set chunkerType(value: WasmChunkerType);
    get embedding(): WasmEmbeddingConfig | undefined;
    set embedding(value: WasmEmbeddingConfig | null | undefined);
    maxCharacters: number;
    overlap: number;
    prependHeadingContext: boolean;
    get preset(): string | undefined;
    set preset(value: string | null | undefined);
    sizing: any;
    get tableChunking(): string;
    set tableChunking(value: WasmTableChunkingMode);
    get topicThreshold(): number | undefined;
    set topicThreshold(value: number | null | undefined);
    trim: boolean;
}

/**
 * Citation file metadata (RIS, PubMed, EndNote).
 */
export class WasmCitationMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCitationMetadata;
    constructor(citationCount?: number | null, authors?: string[] | null, dois?: string[] | null, keywords?: string[] | null, format?: string | null, yearRange?: WasmYearRange | null);
    authors: string[];
    citationCount: number;
    dois: string[];
    get format(): string | undefined;
    set format(value: string | null | undefined);
    keywords: string[];
    get yearRange(): WasmYearRange | undefined;
    set yearRange(value: WasmYearRange | null | undefined);
}

/**
 * Both views of the fused output: citation-wrapped and flattened.
 */
export class WasmCitationOutput {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCitationOutput;
    constructor(structuredOutput: any, structuredOutputFlat: any);
    structuredOutput: any;
    structuredOutputFlat: any;
}

/**
 * Provenance of a cited field value.
 */
export enum WasmCitationSource {
    Llm = 0,
    Extracted = 1,
    Fused = 2,
    None = 3,
}

/**
 * A single field with its citation envelope.
 */
export class WasmCitedField {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCitedField;
    constructor(value: any, source: WasmCitationSource, page?: number | null, confidence?: number | null);
    get confidence(): number | undefined;
    set confidence(value: number | null | undefined);
    get page(): number | undefined;
    set page(value: number | null | undefined);
    get source(): string;
    set source(value: WasmCitationSource);
    value: any;
}

/**
 * A single label + confidence pair.
 */
export class WasmClassificationLabel {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmClassificationLabel;
    constructor(label: string, confidence?: number | null);
    get confidence(): number | undefined;
    set confidence(value: number | null | undefined);
    label: string;
}

/**
 * Content extraction and conversion configuration.
 *
 * Controls how HTML is converted to the output format. Uses
 * html-to-markdown-rs as the conversion engine for all formats
 * (markdown, plain text, djot).
 */
export class WasmContentConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmContentConfig;
    constructor(outputFormat?: string | null, preprocessingPreset?: string | null, removeNavigation?: boolean | null, removeForms?: boolean | null, stripTags?: string[] | null, preserveTags?: string[] | null, excludeSelectors?: string[] | null, skipImages?: boolean | null, wrap?: boolean | null, wrapWidth?: number | null, includeDocumentStructure?: boolean | null, maxDepth?: number | null);
    excludeSelectors: string[];
    includeDocumentStructure: boolean;
    get maxDepth(): number | undefined;
    set maxDepth(value: number | null | undefined);
    outputFormat: string;
    preprocessingPreset: string;
    preserveTags: string[];
    removeForms: boolean;
    removeNavigation: boolean;
    skipImages: boolean;
    stripTags: string[];
    wrap: boolean;
    wrapWidth: number;
}

/**
 * Cross-extractor content filtering configuration.
 *
 * Controls whether "furniture" content (headers, footers, page numbers,
 * watermarks, repeating text) is included in or stripped from extraction
 * results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
 * with format-specific implementation.
 *
 * When `None` on `ExtractionConfig`, each extractor uses its current
 * default behavior unchanged.
 */
export class WasmContentFilterConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmContentFilterConfig;
    constructor(includeHeaders?: boolean | null, includeFooters?: boolean | null, stripRepeatingText?: boolean | null, includeWatermarks?: boolean | null);
    includeFooters: boolean;
    includeHeaders: boolean;
    includeWatermarks: boolean;
    stripRepeatingText: boolean;
}

/**
 * Content layer classification for document nodes.
 *
 * Replaces separate body/furniture arrays with per-node granularity.
 */
export enum WasmContentLayer {
    Body = 0,
    Header = 1,
    Footer = 2,
    Footnote = 3,
}

/**
 * JATS contributor with role.
 */
export class WasmContributorRole {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmContributorRole;
    constructor(name: string, role?: string | null);
    name: string;
    get role(): string | undefined;
    set role(value: string | null | undefined);
}

/**
 * Configuration for crawl, scrape, and map operations.
 */
export class WasmCrawlConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCrawlConfig;
    constructor(respectRobotsTxt?: boolean | null, softHttpErrors?: boolean | null, stayOnDomain?: boolean | null, allowSubdomains?: boolean | null, includePaths?: string[] | null, excludePaths?: string[] | null, customHeaders?: any | null, requestTimeout?: bigint | null, maxRedirects?: number | null, retryCount?: number | null, retryCodes?: Uint16Array | null, cookiesEnabled?: boolean | null, removeTags?: string[] | null, content?: WasmContentConfig | null, downloadAssets?: boolean | null, assetTypes?: any[] | null, browser?: WasmBrowserConfig | null, userAgents?: string[] | null, captureScreenshot?: boolean | null, followDocumentUrls?: boolean | null, downloadDocuments?: boolean | null, documentMimeTypes?: string[] | null, saveBrowserProfile?: boolean | null, ssrf?: WasmSsrfPolicy | null, maxDepth?: number | null, maxPages?: number | null, maxConcurrent?: number | null, userAgent?: string | null, rateLimitMs?: bigint | null, auth?: any | null, maxBodySize?: number | null, mapLimit?: number | null, mapSearch?: string | null, maxAssetSize?: number | null, proxy?: WasmProxyConfig | null, documentUrlDepth?: number | null, documentMaxSize?: number | null, warcOutput?: string | null, browserProfile?: string | null);
    allowSubdomains: boolean;
    assetTypes: string[];
    get auth(): any | undefined;
    set auth(value: any | null | undefined);
    browser: WasmBrowserConfig;
    get browserProfile(): string | undefined;
    set browserProfile(value: string | null | undefined);
    captureScreenshot: boolean;
    content: WasmContentConfig;
    cookiesEnabled: boolean;
    customHeaders: any;
    get documentMaxSize(): number | undefined;
    set documentMaxSize(value: number | null | undefined);
    documentMimeTypes: string[];
    get documentUrlDepth(): number | undefined;
    set documentUrlDepth(value: number | null | undefined);
    downloadAssets: boolean;
    downloadDocuments: boolean;
    excludePaths: string[];
    followDocumentUrls: boolean;
    includePaths: string[];
    get mapLimit(): number | undefined;
    set mapLimit(value: number | null | undefined);
    get mapSearch(): string | undefined;
    set mapSearch(value: string | null | undefined);
    get maxAssetSize(): number | undefined;
    set maxAssetSize(value: number | null | undefined);
    get maxBodySize(): number | undefined;
    set maxBodySize(value: number | null | undefined);
    get maxConcurrent(): number | undefined;
    set maxConcurrent(value: number | null | undefined);
    get maxDepth(): number | undefined;
    set maxDepth(value: number | null | undefined);
    get maxPages(): number | undefined;
    set maxPages(value: number | null | undefined);
    maxRedirects: number;
    get proxy(): WasmProxyConfig | undefined;
    set proxy(value: WasmProxyConfig | null | undefined);
    get rateLimitMs(): bigint | undefined;
    set rateLimitMs(value: bigint | null | undefined);
    removeTags: string[];
    get requestTimeout(): bigint | undefined;
    set requestTimeout(value: bigint | null | undefined);
    respectRobotsTxt: boolean;
    retryCodes: Uint16Array;
    retryCount: number;
    saveBrowserProfile: boolean;
    softHttpErrors: boolean;
    ssrf: WasmSsrfPolicy;
    stayOnDomain: boolean;
    get userAgent(): string | undefined;
    set userAgent(value: string | null | undefined);
    userAgents: string[];
    get warcOutput(): string | undefined;
    set warcOutput(value: string | null | undefined);
}

/**
 * CSV/TSV file metadata.
 */
export class WasmCsvMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmCsvMetadata;
    constructor(rowCount?: number | null, columnCount?: number | null, hasHeader?: boolean | null, delimiter?: string | null, columnTypes?: string[] | null);
    columnCount: number;
    get columnTypes(): string[] | undefined;
    set columnTypes(value: string[] | null | undefined);
    get delimiter(): string | undefined;
    set delimiter(value: string | null | undefined);
    hasHeader: boolean;
    rowCount: number;
}

/**
 * dBASE field information.
 */
export class WasmDbfFieldInfo {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDbfFieldInfo;
    constructor(name: string, fieldType: string);
    fieldType: string;
    name: string;
}

/**
 * dBASE (DBF) file metadata.
 */
export class WasmDbfMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDbfMetadata;
    constructor(recordCount?: number | null, fieldCount?: number | null, fields?: WasmDbfFieldInfo[] | null);
    fieldCount: number;
    fields: WasmDbfFieldInfo[];
    recordCount: number;
}

/**
 * A single line in a unified-diff hunk.
 *
 * Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
 * reference it unconditionally, without requiring the `diff` Cargo feature.
 * `crate.diff` re-exports this type verbatim.
 */
export class WasmDiffLine {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDiffLine;
    constructor();
    get 0(): string | undefined;
    set 0(value: string | null | undefined);
    kind: string;
}

/**
 * Comprehensive Djot document structure with semantic preservation.
 *
 * This type captures the full richness of Djot markup, including:
 * - Block-level structures (headings, lists, blockquotes, code blocks, etc.)
 * - Inline formatting (emphasis, strong, highlight, subscript, superscript, etc.)
 * - Attributes (classes, IDs, key-value pairs)
 * - Links, images, footnotes
 * - Math expressions (inline and display)
 * - Tables with full structure
 *
 * Available when the `djot` feature is enabled.
 */
export class WasmDjotContent {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDjotContent;
    constructor(plainText: string, blocks: WasmFormattedBlock[], metadata: WasmMetadata, tables: WasmTable[], images: WasmDjotImage[], links: WasmDjotLink[], footnotes: WasmFootnote[]);
    blocks: WasmFormattedBlock[];
    footnotes: WasmFootnote[];
    images: WasmDjotImage[];
    links: WasmDjotLink[];
    metadata: WasmMetadata;
    plainText: string;
    tables: WasmTable[];
}

/**
 * Image element in Djot.
 */
export class WasmDjotImage {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDjotImage;
    constructor(src: string, alt: string, title?: string | null);
    alt: string;
    src: string;
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * Link element in Djot.
 */
export class WasmDjotLink {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDjotLink;
    constructor(url: string, text: string, title?: string | null);
    text: string;
    get title(): string | undefined;
    set title(value: string | null | undefined);
    url: string;
}

/**
 * A single node in the document tree.
 *
 * Each node has deterministic `id`, typed `content`, optional `parent`/`children`
 * for tree structure, and metadata like page number, bounding box, and content layer.
 */
export class WasmDocumentNode {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDocumentNode;
    constructor(content: any, children: Uint32Array, contentLayer: WasmContentLayer, annotations: WasmTextAnnotation[], parent?: number | null, page?: number | null, pageEnd?: number | null, bbox?: WasmBoundingBox | null, attributes?: any | null);
    annotations: WasmTextAnnotation[];
    get attributes(): any | undefined;
    set attributes(value: any | null | undefined);
    get bbox(): WasmBoundingBox | undefined;
    set bbox(value: WasmBoundingBox | null | undefined);
    children: Uint32Array;
    content: any;
    get contentLayer(): string;
    set contentLayer(value: WasmContentLayer);
    get page(): number | undefined;
    set page(value: number | null | undefined);
    get pageEnd(): number | undefined;
    set pageEnd(value: number | null | undefined);
    get parent(): number | undefined;
    set parent(value: number | null | undefined);
}

/**
 * A resolved relationship between two nodes in the document tree.
 */
export class WasmDocumentRelationship {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDocumentRelationship;
    constructor(source: number, target: number, kind: WasmRelationshipKind);
    get kind(): string;
    set kind(value: WasmRelationshipKind);
    source: number;
    target: number;
}

/**
 * A single tracked change embedded in a document.
 *
 * Populated by per-format extractors that understand change-tracking metadata
 * (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every
 * extractor defaults to `ExtractedDocument.revisions = None` until a
 * format-specific implementation is added.
 */
export class WasmDocumentRevision {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDocumentRevision;
    constructor(revisionId: string, kind: WasmRevisionKind, delta: WasmRevisionDelta, author?: string | null, timestamp?: string | null, anchor?: any | null);
    get anchor(): any | undefined;
    set anchor(value: any | null | undefined);
    get author(): string | undefined;
    set author(value: string | null | undefined);
    delta: WasmRevisionDelta;
    get kind(): string;
    set kind(value: WasmRevisionKind);
    revisionId: string;
    get timestamp(): string | undefined;
    set timestamp(value: string | null | undefined);
}

/**
 * Top-level structured document representation.
 *
 * A flat array of nodes with index-based parent/child references forming a tree.
 * Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
 * to iterate over top-level content by layer.
 *
 * # Validation
 *
 * Call `validate()` after construction to verify all node indices are in bounds
 * and parent-child relationships are bidirectionally consistent.
 */
export class WasmDocumentStructure {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDocumentStructure;
    /**
     * Compute and populate the `node_types` field from the current `nodes`.
     *
     * Call this after all nodes have been added to the structure. Internal
     * construction paths (builder, derivation) call this automatically.
     *
     * # Examples
     */
    finalizeNodeTypes(): void;
    /**
     * Check if the document structure is empty.
     */
    isEmpty(): boolean;
    constructor(nodes?: WasmDocumentNode[] | null, relationships?: WasmDocumentRelationship[] | null, nodeTypes?: string[] | null, sourceFormat?: string | null);
    nodeTypes: string[];
    nodes: WasmDocumentNode[];
    relationships: WasmDocumentRelationship[];
    get sourceFormat(): string | undefined;
    set sourceFormat(value: string | null | undefined);
}

/**
 * Summary of an extracted document.
 */
export class WasmDocumentSummary {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmDocumentSummary;
    constructor(text: string, strategy: WasmSummaryStrategy, tokenCount?: number | null);
    get strategy(): string;
    set strategy(value: WasmSummaryStrategy);
    text: string;
    get tokenCount(): number | undefined;
    set tokenCount(value: number | null | undefined);
}

/**
 * Semantic element extracted from document.
 *
 * Represents a logical unit of content with semantic classification,
 * unique identifier, and metadata for tracking origin and position.
 */
export class WasmElement {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmElement;
    constructor(elementType: WasmElementType, text: string, metadata: WasmElementMetadata);
    get elementType(): string;
    set elementType(value: WasmElementType);
    metadata: WasmElementMetadata;
    text: string;
}

/**
 * Metadata for a semantic element.
 */
export class WasmElementMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmElementMetadata;
    constructor(additional: any, pageNumber?: number | null, filename?: string | null, coordinates?: WasmBoundingBox | null, elementIndex?: number | null);
    additional: any;
    get coordinates(): WasmBoundingBox | undefined;
    set coordinates(value: WasmBoundingBox | null | undefined);
    get elementIndex(): number | undefined;
    set elementIndex(value: number | null | undefined);
    get filename(): string | undefined;
    set filename(value: string | null | undefined);
    get pageNumber(): number | undefined;
    set pageNumber(value: number | null | undefined);
}

/**
 * Semantic element type classification.
 *
 * Categorizes text content into semantic units for downstream processing.
 * Supports the element types commonly found in Unstructured documents.
 */
export enum WasmElementType {
    Title = 0,
    NarrativeText = 1,
    Heading = 2,
    ListItem = 3,
    Table = 4,
    Image = 5,
    PageBreak = 6,
    CodeBlock = 7,
    BlockQuote = 8,
    Footer = 9,
    Header = 10,
}

/**
 * Email attachment representation.
 *
 * Contains metadata and optionally the content of an email attachment.
 */
export class WasmEmailAttachment {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmailAttachment;
    constructor(isImage: boolean, name?: string | null, filename?: string | null, mimeType?: string | null, size?: number | null, data?: Uint8Array | null);
    get data(): Uint8Array | undefined;
    set data(value: Uint8Array | null | undefined);
    get filename(): string | undefined;
    set filename(value: string | null | undefined);
    isImage: boolean;
    get mimeType(): string | undefined;
    set mimeType(value: string | null | undefined);
    get name(): string | undefined;
    set name(value: string | null | undefined);
    get size(): number | undefined;
    set size(value: number | null | undefined);
}

/**
 * Configuration for email extraction.
 */
export class WasmEmailConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmailConfig;
    constructor(msgFallbackCodepage?: number | null);
    get msgFallbackCodepage(): number | undefined;
    set msgFallbackCodepage(value: number | null | undefined);
}

/**
 * Email extraction result.
 *
 * Complete representation of an extracted email message (.eml or .msg)
 * including headers, body content, and attachments.
 */
export class WasmEmailExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmailExtractionResult;
    constructor(toEmails: string[], ccEmails: string[], bccEmails: string[], content: string, attachments: WasmEmailAttachment[], metadata: any, subject?: string | null, fromEmail?: string | null, date?: string | null, messageId?: string | null, plainText?: string | null, htmlContent?: string | null);
    attachments: WasmEmailAttachment[];
    bccEmails: string[];
    ccEmails: string[];
    content: string;
    get date(): string | undefined;
    set date(value: string | null | undefined);
    get fromEmail(): string | undefined;
    set fromEmail(value: string | null | undefined);
    get htmlContent(): string | undefined;
    set htmlContent(value: string | null | undefined);
    get messageId(): string | undefined;
    set messageId(value: string | null | undefined);
    metadata: any;
    get plainText(): string | undefined;
    set plainText(value: string | null | undefined);
    get subject(): string | undefined;
    set subject(value: string | null | undefined);
    toEmails: string[];
}

/**
 * Email metadata extracted from .eml and .msg files.
 *
 * Includes sender/recipient information, message ID, and attachment list.
 */
export class WasmEmailMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmailMetadata;
    constructor(toEmails?: string[] | null, ccEmails?: string[] | null, bccEmails?: string[] | null, attachments?: string[] | null, fromEmail?: string | null, fromName?: string | null, messageId?: string | null);
    attachments: string[];
    bccEmails: string[];
    ccEmails: string[];
    get fromEmail(): string | undefined;
    set fromEmail(value: string | null | undefined);
    get fromName(): string | undefined;
    set fromName(value: string | null | undefined);
    get messageId(): string | undefined;
    set messageId(value: string | null | undefined);
    toEmails: string[];
}

/**
 * Embedding configuration for text chunks.
 *
 * Configures embedding generation using ONNX models via the vendored embedding engine.
 * Requires the `embeddings` feature to be enabled.
 */
export class WasmEmbeddingConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmbeddingConfig;
    constructor(model?: any | null, normalize?: boolean | null, batchSize?: number | null, showDownloadProgress?: boolean | null, cacheDir?: string | null, acceleration?: WasmAccelerationConfig | null, maxEmbedDurationSecs?: bigint | null);
    get acceleration(): WasmAccelerationConfig | undefined;
    set acceleration(value: WasmAccelerationConfig | null | undefined);
    batchSize: number;
    get cacheDir(): string | undefined;
    set cacheDir(value: string | null | undefined);
    get maxEmbedDurationSecs(): bigint | undefined;
    set maxEmbedDurationSecs(value: bigint | null | undefined);
    model: any;
    normalize: boolean;
    showDownloadProgress: boolean;
}

/**
 * Embedding model types supported by Xberg.
 */
export class WasmEmbeddingModelType {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEmbeddingModelType;
    constructor();
    get dimensions(): number | undefined;
    set dimensions(value: number | null | undefined);
    get llm(): WasmLlmConfig | undefined;
    set llm(value: WasmLlmConfig | null | undefined);
    get modelId(): string | undefined;
    set modelId(value: string | null | undefined);
    get name(): string | undefined;
    set name(value: string | null | undefined);
    type: string;
}

/**
 * A single named entity detected in the extracted text.
 */
export class WasmEntity {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEntity;
    constructor(category: WasmEntityCategory, text: string, start: number, end: number, confidence?: number | null);
    get category(): string;
    set category(value: WasmEntityCategory);
    get confidence(): number | undefined;
    set confidence(value: number | null | undefined);
    end: number;
    start: number;
    text: string;
}

/**
 * Standard entity categories produced by built-in NER backends.
 *
 * The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
 * schemas) flow through without losing fidelity to the consumer.
 */
export enum WasmEntityCategory {
    Person = 0,
    Organization = 1,
    Location = 2,
    Date = 3,
    Time = 4,
    Money = 5,
    Percent = 6,
    Email = 7,
    Phone = 8,
    Url = 9,
    Custom = 10,
}

/**
 * EPUB metadata (Dublin Core extensions).
 */
export class WasmEpubMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmEpubMetadata;
    constructor(coverage?: string | null, dcFormat?: string | null, relation?: string | null, source?: string | null, dcType?: string | null, coverImage?: string | null);
    get coverImage(): string | undefined;
    set coverImage(value: string | null | undefined);
    get coverage(): string | undefined;
    set coverage(value: string | null | undefined);
    get dcFormat(): string | undefined;
    set dcFormat(value: string | null | undefined);
    get dcType(): string | undefined;
    set dcType(value: string | null | undefined);
    get relation(): string | undefined;
    set relation(value: string | null | undefined);
    get source(): string | undefined;
    set source(value: string | null | undefined);
}

/**
 * Error metadata (for batch operations).
 */
export class WasmErrorMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmErrorMetadata;
    constructor(errorType: string, message: string);
    errorType: string;
    message: string;
}

/**
 * Excel/spreadsheet format metadata.
 *
 * Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
 * discriminant. Sheet count and sheet names are stored inside this struct.
 */
export class WasmExcelMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExcelMetadata;
    constructor(sheetCount?: number | null, sheetNames?: string[] | null);
    get sheetCount(): number | undefined;
    set sheetCount(value: number | null | undefined);
    get sheetNames(): string[] | undefined;
    set sheetNames(value: string[] | null | undefined);
}

/**
 * Single Excel worksheet.
 *
 * Represents one sheet from an Excel workbook with its content
 * converted to Markdown format and dimensional statistics.
 */
export class WasmExcelSheet {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExcelSheet;
    constructor(name: string, markdown: string, rowCount: number, colCount: number, cellCount: number, tableCells?: any | null);
    cellCount: number;
    colCount: number;
    markdown: string;
    name: string;
    rowCount: number;
    get tableCells(): any | undefined;
    set tableCells(value: any | null | undefined);
}

/**
 * Excel workbook representation.
 *
 * Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
 * extracted content and metadata.
 */
export class WasmExcelWorkbook {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExcelWorkbook;
    constructor(sheets: WasmExcelSheet[], metadata: any, revisions?: WasmDocumentRevision[] | null);
    metadata: any;
    get revisions(): Array<any> | undefined;
    set revisions(value: WasmDocumentRevision[] | null | undefined);
    sheets: WasmExcelSheet[];
}

/**
 * ONNX Runtime execution provider type.
 *
 * Determines which hardware backend is used for model inference.
 * `Auto` (default) selects the best available provider per platform.
 */
export enum WasmExecutionProviderType {
    Auto = 0,
    Cpu = 1,
    CoreMl = 2,
    Cuda = 3,
    TensorRt = 4,
}

/**
 * Unified extraction input for all public extraction entry points.
 */
export class WasmExtractInput {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractInput;
    /**
     * Build a bytes input with a MIME type and optional filename hint.
     */
    static fromBytes(bytes: Uint8Array, mime_type: string, filename?: string | null): WasmExtractInput;
    /**
     * Build a URI input from a local path, `file://` URI, or HTTP(S) URL.
     */
    static fromUri(uri: string): WasmExtractInput;
    constructor(kind?: WasmExtractInputKind | null, bytes?: Uint8Array | null, uri?: string | null, mimeType?: string | null, filename?: string | null, config?: WasmFileExtractionConfig | null);
    get bytes(): Uint8Array | undefined;
    set bytes(value: Uint8Array | null | undefined);
    get config(): WasmFileExtractionConfig | undefined;
    set config(value: WasmFileExtractionConfig | null | undefined);
    get filename(): string | undefined;
    set filename(value: string | null | undefined);
    get kind(): string;
    set kind(value: WasmExtractInputKind);
    get mimeType(): string | undefined;
    set mimeType(value: string | null | undefined);
    get uri(): string | undefined;
    set uri(value: string | null | undefined);
}

/**
 * Source kind for `ExtractInput`.
 */
export enum WasmExtractInputKind {
    Bytes = 0,
    Uri = 1,
}

/**
 * Document extracted by the core extraction pipeline.
 *
 * `extract` and `extract_batch` return an `ExtractionResult` envelope whose
 * `results` field contains these per-document payloads.
 */
export class WasmExtractedDocument {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractedDocument;
    constructor(content?: string | null, mimeType?: string | null, metadata?: WasmMetadata | null, tables?: WasmTable[] | null, processingWarnings?: WasmProcessingWarning[] | null, formulas?: WasmFormula[] | null, formFields?: WasmPdfFormField[] | null, extractionMethod?: WasmExtractionMethod | null, detectedLanguages?: string[] | null, chunks?: WasmChunk[] | null, images?: WasmExtractedImage[] | null, pages?: WasmPageContent[] | null, elements?: WasmElement[] | null, djotContent?: WasmDjotContent | null, ocrElements?: WasmOcrElement[] | null, document?: WasmDocumentStructure | null, qualityScore?: number | null, annotations?: WasmPdfAnnotation[] | null, children?: WasmArchiveEntry[] | null, uris?: WasmExtractedUri[] | null, revisions?: WasmDocumentRevision[] | null, structuredOutput?: any | null, llmUsage?: WasmLlmUsage[] | null, entities?: WasmEntity[] | null, summary?: WasmDocumentSummary | null, translation?: WasmTranslation | null, pageClassifications?: WasmPageClassification[] | null, redactionReport?: WasmRedactionReport | null, formattedContent?: string | null);
    get annotations(): Array<any> | undefined;
    set annotations(value: WasmPdfAnnotation[] | null | undefined);
    get children(): Array<any> | undefined;
    set children(value: WasmArchiveEntry[] | null | undefined);
    get chunks(): Array<any> | undefined;
    set chunks(value: WasmChunk[] | null | undefined);
    content: string;
    get detectedLanguages(): string[] | undefined;
    set detectedLanguages(value: string[] | null | undefined);
    get djotContent(): WasmDjotContent | undefined;
    set djotContent(value: WasmDjotContent | null | undefined);
    get document(): WasmDocumentStructure | undefined;
    set document(value: WasmDocumentStructure | null | undefined);
    get elements(): Array<any> | undefined;
    set elements(value: WasmElement[] | null | undefined);
    get entities(): Array<any> | undefined;
    set entities(value: WasmEntity[] | null | undefined);
    get extractionMethod(): string | undefined;
    set extractionMethod(value: WasmExtractionMethod | null | undefined);
    formFields: WasmPdfFormField[];
    get formattedContent(): string | undefined;
    set formattedContent(value: string | null | undefined);
    formulas: WasmFormula[];
    get images(): Array<any> | undefined;
    set images(value: WasmExtractedImage[] | null | undefined);
    get llmUsage(): Array<any> | undefined;
    set llmUsage(value: WasmLlmUsage[] | null | undefined);
    metadata: WasmMetadata;
    mimeType: string;
    get ocrElements(): Array<any> | undefined;
    set ocrElements(value: WasmOcrElement[] | null | undefined);
    get pageClassifications(): Array<any> | undefined;
    set pageClassifications(value: WasmPageClassification[] | null | undefined);
    get pages(): Array<any> | undefined;
    set pages(value: WasmPageContent[] | null | undefined);
    processingWarnings: WasmProcessingWarning[];
    get qualityScore(): number | undefined;
    set qualityScore(value: number | null | undefined);
    get redactionReport(): WasmRedactionReport | undefined;
    set redactionReport(value: WasmRedactionReport | null | undefined);
    get revisions(): Array<any> | undefined;
    set revisions(value: WasmDocumentRevision[] | null | undefined);
    get structuredOutput(): any | undefined;
    set structuredOutput(value: any | null | undefined);
    get summary(): WasmDocumentSummary | undefined;
    set summary(value: WasmDocumentSummary | null | undefined);
    tables: WasmTable[];
    get translation(): WasmTranslation | undefined;
    set translation(value: WasmTranslation | null | undefined);
    get uris(): Array<any> | undefined;
    set uris(value: WasmExtractedUri[] | null | undefined);
}

/**
 * Extracted image from a document.
 *
 * Contains raw image data, metadata, and optional nested OCR results.
 * Raw bytes allow cross-language compatibility - users can convert to
 * PIL.Image (Python), Sharp (Node.js), or other formats as needed.
 */
export class WasmExtractedImage {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractedImage;
    constructor(data?: Uint8Array | null, format?: string | null, imageIndex?: number | null, isMask?: boolean | null, pageNumber?: number | null, width?: number | null, height?: number | null, colorspace?: string | null, bitsPerComponent?: number | null, description?: string | null, ocrResult?: WasmExtractedDocument | null, boundingBox?: WasmBoundingBox | null, sourcePath?: string | null, imageKind?: WasmImageKind | null, kindConfidence?: number | null, clusterId?: number | null, caption?: string | null, qrCodes?: WasmQrCode[] | null, dataBase64?: string | null);
    get bitsPerComponent(): number | undefined;
    set bitsPerComponent(value: number | null | undefined);
    get boundingBox(): WasmBoundingBox | undefined;
    set boundingBox(value: WasmBoundingBox | null | undefined);
    get caption(): string | undefined;
    set caption(value: string | null | undefined);
    get clusterId(): number | undefined;
    set clusterId(value: number | null | undefined);
    get colorspace(): string | undefined;
    set colorspace(value: string | null | undefined);
    data: Uint8Array;
    get dataBase64(): string | undefined;
    set dataBase64(value: string | null | undefined);
    get description(): string | undefined;
    set description(value: string | null | undefined);
    format: string;
    get height(): number | undefined;
    set height(value: number | null | undefined);
    imageIndex: number;
    get imageKind(): string | undefined;
    set imageKind(value: WasmImageKind | null | undefined);
    isMask: boolean;
    get kindConfidence(): number | undefined;
    set kindConfidence(value: number | null | undefined);
    get ocrResult(): WasmExtractedDocument | undefined;
    set ocrResult(value: WasmExtractedDocument | null | undefined);
    get pageNumber(): number | undefined;
    set pageNumber(value: number | null | undefined);
    get qrCodes(): Array<any> | undefined;
    set qrCodes(value: WasmQrCode[] | null | undefined);
    get sourcePath(): string | undefined;
    set sourcePath(value: string | null | undefined);
    get width(): number | undefined;
    set width(value: number | null | undefined);
}

/**
 * A URI extracted from a document.
 *
 * Represents any link, reference, or resource pointer found during extraction.
 * The `kind` field classifies the URI semantically, while `label` carries
 * optional human-readable display text.
 */
export class WasmExtractedUri {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractedUri;
    constructor(url: string, kind: WasmUriKind, label?: string | null, page?: number | null);
    get kind(): string;
    set kind(value: WasmUriKind);
    get label(): string | undefined;
    set label(value: string | null | undefined);
    get page(): number | undefined;
    set page(value: number | null | undefined);
    url: string;
}

/**
 * Main extraction configuration.
 *
 * This struct contains all configuration options for the extraction process.
 * It can be loaded from TOML, YAML, or JSON files, or created programmatically.
 *
 * # Example
 */
export class WasmExtractionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractionConfig;
    /**
     * Check if image processing is needed by examining OCR and image extraction settings.
     *
     * Returns `true` if either OCR is enabled or image extraction is configured,
     * indicating that image decompression and processing should occur.
     * Returns `false` if both are disabled, allowing optimization to skip unnecessary
     * image decompression for text-only extraction workflows.
     *
     * # Optimization Impact
     * For text-only extractions (no OCR, no image extraction), skipping image
     * decompression can improve CPU utilization by 5-10% by avoiding wasteful
     * image I/O and processing when results won't be used.
     * Returns `true` when image binary data should be extracted.
     *
     * True when `config.images.extract_images` is set, captioning is configured, or QR-code
     * detection is enabled. Captioning and QR-code detection both require image bytes
     * regardless of whether the caller also requested image extraction.
     */
    needsImageData(): boolean;
    /**
     * Returns `true` when any image processing is needed during extraction.
     *
     * # Optimization Impact
     *
     * For text-only extractions (no OCR, no image extraction, no captioning), skipping
     * image decompression can improve CPU utilization by 5-10% by avoiding wasteful
     * image I/O and processing when results won't be used.
     */
    needsImageProcessing(): boolean;
    constructor(useCache?: boolean | null, enableQualityProcessing?: boolean | null, forceOcr?: boolean | null, disableOcr?: boolean | null, resultFormat?: WasmResultFormat | null, outputFormat?: WasmOutputFormat | null, useLayoutForMarkdown?: boolean | null, includeDocumentStructure?: boolean | null, url?: WasmUrlExtractionConfig | null, maxArchiveDepth?: number | null, ocr?: WasmOcrConfig | null, forceOcrPages?: Uint32Array | null, chunking?: WasmChunkingConfig | null, contentFilter?: WasmContentFilterConfig | null, images?: WasmImageExtractionConfig | null, tokenReduction?: WasmTokenReductionOptions | null, languageDetection?: WasmLanguageDetectionConfig | null, pages?: WasmPageConfig | null, postprocessor?: WasmPostProcessorConfig | null, extractionTimeoutSecs?: bigint | null, maxConcurrentExtractions?: number | null, securityLimits?: WasmSecurityLimits | null, maxEmbeddedFileBytes?: bigint | null, acceleration?: WasmAccelerationConfig | null, cacheNamespace?: string | null, cacheTtlSecs?: bigint | null, email?: WasmEmailConfig | null, structuredExtraction?: WasmStructuredExtractionConfig | null, ner?: WasmNerConfig | null, redaction?: WasmRedactionConfig | null, summarization?: WasmSummarizationConfig | null, translation?: WasmTranslationConfig | null, pageClassification?: WasmPageClassificationConfig | null, captioning?: WasmCaptioningConfig | null, qrCodes?: boolean | null);
    get acceleration(): WasmAccelerationConfig | undefined;
    set acceleration(value: WasmAccelerationConfig | null | undefined);
    get cacheNamespace(): string | undefined;
    set cacheNamespace(value: string | null | undefined);
    get cacheTtlSecs(): bigint | undefined;
    set cacheTtlSecs(value: bigint | null | undefined);
    get captioning(): WasmCaptioningConfig | undefined;
    set captioning(value: WasmCaptioningConfig | null | undefined);
    get chunking(): WasmChunkingConfig | undefined;
    set chunking(value: WasmChunkingConfig | null | undefined);
    get contentFilter(): WasmContentFilterConfig | undefined;
    set contentFilter(value: WasmContentFilterConfig | null | undefined);
    disableOcr: boolean;
    get email(): WasmEmailConfig | undefined;
    set email(value: WasmEmailConfig | null | undefined);
    enableQualityProcessing: boolean;
    get extractionTimeoutSecs(): bigint | undefined;
    set extractionTimeoutSecs(value: bigint | null | undefined);
    forceOcr: boolean;
    get forceOcrPages(): Uint32Array | undefined;
    set forceOcrPages(value: Uint32Array | null | undefined);
    get images(): WasmImageExtractionConfig | undefined;
    set images(value: WasmImageExtractionConfig | null | undefined);
    includeDocumentStructure: boolean;
    get languageDetection(): WasmLanguageDetectionConfig | undefined;
    set languageDetection(value: WasmLanguageDetectionConfig | null | undefined);
    maxArchiveDepth: number;
    get maxConcurrentExtractions(): number | undefined;
    set maxConcurrentExtractions(value: number | null | undefined);
    get maxEmbeddedFileBytes(): bigint | undefined;
    set maxEmbeddedFileBytes(value: bigint | null | undefined);
    get ner(): WasmNerConfig | undefined;
    set ner(value: WasmNerConfig | null | undefined);
    get ocr(): WasmOcrConfig | undefined;
    set ocr(value: WasmOcrConfig | null | undefined);
    get outputFormat(): string;
    set outputFormat(value: WasmOutputFormat);
    get pageClassification(): WasmPageClassificationConfig | undefined;
    set pageClassification(value: WasmPageClassificationConfig | null | undefined);
    get pages(): WasmPageConfig | undefined;
    set pages(value: WasmPageConfig | null | undefined);
    get postprocessor(): WasmPostProcessorConfig | undefined;
    set postprocessor(value: WasmPostProcessorConfig | null | undefined);
    get qrCodes(): boolean | undefined;
    set qrCodes(value: boolean | null | undefined);
    get redaction(): WasmRedactionConfig | undefined;
    set redaction(value: WasmRedactionConfig | null | undefined);
    get resultFormat(): string;
    set resultFormat(value: WasmResultFormat);
    get securityLimits(): WasmSecurityLimits | undefined;
    set securityLimits(value: WasmSecurityLimits | null | undefined);
    get structuredExtraction(): WasmStructuredExtractionConfig | undefined;
    set structuredExtraction(value: WasmStructuredExtractionConfig | null | undefined);
    get summarization(): WasmSummarizationConfig | undefined;
    set summarization(value: WasmSummarizationConfig | null | undefined);
    get tokenReduction(): WasmTokenReductionOptions | undefined;
    set tokenReduction(value: WasmTokenReductionOptions | null | undefined);
    get translation(): WasmTranslationConfig | undefined;
    set translation(value: WasmTranslationConfig | null | undefined);
    url: WasmUrlExtractionConfig;
    useCache: boolean;
    useLayoutForMarkdown: boolean;
}

/**
 * Non-fatal per-input extraction error captured by `ExtractionResult`.
 */
export class WasmExtractionErrorItem {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractionErrorItem;
    constructor(index: number, code: number, errorType: string, source: string, message: string);
    code: number;
    errorType: string;
    index: number;
    message: string;
    source: string;
}

/**
 * How the extracted text was produced.
 */
export enum WasmExtractionMethod {
    Native = 0,
    Ocr = 1,
    Mixed = 2,
}

/**
 * Unified extraction result envelope.
 */
export class WasmExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractionResult;
    constructor(results?: WasmExtractedDocument[] | null, errors?: WasmExtractionErrorItem[] | null, summary?: WasmExtractionSummary | null, crawlFinalUrls?: string[] | null, crawlRedirectCount?: number | null, crawlUniqueNormalizedUrls?: string[] | null);
    /**
     * Build an output containing one successful result.
     */
    static single(result: WasmExtractedDocument): WasmExtractionResult;
    crawlFinalUrls: string[];
    crawlRedirectCount: number;
    crawlUniqueNormalizedUrls: string[];
    errors: WasmExtractionErrorItem[];
    results: WasmExtractedDocument[];
    summary: WasmExtractionSummary;
}

/**
 * Summary for a unified extraction call.
 */
export class WasmExtractionSummary {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmExtractionSummary;
    constructor(inputs?: number | null, results?: number | null, errors?: number | null, remoteUrls?: number | null, pagesCrawled?: number | null, documentsDownloaded?: number | null);
    documentsDownloaded: number;
    errors: number;
    inputs: number;
    pagesCrawled: number;
    remoteUrls: number;
    results: number;
}

/**
 * FictionBook (FB2) metadata.
 */
export class WasmFictionBookMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFictionBookMetadata;
    constructor(genres?: string[] | null, sequences?: string[] | null, annotation?: string | null);
    get annotation(): string | undefined;
    set annotation(value: string | null | undefined);
    genres: string[];
    sequences: string[];
}

/**
 * Per-file extraction configuration overrides for batch processing.
 *
 * All fields are `Option<T>` — `None` means "use the batch-level default."
 * This type is used by `config` and `extract_batch`
 * to allow heterogeneous extraction settings within a single batch.
 *
 * # Excluded Fields
 *
 * The following `ExtractionConfig` fields are batch-level only and
 * cannot be overridden per file:
 * - `max_concurrent_extractions` — controls batch parallelism
 * - `use_cache` — global caching policy
 * - `acceleration` — shared ONNX execution provider
 * - `security_limits` — global archive security policy
 *
 * # Example
 */
export class WasmFileExtractionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFileExtractionConfig;
    constructor(enableQualityProcessing?: boolean | null, ocr?: WasmOcrConfig | null, forceOcr?: boolean | null, forceOcrPages?: Uint32Array | null, disableOcr?: boolean | null, chunking?: WasmChunkingConfig | null, contentFilter?: WasmContentFilterConfig | null, images?: WasmImageExtractionConfig | null, tokenReduction?: WasmTokenReductionOptions | null, languageDetection?: WasmLanguageDetectionConfig | null, pages?: WasmPageConfig | null, postprocessor?: WasmPostProcessorConfig | null, resultFormat?: WasmResultFormat | null, outputFormat?: WasmOutputFormat | null, includeDocumentStructure?: boolean | null, timeoutSecs?: bigint | null, structuredExtraction?: WasmStructuredExtractionConfig | null, url?: WasmUrlExtractionConfig | null, ner?: WasmNerConfig | null, redaction?: WasmRedactionConfig | null, summarization?: WasmSummarizationConfig | null, translation?: WasmTranslationConfig | null, pageClassification?: WasmPageClassificationConfig | null, captioning?: WasmCaptioningConfig | null, qrCodes?: boolean | null);
    get captioning(): WasmCaptioningConfig | undefined;
    set captioning(value: WasmCaptioningConfig | null | undefined);
    get chunking(): WasmChunkingConfig | undefined;
    set chunking(value: WasmChunkingConfig | null | undefined);
    get contentFilter(): WasmContentFilterConfig | undefined;
    set contentFilter(value: WasmContentFilterConfig | null | undefined);
    get disableOcr(): boolean | undefined;
    set disableOcr(value: boolean | null | undefined);
    get enableQualityProcessing(): boolean | undefined;
    set enableQualityProcessing(value: boolean | null | undefined);
    get forceOcr(): boolean | undefined;
    set forceOcr(value: boolean | null | undefined);
    get forceOcrPages(): Uint32Array | undefined;
    set forceOcrPages(value: Uint32Array | null | undefined);
    get images(): WasmImageExtractionConfig | undefined;
    set images(value: WasmImageExtractionConfig | null | undefined);
    get includeDocumentStructure(): boolean | undefined;
    set includeDocumentStructure(value: boolean | null | undefined);
    get languageDetection(): WasmLanguageDetectionConfig | undefined;
    set languageDetection(value: WasmLanguageDetectionConfig | null | undefined);
    get ner(): WasmNerConfig | undefined;
    set ner(value: WasmNerConfig | null | undefined);
    get ocr(): WasmOcrConfig | undefined;
    set ocr(value: WasmOcrConfig | null | undefined);
    get outputFormat(): string | undefined;
    set outputFormat(value: WasmOutputFormat | null | undefined);
    get pageClassification(): WasmPageClassificationConfig | undefined;
    set pageClassification(value: WasmPageClassificationConfig | null | undefined);
    get pages(): WasmPageConfig | undefined;
    set pages(value: WasmPageConfig | null | undefined);
    get postprocessor(): WasmPostProcessorConfig | undefined;
    set postprocessor(value: WasmPostProcessorConfig | null | undefined);
    get qrCodes(): boolean | undefined;
    set qrCodes(value: boolean | null | undefined);
    get redaction(): WasmRedactionConfig | undefined;
    set redaction(value: WasmRedactionConfig | null | undefined);
    get resultFormat(): string | undefined;
    set resultFormat(value: WasmResultFormat | null | undefined);
    get structuredExtraction(): WasmStructuredExtractionConfig | undefined;
    set structuredExtraction(value: WasmStructuredExtractionConfig | null | undefined);
    get summarization(): WasmSummarizationConfig | undefined;
    set summarization(value: WasmSummarizationConfig | null | undefined);
    get timeoutSecs(): bigint | undefined;
    set timeoutSecs(value: bigint | null | undefined);
    get tokenReduction(): WasmTokenReductionOptions | undefined;
    set tokenReduction(value: WasmTokenReductionOptions | null | undefined);
    get translation(): WasmTranslationConfig | undefined;
    set translation(value: WasmTranslationConfig | null | undefined);
    get url(): WasmUrlExtractionConfig | undefined;
    set url(value: WasmUrlExtractionConfig | null | undefined);
}

/**
 * Footnote in Djot.
 */
export class WasmFootnote {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFootnote;
    constructor(label: string, content: WasmFormattedBlock[]);
    content: WasmFormattedBlock[];
    label: string;
}

/**
 * Kind of a PDF form field.
 *
 * Mirrors `pdf_oxide`'s widget field taxonomy without leaking the upstream
 * type across the binding surface.
 */
export enum WasmFormFieldType {
    Text = 0,
    Checkbox = 1,
    Radio = 2,
    Choice = 3,
    Signature = 4,
    Button = 5,
    Unknown = 6,
}

/**
 * Format-specific metadata (discriminated union).
 *
 * Only one format type can exist per extraction result. This provides
 * type-safe, clean metadata without nested optionals.
 */
export class WasmFormatMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFormatMetadata;
    constructor();
    get 0(): any | undefined;
    set 0(value: any | null | undefined);
    formatType: string;
}

/**
 * Block-level element in a Djot document.
 *
 * Represents structural elements like headings, paragraphs, lists, code blocks, etc.
 */
export class WasmFormattedBlock {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFormattedBlock;
    constructor(blockType: WasmBlockType, inlineContent: WasmInlineElement[], children: WasmFormattedBlock[], level?: number | null, language?: string | null, code?: string | null);
    get blockType(): string;
    set blockType(value: WasmBlockType);
    children: WasmFormattedBlock[];
    get code(): string | undefined;
    set code(value: string | null | undefined);
    inlineContent: WasmInlineElement[];
    get language(): string | undefined;
    set language(value: string | null | undefined);
    get level(): number | undefined;
    set level(value: number | null | undefined);
}

/**
 * A mathematical formula detected and recognized in a document.
 *
 * Populated by the layout-guided formula pipeline: regions classified as
 * `LayoutClass.Formula` are routed to the formula OCR task, which returns the
 * LaTeX source for the region. The field is always present on
 * `ExtractedDocument` but only populated
 * when the `layout-detection` feature is active and the document contains
 * formula regions.
 */
export class WasmFormula {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmFormula;
    constructor(latex: string, bbox: WasmBoundingBox, page: number);
    bbox: WasmBoundingBox;
    latex: string;
    page: number;
}

/**
 * GLiNER ONNX architecture family. Determines which tensor I/O contract and
 * preprocessing pipeline xberg uses — only relevant when `hf_repo` is set,
 * since the pinned `xberg-io/gliner-models` catalog is always `GlinerArchitecture.Gliner1`.
 */
export enum WasmGlinerArchitecture {
    Gliner1 = 0,
    Gliner2 = 1,
}

/**
 * Individual grid cell with position and span metadata.
 */
export class WasmGridCell {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmGridCell;
    constructor(content: string, row: number, col: number, rowSpan: number, colSpan: number, isHeader: boolean, bbox?: WasmBoundingBox | null);
    get bbox(): WasmBoundingBox | undefined;
    set bbox(value: WasmBoundingBox | null | undefined);
    col: number;
    colSpan: number;
    content: string;
    isHeader: boolean;
    row: number;
    rowSpan: number;
}

/**
 * Header/heading element metadata.
 */
export class WasmHeaderMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmHeaderMetadata;
    constructor(level: number, text: string, depth: number, htmlOffset: number, id?: string | null);
    depth: number;
    htmlOffset: number;
    get id(): string | undefined;
    set id(value: string | null | undefined);
    level: number;
    text: string;
}

/**
 * Heading context for a chunk within a Markdown document.
 *
 * Contains the heading hierarchy from document root to this chunk's section.
 */
export class WasmHeadingContext {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmHeadingContext;
    constructor(headings: WasmHeadingLevel[]);
    headings: WasmHeadingLevel[];
}

/**
 * A single heading in the hierarchy.
 */
export class WasmHeadingLevel {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmHeadingLevel;
    constructor(level: number, text: string);
    level: number;
    text: string;
}

/**
 * A text block with hierarchy level assignment.
 *
 * Represents a block of text with semantic heading information extracted from
 * font size clustering and hierarchical analysis.
 */
export class WasmHierarchicalBlock {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmHierarchicalBlock;
    constructor(text: string, fontSize: number, level: string);
    fontSize: number;
    level: string;
    text: string;
}

/**
 * HTML metadata extracted from HTML documents.
 *
 * Includes document-level metadata, Open Graph data, Twitter Card metadata,
 * and extracted structural elements (headers, links, images, structured data).
 */
export class WasmHtmlMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmHtmlMetadata;
    constructor(keywords?: string[] | null, openGraph?: any | null, twitterCard?: any | null, metaTags?: any | null, headers?: WasmHeaderMetadata[] | null, links?: WasmLinkMetadata[] | null, images?: WasmImageMetadataType[] | null, structuredData?: WasmStructuredData[] | null, title?: string | null, description?: string | null, author?: string | null, canonicalUrl?: string | null, baseHref?: string | null, language?: string | null, textDirection?: WasmTextDirection | null);
    get author(): string | undefined;
    set author(value: string | null | undefined);
    get baseHref(): string | undefined;
    set baseHref(value: string | null | undefined);
    get canonicalUrl(): string | undefined;
    set canonicalUrl(value: string | null | undefined);
    get description(): string | undefined;
    set description(value: string | null | undefined);
    headers: WasmHeaderMetadata[];
    images: WasmImageMetadataType[];
    keywords: string[];
    get language(): string | undefined;
    set language(value: string | null | undefined);
    links: WasmLinkMetadata[];
    metaTags: any;
    openGraph: any;
    structuredData: WasmStructuredData[];
    get textDirection(): string | undefined;
    set textDirection(value: WasmTextDirection | null | undefined);
    get title(): string | undefined;
    set title(value: string | null | undefined);
    twitterCard: any;
}

/**
 * Image extraction configuration.
 */
export class WasmImageExtractionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImageExtractionConfig;
    constructor(extractImages?: boolean | null, targetDpi?: number | null, maxImageDimension?: number | null, injectPlaceholders?: boolean | null, autoAdjustDpi?: boolean | null, minDpi?: number | null, maxDpi?: number | null, classify?: boolean | null, includePageRasters?: boolean | null, runOcrOnImages?: boolean | null, ocrTextOnly?: boolean | null, appendOcrText?: boolean | null, outputFormat?: any | null, includeDataBase64?: boolean | null, maxImagesPerPage?: number | null);
    appendOcrText: boolean;
    autoAdjustDpi: boolean;
    classify: boolean;
    extractImages: boolean;
    includeDataBase64: boolean;
    includePageRasters: boolean;
    injectPlaceholders: boolean;
    maxDpi: number;
    maxImageDimension: number;
    get maxImagesPerPage(): number | undefined;
    set maxImagesPerPage(value: number | null | undefined);
    minDpi: number;
    ocrTextOnly: boolean;
    outputFormat: any;
    runOcrOnImages: boolean;
    targetDpi: number;
}

/**
 * Heuristic classification of what an image likely depicts.
 */
export enum WasmImageKind {
    Photograph = 0,
    Diagram = 1,
    Chart = 2,
    Drawing = 3,
    TextBlock = 4,
    Decoration = 5,
    Logo = 6,
    Icon = 7,
    TileFragment = 8,
    Mask = 9,
    PageRaster = 10,
    Unknown = 11,
}

/**
 * Image metadata extracted from image files.
 *
 * Includes dimensions, format, and EXIF data.
 */
export class WasmImageMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImageMetadata;
    constructor(width?: number | null, height?: number | null, format?: string | null, exif?: any | null);
    exif: any;
    format: string;
    height: number;
    width: number;
}

/**
 * Image element metadata.
 */
export class WasmImageMetadataType {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImageMetadataType;
    constructor(src: string, imageType: WasmImageType, alt?: string | null, title?: string | null);
    get alt(): string | undefined;
    set alt(value: string | null | undefined);
    get imageType(): string;
    set imageType(value: WasmImageType);
    src: string;
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * Target format for re-encoding extracted images.
 *
 * Controls whether and how extracted images are normalised to a uniform
 * container format before being returned in `ExtractedDocument.images`.
 * The default (`Native`) preserves the format produced by each extractor
 * without any additional encode pass.
 *
 * Callers that need uniform output — e.g. cloud pipelines that always store
 * WebP thumbnails — set this once on `ImageExtractionConfig.output_format`
 * rather than re-encoding downstream.
 *
 * # Serde shape
 *
 * Uses a tagged enum: `{"type": "native"}`, `{"type": "png"}`,
 * `{"type": "jpeg", "quality": 90}`, etc.
 */
export class WasmImageOutputFormat {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImageOutputFormat;
    constructor();
    get quality(): number | undefined;
    set quality(value: number | null | undefined);
    type: string;
}

/**
 * Image preprocessing configuration for OCR.
 *
 * These settings control how images are preprocessed before OCR to improve
 * text recognition quality. Different preprocessing strategies work better
 * for different document types.
 */
export class WasmImagePreprocessingConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImagePreprocessingConfig;
    constructor(targetDpi?: number | null, autoRotate?: boolean | null, deskew?: boolean | null, denoise?: boolean | null, contrastEnhance?: boolean | null, binarizationMethod?: string | null, invertColors?: boolean | null);
    autoRotate: boolean;
    binarizationMethod: string;
    contrastEnhance: boolean;
    denoise: boolean;
    deskew: boolean;
    invertColors: boolean;
    targetDpi: number;
}

/**
 * Image preprocessing metadata.
 *
 * Tracks the transformations applied to an image during OCR preprocessing,
 * including DPI normalization, resizing, and resampling.
 */
export class WasmImagePreprocessingMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmImagePreprocessingMetadata;
    constructor(targetDpi: number, scaleFactor: number, autoAdjusted: boolean, finalDpi: number, resampleMethod: string, dimensionClamped: boolean, skippedResize: boolean, calculatedDpi?: number | null, resizeError?: string | null);
    autoAdjusted: boolean;
    get calculatedDpi(): number | undefined;
    set calculatedDpi(value: number | null | undefined);
    dimensionClamped: boolean;
    finalDpi: number;
    resampleMethod: string;
    get resizeError(): string | undefined;
    set resizeError(value: string | null | undefined);
    scaleFactor: number;
    skippedResize: boolean;
    targetDpi: number;
}

/**
 * Image type classification.
 */
export enum WasmImageType {
    DataUri = 0,
    InlineSvg = 1,
    External = 2,
    Relative = 3,
}

/**
 * Inline element within a block.
 *
 * Represents text with formatting, links, images, etc.
 */
export class WasmInlineElement {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmInlineElement;
    constructor(elementType: WasmInlineType, content: string, metadata?: any | null);
    content: string;
    get elementType(): string;
    set elementType(value: WasmInlineType);
    get metadata(): any | undefined;
    set metadata(value: any | null | undefined);
}

/**
 * Types of inline elements in Djot.
 */
export enum WasmInlineType {
    Text = 0,
    Strong = 1,
    Emphasis = 2,
    Highlight = 3,
    Subscript = 4,
    Superscript = 5,
    Insert = 6,
    Delete = 7,
    Code = 8,
    Link = 9,
    Image = 10,
    Span = 11,
    Math = 12,
    RawInline = 13,
    FootnoteRef = 14,
    Symbol = 15,
}

/**
 * JATS (Journal Article Tag Suite) metadata.
 */
export class WasmJatsMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmJatsMetadata;
    constructor(historyDates?: any | null, contributorRoles?: WasmContributorRole[] | null, copyright?: string | null, license?: string | null);
    contributorRoles: WasmContributorRole[];
    get copyright(): string | undefined;
    set copyright(value: string | null | undefined);
    historyDates: any;
    get license(): string | undefined;
    set license(value: string | null | undefined);
}

/**
 * Language detection configuration.
 */
export class WasmLanguageDetectionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmLanguageDetectionConfig;
    constructor(enabled?: boolean | null, minConfidence?: number | null, detectMultiple?: boolean | null);
    detectMultiple: boolean;
    enabled: boolean;
    minConfidence: number;
}

/**
 * A detected layout region on a page.
 *
 * When layout detection is enabled, each page may have layout regions
 * identifying different content types (text, pictures, tables, etc.)
 * with confidence scores and spatial positions.
 */
export class WasmLayoutRegion {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmLayoutRegion;
    constructor(className?: string | null, confidence?: number | null, boundingBox?: WasmBoundingBox | null, areaFraction?: number | null);
    areaFraction: number;
    boundingBox: WasmBoundingBox;
    className: string;
    confidence: number;
}

/**
 * Link element metadata.
 */
export class WasmLinkMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmLinkMetadata;
    constructor(href: string, text: string, linkType: WasmLinkType, rel: string[], title?: string | null);
    href: string;
    get linkType(): string;
    set linkType(value: WasmLinkType);
    rel: string[];
    text: string;
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * Link type classification.
 */
export enum WasmLinkType {
    Anchor = 0,
    Internal = 1,
    External = 2,
    Email = 3,
    Phone = 4,
    Other = 5,
}

/**
 * Type of list detection.
 */
export enum WasmListType {
    Bullet = 0,
    Numbered = 1,
    Lettered = 2,
    Indented = 3,
}

/**
 * Configuration for an LLM provider/model via liter-llm.
 *
 * Each feature (VLM OCR, VLM embeddings, structured extraction) carries
 * its own `LlmConfig`, allowing different providers per feature.
 *
 * # Example
 *
 * ```toml
 * [structured_extraction.llm]
 * model = "openai/gpt-4o"
 * api_key = "sk-..."  # or use XBERG_LLM_API_KEY env var
 * ```
 */
export class WasmLlmConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmLlmConfig;
    constructor(model?: string | null, apiKey?: string | null, baseUrl?: string | null, timeoutSecs?: bigint | null, maxRetries?: number | null, temperature?: number | null, maxTokens?: bigint | null);
    get apiKey(): string | undefined;
    set apiKey(value: string | null | undefined);
    get baseUrl(): string | undefined;
    set baseUrl(value: string | null | undefined);
    get maxRetries(): number | undefined;
    set maxRetries(value: number | null | undefined);
    get maxTokens(): bigint | undefined;
    set maxTokens(value: bigint | null | undefined);
    model: string;
    get temperature(): number | undefined;
    set temperature(value: number | null | undefined);
    get timeoutSecs(): bigint | undefined;
    set timeoutSecs(value: bigint | null | undefined);
}

/**
 * Token usage and cost data for a single LLM call made during extraction.
 *
 * Populated when VLM OCR, structured extraction, or LLM-based embeddings
 * are used. Multiple entries may be present when multiple LLM calls occur
 * within one extraction (e.g. VLM OCR + structured extraction).
 */
export class WasmLlmUsage {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmLlmUsage;
    constructor(model?: string | null, source?: string | null, inputTokens?: bigint | null, outputTokens?: bigint | null, totalTokens?: bigint | null, estimatedCost?: number | null, finishReason?: string | null);
    get estimatedCost(): number | undefined;
    set estimatedCost(value: number | null | undefined);
    get finishReason(): string | undefined;
    set finishReason(value: string | null | undefined);
    get inputTokens(): bigint | undefined;
    set inputTokens(value: bigint | null | undefined);
    model: string;
    get outputTokens(): bigint | undefined;
    set outputTokens(value: bigint | null | undefined);
    source: string;
    get totalTokens(): bigint | undefined;
    set totalTokens(value: bigint | null | undefined);
}

/**
 * How partial results from multiple model calls (e.g. per page batch) are combined.
 *
 * Canonical home for the merge strategy referenced by presets and by the
 * structured pipeline's post-processing. There is intentionally only one merge
 * type across the crate — do not introduce a second.
 */
export enum WasmMergeMode {
    ObjectMerge = 0,
    ArrayConcat = 1,
    ObjectFirst = 2,
}

/**
 * Merged structured output plus validation bookkeeping.
 */
export class WasmMergedOutput {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmMergedOutput;
    constructor(merged: any, outcome: WasmOutcome, perBatchErrors: string[], errorMessage?: string | null);
    get errorMessage(): string | undefined;
    set errorMessage(value: string | null | undefined);
    merged: any;
    get outcome(): string;
    set outcome(value: WasmOutcome);
    perBatchErrors: string[];
}

/**
 * Extraction result metadata.
 *
 * Contains common fields applicable to all formats, format-specific metadata
 * via a discriminated union, and additional custom fields from postprocessors.
 */
export class WasmMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmMetadata;
    /**
     * Returns `true` when no metadata fields, format-specific metadata, or
     * additional postprocessor fields are populated.
     */
    isEmpty(): boolean;
    constructor(ocrUsed?: boolean | null, additional?: any | null, title?: string | null, subject?: string | null, authors?: string[] | null, keywords?: string[] | null, language?: string | null, createdAt?: string | null, modifiedAt?: string | null, createdBy?: string | null, modifiedBy?: string | null, pages?: WasmPageStructure | null, format?: any | null, imagePreprocessing?: WasmImagePreprocessingMetadata | null, jsonSchema?: any | null, error?: WasmErrorMetadata | null, extractionDurationMs?: bigint | null, category?: string | null, tags?: string[] | null, documentVersion?: string | null, abstractText?: string | null, outputFormat?: string | null);
    get abstractText(): string | undefined;
    set abstractText(value: string | null | undefined);
    additional: any;
    get authors(): string[] | undefined;
    set authors(value: string[] | null | undefined);
    get category(): string | undefined;
    set category(value: string | null | undefined);
    get createdAt(): string | undefined;
    set createdAt(value: string | null | undefined);
    get createdBy(): string | undefined;
    set createdBy(value: string | null | undefined);
    get documentVersion(): string | undefined;
    set documentVersion(value: string | null | undefined);
    get error(): WasmErrorMetadata | undefined;
    set error(value: WasmErrorMetadata | null | undefined);
    get extractionDurationMs(): bigint | undefined;
    set extractionDurationMs(value: bigint | null | undefined);
    get format(): any | undefined;
    set format(value: any | null | undefined);
    get imagePreprocessing(): WasmImagePreprocessingMetadata | undefined;
    set imagePreprocessing(value: WasmImagePreprocessingMetadata | null | undefined);
    get jsonSchema(): any | undefined;
    set jsonSchema(value: any | null | undefined);
    get keywords(): string[] | undefined;
    set keywords(value: string[] | null | undefined);
    get language(): string | undefined;
    set language(value: string | null | undefined);
    get modifiedAt(): string | undefined;
    set modifiedAt(value: string | null | undefined);
    get modifiedBy(): string | undefined;
    set modifiedBy(value: string | null | undefined);
    ocrUsed: boolean;
    get outputFormat(): string | undefined;
    set outputFormat(value: string | null | undefined);
    get pages(): WasmPageStructure | undefined;
    set pages(value: WasmPageStructure | null | undefined);
    get subject(): string | undefined;
    set subject(value: string | null | undefined);
    get tags(): string[] | undefined;
    set tags(value: string[] | null | undefined);
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * NER backend selector.
 */
export enum WasmNerBackendKind {
    Onnx = 0,
    Llm = 1,
    Candle = 2,
}

/**
 * Configuration for the NER post-processor.
 */
export class WasmNerConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmNerConfig;
    constructor(backend?: WasmNerBackendKind | null, categories?: any[] | null, customLabels?: string[] | null, model?: string | null, hfRepo?: string | null, hfModelFile?: string | null, hfTokenizerFile?: string | null, hfArchitecture?: WasmGlinerArchitecture | null, llm?: WasmLlmConfig | null, modelDir?: string | null, loraAdapterDir?: string | null);
    get backend(): string;
    set backend(value: WasmNerBackendKind);
    categories: string[];
    customLabels: string[];
    get hfArchitecture(): string | undefined;
    set hfArchitecture(value: WasmGlinerArchitecture | null | undefined);
    get hfModelFile(): string | undefined;
    set hfModelFile(value: string | null | undefined);
    get hfRepo(): string | undefined;
    set hfRepo(value: string | null | undefined);
    get hfTokenizerFile(): string | undefined;
    set hfTokenizerFile(value: string | null | undefined);
    get llm(): WasmLlmConfig | undefined;
    set llm(value: WasmLlmConfig | null | undefined);
    get loraAdapterDir(): string | undefined;
    set loraAdapterDir(value: string | null | undefined);
    get model(): string | undefined;
    set model(value: string | null | undefined);
    get modelDir(): string | undefined;
    set modelDir(value: string | null | undefined);
}

/**
 * Tagged enum for node content. Each variant carries only type-specific data.
 *
 * Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
 * Go/Java/TypeScript bindings.
 */
export class WasmNodeContent {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmNodeContent;
    constructor();
    get content(): string | undefined;
    set content(value: string | null | undefined);
    get definition(): string | undefined;
    set definition(value: string | null | undefined);
    get description(): string | undefined;
    set description(value: string | null | undefined);
    get entries(): any | undefined;
    set entries(value: any | null | undefined);
    get format(): string | undefined;
    set format(value: string | null | undefined);
    get grid(): WasmTableGrid | undefined;
    set grid(value: WasmTableGrid | null | undefined);
    get headingLevel(): number | undefined;
    set headingLevel(value: number | null | undefined);
    get headingText(): string | undefined;
    set headingText(value: string | null | undefined);
    get imageIndex(): number | undefined;
    set imageIndex(value: number | null | undefined);
    get key(): string | undefined;
    set key(value: string | null | undefined);
    get kind(): string | undefined;
    set kind(value: string | null | undefined);
    get label(): string | undefined;
    set label(value: string | null | undefined);
    get language(): string | undefined;
    set language(value: string | null | undefined);
    get level(): number | undefined;
    set level(value: number | null | undefined);
    nodeType: string;
    get number(): number | undefined;
    set number(value: number | null | undefined);
    get ordered(): boolean | undefined;
    set ordered(value: boolean | null | undefined);
    get src(): string | undefined;
    set src(value: string | null | undefined);
    get term(): string | undefined;
    set term(value: string | null | undefined);
    get text(): string | undefined;
    set text(value: string | null | undefined);
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * OCR backend types.
 */
export enum WasmOcrBackendType {
    Tesseract = 0,
    PaddleOCR = 1,
    Candle = 2,
    Custom = 3,
}

/**
 * Bounding geometry for an OCR element.
 *
 * Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
 * (from PaddleOCR and rotated text detection).
 */
export class WasmOcrBoundingGeometry {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrBoundingGeometry;
    constructor();
    get height(): number | undefined;
    set height(value: number | null | undefined);
    get left(): number | undefined;
    set left(value: number | null | undefined);
    get points(): any | undefined;
    set points(value: any | null | undefined);
    get top(): number | undefined;
    set top(value: number | null | undefined);
    type: string;
    get width(): number | undefined;
    set width(value: number | null | undefined);
}

/**
 * Confidence scores for an OCR element.
 *
 * Separates detection confidence (how confident that text exists at this location)
 * from recognition confidence (how confident about the actual text content).
 */
export class WasmOcrConfidence {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrConfidence;
    constructor(recognition?: number | null, detection?: number | null);
    get detection(): number | undefined;
    set detection(value: number | null | undefined);
    recognition: number;
}

/**
 * OCR configuration.
 */
export class WasmOcrConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrConfig;
    constructor(enabled?: boolean | null, backend?: string | null, language?: string[] | null, autoRotate?: boolean | null, vlmFallback?: any | null, tesseractConfig?: WasmTesseractConfig | null, outputFormat?: WasmOutputFormat | null, paddleOcrConfig?: any | null, backendOptions?: any | null, elementConfig?: WasmOcrElementConfig | null, qualityThresholds?: WasmOcrQualityThresholds | null, pipeline?: WasmOcrPipelineConfig | null, vlmConfig?: WasmLlmConfig | null, vlmPrompt?: string | null, acceleration?: WasmAccelerationConfig | null, tessdataBytes?: any | null, tessdataPath?: string | null);
    get acceleration(): WasmAccelerationConfig | undefined;
    set acceleration(value: WasmAccelerationConfig | null | undefined);
    autoRotate: boolean;
    backend: string;
    get backendOptions(): any | undefined;
    set backendOptions(value: any | null | undefined);
    get elementConfig(): WasmOcrElementConfig | undefined;
    set elementConfig(value: WasmOcrElementConfig | null | undefined);
    enabled: boolean;
    language: string[];
    get outputFormat(): string | undefined;
    set outputFormat(value: WasmOutputFormat | null | undefined);
    get paddleOcrConfig(): any | undefined;
    set paddleOcrConfig(value: any | null | undefined);
    get pipeline(): WasmOcrPipelineConfig | undefined;
    set pipeline(value: WasmOcrPipelineConfig | null | undefined);
    get qualityThresholds(): WasmOcrQualityThresholds | undefined;
    set qualityThresholds(value: WasmOcrQualityThresholds | null | undefined);
    get tessdataBytes(): any | undefined;
    set tessdataBytes(value: any | null | undefined);
    get tessdataPath(): string | undefined;
    set tessdataPath(value: string | null | undefined);
    get tesseractConfig(): WasmTesseractConfig | undefined;
    set tesseractConfig(value: WasmTesseractConfig | null | undefined);
    get vlmConfig(): WasmLlmConfig | undefined;
    set vlmConfig(value: WasmLlmConfig | null | undefined);
    vlmFallback: any;
    get vlmPrompt(): string | undefined;
    set vlmPrompt(value: string | null | undefined);
}

/**
 * A unified OCR element representing detected text with full metadata.
 *
 * This is the primary type for structured OCR output, preserving all information
 * from both Tesseract and PaddleOCR backends.
 */
export class WasmOcrElement {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrElement;
    constructor(text?: string | null, geometry?: any | null, confidence?: WasmOcrConfidence | null, level?: WasmOcrElementLevel | null, pageNumber?: number | null, backendMetadata?: any | null, rotation?: WasmOcrRotation | null, parentId?: string | null);
    backendMetadata: any;
    confidence: WasmOcrConfidence;
    geometry: any;
    get level(): string;
    set level(value: WasmOcrElementLevel);
    pageNumber: number;
    get parentId(): string | undefined;
    set parentId(value: string | null | undefined);
    get rotation(): WasmOcrRotation | undefined;
    set rotation(value: WasmOcrRotation | null | undefined);
    text: string;
}

/**
 * Configuration for OCR element extraction.
 *
 * Controls how OCR elements are extracted and filtered.
 */
export class WasmOcrElementConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrElementConfig;
    constructor(includeElements?: boolean | null, minLevel?: WasmOcrElementLevel | null, minConfidence?: number | null, buildHierarchy?: boolean | null);
    buildHierarchy: boolean;
    includeElements: boolean;
    minConfidence: number;
    get minLevel(): string;
    set minLevel(value: WasmOcrElementLevel);
}

/**
 * Hierarchical level of an OCR element.
 *
 * Maps to Tesseract's page segmentation hierarchy and provides
 * equivalent semantics for PaddleOCR.
 */
export enum WasmOcrElementLevel {
    Word = 0,
    Line = 1,
    Block = 2,
    Page = 3,
}

/**
 * OCR extraction result.
 *
 * Result of performing OCR on an image or scanned document,
 * including recognized text and detected tables.
 */
export class WasmOcrExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrExtractionResult;
    constructor(content?: string | null, mimeType?: string | null, metadata?: any | null, tables?: WasmOcrTable[] | null, ocrElements?: WasmOcrElement[] | null);
    content: string;
    metadata: any;
    mimeType: string;
    get ocrElements(): Array<any> | undefined;
    set ocrElements(value: WasmOcrElement[] | null | undefined);
    tables: WasmOcrTable[];
}

/**
 * OCR processing metadata.
 *
 * Captures information about OCR processing configuration and results.
 */
export class WasmOcrMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrMetadata;
    constructor(language?: string | null, psm?: number | null, outputFormat?: string | null, tableCount?: number | null, tableRows?: number | null, tableCols?: number | null);
    language: string;
    outputFormat: string;
    psm: number;
    get tableCols(): number | undefined;
    set tableCols(value: number | null | undefined);
    tableCount: number;
    get tableRows(): number | undefined;
    set tableRows(value: number | null | undefined);
}

/**
 * Multi-backend OCR pipeline with quality-based fallback.
 *
 * Backends are tried in priority order (highest first). After each backend
 * produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
 * the result is accepted. Otherwise the next backend is tried.
 */
export class WasmOcrPipelineConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrPipelineConfig;
    constructor(stages: WasmOcrPipelineStage[], qualityThresholds: WasmOcrQualityThresholds);
    qualityThresholds: WasmOcrQualityThresholds;
    stages: WasmOcrPipelineStage[];
}

/**
 * A single backend stage in the OCR pipeline.
 */
export class WasmOcrPipelineStage {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrPipelineStage;
    constructor(backend: string, priority: number, language?: string[] | null, tesseractConfig?: WasmTesseractConfig | null, paddleOcrConfig?: any | null, vlmConfig?: WasmLlmConfig | null, backendOptions?: any | null);
    backend: string;
    get backendOptions(): any | undefined;
    set backendOptions(value: any | null | undefined);
    get language(): string[] | undefined;
    set language(value: string[] | null | undefined);
    get paddleOcrConfig(): any | undefined;
    set paddleOcrConfig(value: any | null | undefined);
    priority: number;
    get tesseractConfig(): WasmTesseractConfig | undefined;
    set tesseractConfig(value: WasmTesseractConfig | null | undefined);
    get vlmConfig(): WasmLlmConfig | undefined;
    set vlmConfig(value: WasmLlmConfig | null | undefined);
}

/**
 * Quality thresholds for OCR fallback decisions and pipeline quality gating.
 *
 * All fields default to the values that match the previous hardcoded behavior,
 * so `OcrQualityThresholds.default()` preserves existing semantics exactly.
 */
export class WasmOcrQualityThresholds {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrQualityThresholds;
    constructor(minTotalNonWhitespace?: number | null, minNonWhitespacePerPage?: number | null, minMeaningfulWordLen?: number | null, minMeaningfulWords?: number | null, minAlnumRatio?: number | null, minGarbageChars?: number | null, maxFragmentedWordRatio?: number | null, criticalFragmentedWordRatio?: number | null, minAvgWordLength?: number | null, minWordsForAvgLengthCheck?: number | null, minConsecutiveRepeatRatio?: number | null, minWordsForRepeatCheck?: number | null, substantiveMinChars?: number | null, nonTextMinChars?: number | null, alnumWsRatioThreshold?: number | null, pipelineMinQuality?: number | null);
    alnumWsRatioThreshold: number;
    criticalFragmentedWordRatio: number;
    maxFragmentedWordRatio: number;
    minAlnumRatio: number;
    minAvgWordLength: number;
    minConsecutiveRepeatRatio: number;
    minGarbageChars: number;
    minMeaningfulWordLen: number;
    minMeaningfulWords: number;
    minNonWhitespacePerPage: number;
    minTotalNonWhitespace: number;
    minWordsForAvgLengthCheck: number;
    minWordsForRepeatCheck: number;
    nonTextMinChars: number;
    pipelineMinQuality: number;
    substantiveMinChars: number;
}

/**
 * Rotation information for an OCR element.
 */
export class WasmOcrRotation {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrRotation;
    constructor(angleDegrees: number, confidence?: number | null);
    angleDegrees: number;
    get confidence(): number | undefined;
    set confidence(value: number | null | undefined);
}

/**
 * Table detected via OCR.
 *
 * Represents a table structure recognized during OCR processing.
 */
export class WasmOcrTable {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrTable;
    constructor(cells: any, markdown: string, pageNumber: number, boundingBox?: WasmOcrTableBoundingBox | null);
    get boundingBox(): WasmOcrTableBoundingBox | undefined;
    set boundingBox(value: WasmOcrTableBoundingBox | null | undefined);
    cells: any;
    markdown: string;
    pageNumber: number;
}

/**
 * Bounding box for an OCR-detected table in pixel coordinates.
 */
export class WasmOcrTableBoundingBox {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmOcrTableBoundingBox;
    constructor(left: number, top: number, right: number, bottom: number);
    bottom: number;
    left: number;
    right: number;
    top: number;
}

/**
 * Outcome of validating and merging a set of batch responses.
 */
export enum WasmOutcome {
    Success = 0,
    PartialSuccess = 1,
    SchemaInvalid = 2,
    Error = 3,
}

/**
 * Output format for extraction results.
 *
 * Controls the format of the `content` field in `ExtractedDocument`.
 * When set to `Markdown`, `Djot`, or `Html`, the output uses that format.
 * `Plain` returns the raw extracted text.
 * `Structured` returns JSON with full OCR element data including bounding
 * boxes and confidence scores.
 */
export enum WasmOutputFormat {
    Plain = 0,
    Markdown = 1,
    Djot = 2,
    Html = 3,
    Json = 4,
    Structured = 5,
    Custom = 6,
}

/**
 * Byte offset boundary for a page.
 *
 * Tracks where a specific page's content starts and ends in the main content string,
 * enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
 * at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).
 */
export class WasmPageBoundary {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageBoundary;
    constructor(byteStart: number, byteEnd: number, pageNumber: number);
    byteEnd: number;
    byteStart: number;
    pageNumber: number;
}

/**
 * Classification result for a single page.
 */
export class WasmPageClassification {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageClassification;
    constructor(pageNumber: number, labels: WasmClassificationLabel[]);
    labels: WasmClassificationLabel[];
    pageNumber: number;
}

/**
 * Configuration for the page-classification post-processor.
 */
export class WasmPageClassificationConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageClassificationConfig;
    constructor(labels: string[], multiLabel: boolean, llm: WasmLlmConfig, promptTemplate?: string | null);
    labels: string[];
    llm: WasmLlmConfig;
    multiLabel: boolean;
    get promptTemplate(): string | undefined;
    set promptTemplate(value: string | null | undefined);
}

/**
 * Page extraction and tracking configuration.
 *
 * Controls how pages are extracted, tracked, and represented in the extraction results.
 * When `None`, page tracking is disabled.
 *
 * Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
 * when page boundaries are available and chunking is configured.
 */
export class WasmPageConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageConfig;
    constructor(extractPages?: boolean | null, insertPageMarkers?: boolean | null, markerFormat?: string | null);
    extractPages: boolean;
    insertPageMarkers: boolean;
    markerFormat: string;
}

/**
 * Content for a single page/slide.
 *
 * When page extraction is enabled, documents are split into per-page content
 * with associated tables and images mapped to each page.
 *
 * # Performance
 *
 * Uses Arc-wrapped tables and images for memory efficiency:
 * - `Vec<Arc<Table>>` enables zero-copy sharing of table data
 * - `Vec<Arc<ExtractedImage>>` enables zero-copy sharing of image data
 * - Maintains exact JSON compatibility via custom Serialize/Deserialize
 *
 * This reduces memory overhead for documents with shared tables/images
 * by avoiding redundant copies during serialization.
 */
export class WasmPageContent {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageContent;
    constructor(pageNumber: number, content: string, tables: WasmTable[], imageIndices: Uint32Array, hierarchy?: WasmPageHierarchy | null, isBlank?: boolean | null, layoutRegions?: WasmLayoutRegion[] | null, speakerNotes?: string | null, sectionName?: string | null, sheetName?: string | null);
    content: string;
    get hierarchy(): WasmPageHierarchy | undefined;
    set hierarchy(value: WasmPageHierarchy | null | undefined);
    imageIndices: Uint32Array;
    get isBlank(): boolean | undefined;
    set isBlank(value: boolean | null | undefined);
    get layoutRegions(): Array<any> | undefined;
    set layoutRegions(value: WasmLayoutRegion[] | null | undefined);
    pageNumber: number;
    get sectionName(): string | undefined;
    set sectionName(value: string | null | undefined);
    get sheetName(): string | undefined;
    set sheetName(value: string | null | undefined);
    get speakerNotes(): string | undefined;
    set speakerNotes(value: string | null | undefined);
    tables: WasmTable[];
}

/**
 * Page hierarchy structure containing heading levels and block information.
 *
 * Used when PDF text hierarchy extraction is enabled. Contains hierarchical
 * blocks with heading levels (H1-H6) for semantic document structure.
 */
export class WasmPageHierarchy {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageHierarchy;
    constructor(blockCount: number, blocks: WasmHierarchicalBlock[]);
    blockCount: number;
    blocks: WasmHierarchicalBlock[];
}

/**
 * A rendered page ready for inline-base64 transport to the vision model.
 */
export class WasmPageImage {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageImage;
    constructor(pageNumber: number, pngBytes: Uint8Array);
    pageNumber: number;
    pngBytes: Uint8Array;
}

/**
 * Metadata for individual page/slide/sheet.
 *
 * Captures per-page information including dimensions, content counts,
 * and visibility state (for presentations).
 */
export class WasmPageInfo {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageInfo;
    constructor(number: number, hasVectorGraphics: boolean, title?: string | null, imageCount?: number | null, tableCount?: number | null, hidden?: boolean | null, isBlank?: boolean | null);
    hasVectorGraphics: boolean;
    get hidden(): boolean | undefined;
    set hidden(value: boolean | null | undefined);
    get imageCount(): number | undefined;
    set imageCount(value: number | null | undefined);
    get isBlank(): boolean | undefined;
    set isBlank(value: boolean | null | undefined);
    number: number;
    get tableCount(): number | undefined;
    set tableCount(value: number | null | undefined);
    get title(): string | undefined;
    set title(value: string | null | undefined);
}

/**
 * Unified page structure for documents.
 *
 * Supports different page types (PDF pages, PPTX slides, Excel sheets)
 * with character offset boundaries for chunk-to-page mapping.
 */
export class WasmPageStructure {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPageStructure;
    constructor(totalCount: number, unitType: WasmPageUnitType, boundaries?: WasmPageBoundary[] | null, pages?: WasmPageInfo[] | null);
    get boundaries(): Array<any> | undefined;
    set boundaries(value: WasmPageBoundary[] | null | undefined);
    get pages(): Array<any> | undefined;
    set pages(value: WasmPageInfo[] | null | undefined);
    totalCount: number;
    get unitType(): string;
    set unitType(value: WasmPageUnitType);
}

/**
 * Type of paginated unit in a document.
 *
 * Distinguishes between different types of "pages" (PDF pages, presentation slides, spreadsheet sheets).
 */
export enum WasmPageUnitType {
    Page = 0,
    Slide = 1,
    Sheet = 2,
}

/**
 * One detected PII span in the input text.
 */
export class WasmPatternMatch {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPatternMatch;
    constructor(start: number, end: number, category: WasmPiiCategory, text: string);
    get category(): string;
    set category(value: WasmPiiCategory);
    end: number;
    start: number;
    text: string;
}

/**
 * A PDF annotation extracted from a document page.
 */
export class WasmPdfAnnotation {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPdfAnnotation;
    constructor(annotationType: WasmPdfAnnotationType, pageNumber: number, content?: string | null, boundingBox?: WasmBoundingBox | null);
    get annotationType(): string;
    set annotationType(value: WasmPdfAnnotationType);
    get boundingBox(): WasmBoundingBox | undefined;
    set boundingBox(value: WasmBoundingBox | null | undefined);
    get content(): string | undefined;
    set content(value: string | null | undefined);
    pageNumber: number;
}

/**
 * Type of PDF annotation.
 */
export enum WasmPdfAnnotationType {
    Text = 0,
    Highlight = 1,
    Link = 2,
    Stamp = 3,
    Underline = 4,
    StrikeOut = 5,
    Other = 6,
}

/**
 * A form field extracted from a PDF's AcroForm or XFA structure.
 *
 * Populated by the PDF extractor when `PdfConfig.extract_form_fields` is
 * enabled and the document is a fillable form. Supports both AcroForm (standard)
 * and XFA (XML Forms Architecture) layers. When both are present, AcroForm fields
 * take priority (canonical fallback per PDF spec), and XFA-only fields are appended.
 * The collection is empty for non-form PDFs and for non-PDF formats.
 *
 * `PdfConfig.extract_form_fields`: crate.core.config.PdfConfig.extract_form_fields
 */
export class WasmPdfFormField {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPdfFormField;
    constructor(name: string, fullName: string, fieldType: WasmFormFieldType, flags: number, value?: string | null, defaultValue?: string | null, page?: number | null, bbox?: WasmBoundingBox | null, maxLength?: number | null, tooltip?: string | null);
    get bbox(): WasmBoundingBox | undefined;
    set bbox(value: WasmBoundingBox | null | undefined);
    get defaultValue(): string | undefined;
    set defaultValue(value: string | null | undefined);
    get fieldType(): string;
    set fieldType(value: WasmFormFieldType);
    flags: number;
    fullName: string;
    get maxLength(): number | undefined;
    set maxLength(value: number | null | undefined);
    name: string;
    get page(): number | undefined;
    set page(value: number | null | undefined);
    get tooltip(): string | undefined;
    set tooltip(value: string | null | undefined);
    get value(): string | undefined;
    set value(value: string | null | undefined);
}

/**
 * PII categories the pattern engine recognises.
 */
export enum WasmPiiCategory {
    Email = 0,
    Phone = 1,
    Ssn = 2,
    CreditCard = 3,
    PostalCode = 4,
    IpAddress = 5,
    Iban = 6,
    SwiftBic = 7,
    DateOfBirth = 8,
    Person = 9,
    Organization = 10,
    Location = 11,
    Custom = 12,
}

/**
 * Post-processor configuration.
 */
export class WasmPostProcessorConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPostProcessorConfig;
    constructor(enabled?: boolean | null, enabledProcessors?: string[] | null, disabledProcessors?: string[] | null, enabledSet?: string[] | null, disabledSet?: string[] | null);
    get disabledProcessors(): string[] | undefined;
    set disabledProcessors(value: string[] | null | undefined);
    get disabledSet(): string[] | undefined;
    set disabledSet(value: string[] | null | undefined);
    enabled: boolean;
    get enabledProcessors(): string[] | undefined;
    set enabledProcessors(value: string[] | null | undefined);
    get enabledSet(): string[] | undefined;
    set enabledSet(value: string[] | null | undefined);
}

/**
 * Application properties from docProps/app.xml for PPTX
 *
 * Contains PowerPoint-specific document metadata.
 */
export class WasmPptxAppProperties {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPptxAppProperties;
    constructor(slideTitles?: string[] | null, application?: string | null, appVersion?: string | null, totalTime?: number | null, company?: string | null, docSecurity?: number | null, scaleCrop?: boolean | null, linksUpToDate?: boolean | null, sharedDoc?: boolean | null, hyperlinksChanged?: boolean | null, slides?: number | null, notes?: number | null, hiddenSlides?: number | null, multimediaClips?: number | null, presentationFormat?: string | null);
    get appVersion(): string | undefined;
    set appVersion(value: string | null | undefined);
    get application(): string | undefined;
    set application(value: string | null | undefined);
    get company(): string | undefined;
    set company(value: string | null | undefined);
    get docSecurity(): number | undefined;
    set docSecurity(value: number | null | undefined);
    get hiddenSlides(): number | undefined;
    set hiddenSlides(value: number | null | undefined);
    get hyperlinksChanged(): boolean | undefined;
    set hyperlinksChanged(value: boolean | null | undefined);
    get linksUpToDate(): boolean | undefined;
    set linksUpToDate(value: boolean | null | undefined);
    get multimediaClips(): number | undefined;
    set multimediaClips(value: number | null | undefined);
    get notes(): number | undefined;
    set notes(value: number | null | undefined);
    get presentationFormat(): string | undefined;
    set presentationFormat(value: string | null | undefined);
    get scaleCrop(): boolean | undefined;
    set scaleCrop(value: boolean | null | undefined);
    get sharedDoc(): boolean | undefined;
    set sharedDoc(value: boolean | null | undefined);
    slideTitles: string[];
    get slides(): number | undefined;
    set slides(value: number | null | undefined);
    get totalTime(): number | undefined;
    set totalTime(value: number | null | undefined);
}

/**
 * PowerPoint (PPTX) extraction result.
 *
 * Contains extracted slide content, metadata, and embedded images/tables.
 */
export class WasmPptxExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPptxExtractionResult;
    constructor(content: string, metadata: WasmPptxMetadata, slideCount: number, imageCount: number, tableCount: number, images: WasmExtractedImage[], officeMetadata: any, pageStructure?: WasmPageStructure | null, pageContents?: WasmPageContent[] | null, document?: WasmDocumentStructure | null, revisions?: WasmDocumentRevision[] | null);
    content: string;
    get document(): WasmDocumentStructure | undefined;
    set document(value: WasmDocumentStructure | null | undefined);
    imageCount: number;
    images: WasmExtractedImage[];
    metadata: WasmPptxMetadata;
    officeMetadata: any;
    get pageContents(): Array<any> | undefined;
    set pageContents(value: WasmPageContent[] | null | undefined);
    get pageStructure(): WasmPageStructure | undefined;
    set pageStructure(value: WasmPageStructure | null | undefined);
    get revisions(): Array<any> | undefined;
    set revisions(value: WasmDocumentRevision[] | null | undefined);
    slideCount: number;
    tableCount: number;
}

/**
 * PowerPoint presentation metadata.
 *
 * Extracted from PPTX files containing slide counts and presentation details.
 */
export class WasmPptxMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPptxMetadata;
    constructor(slideCount?: number | null, slideNames?: string[] | null, imageCount?: number | null, tableCount?: number | null);
    get imageCount(): number | undefined;
    set imageCount(value: number | null | undefined);
    slideCount: number;
    slideNames: string[];
    get tableCount(): number | undefined;
    set tableCount(value: number | null | undefined);
}

/**
 * Processing stages for post-processors.
 *
 * Post-processors are executed in stage order (Early → Middle → Late).
 * Use stages to control the order of post-processing operations.
 */
export enum WasmProcessingStage {
    Early = 0,
    Middle = 1,
    Late = 2,
}

/**
 * A non-fatal warning from a processing pipeline stage.
 *
 * Captures errors from optional features that don't prevent extraction
 * but may indicate degraded results.
 */
export class WasmProcessingWarning {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmProcessingWarning;
    constructor(source: string, message: string);
    message: string;
    source: string;
}

/**
 * A coarse progress event emitted during extraction.
 *
 * Intentionally minimal: a stage label plus optional detail and completion
 * fraction. Richer event shapes are layered on by the sink implementation.
 */
export class WasmProgressEvent {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmProgressEvent;
    constructor(stage: string, message?: string | null, fraction?: number | null);
    get fraction(): number | undefined;
    set fraction(value: number | null | undefined);
    get message(): string | undefined;
    set message(value: string | null | undefined);
    stage: string;
}

/**
 * Proxy configuration for HTTP requests.
 */
export class WasmProxyConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmProxyConfig;
    constructor(url?: string | null, username?: string | null, password?: string | null);
    get password(): string | undefined;
    set password(value: string | null | undefined);
    url: string;
    get username(): string | undefined;
    set username(value: string | null | undefined);
}

/**
 * Outlook PST archive metadata.
 */
export class WasmPstMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmPstMetadata;
    constructor(messageCount?: number | null);
    messageCount: number;
}

/**
 * Pixel-space bounding box of a QR code inside its source image.
 */
export class WasmQrBoundingBox {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmQrBoundingBox;
    constructor(x: number, y: number, width: number, height: number);
    height: number;
    width: number;
    x: number;
    y: number;
}

/**
 * One QR code decoded from an extracted image.
 */
export class WasmQrCode {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmQrCode;
    constructor(payload: string, confidence?: number | null, bbox?: WasmQrBoundingBox | null);
    get bbox(): WasmQrBoundingBox | undefined;
    set bbox(value: WasmQrBoundingBox | null | undefined);
    get confidence(): number | undefined;
    set confidence(value: number | null | undefined);
    payload: string;
}

/**
 * Configuration for the redaction post-processor.
 */
export class WasmRedactionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRedactionConfig;
    constructor(categories?: any[] | null, strategy?: WasmRedactionStrategy | null, preserveOffsets?: boolean | null, customTerms?: WasmRedactionTerm[] | null, customPatterns?: WasmRedactionPattern[] | null, preserveTerms?: WasmRedactionTerm[] | null, ner?: WasmNerConfig | null);
    /**
     * Validate user-supplied terms and patterns at config-construction time.
     *
     * Compiles every `RedactionPattern.pattern` (with the case-insensitive
     * inline flag where applicable) and returns the first compilation error so
     * the caller can reject the config before the redaction pipeline runs.
     * Pure terms (regex-escaped) cannot fail to compile, but the function
     * still rejects empty values to avoid degenerate zero-length matches.
     */
    validate(): void;
    categories: string[];
    customPatterns: WasmRedactionPattern[];
    customTerms: WasmRedactionTerm[];
    get ner(): WasmNerConfig | undefined;
    set ner(value: WasmNerConfig | null | undefined);
    preserveOffsets: boolean;
    preserveTerms: WasmRedactionTerm[];
    get strategy(): string;
    set strategy(value: WasmRedactionStrategy);
}

/**
 * One redaction event: which span was rewritten, why, and with what.
 */
export class WasmRedactionFinding {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRedactionFinding;
    constructor(start: number, end: number, category: WasmPiiCategory, strategy: WasmRedactionStrategy, replacementToken: string);
    get category(): string;
    set category(value: WasmPiiCategory);
    end: number;
    replacementToken: string;
    start: number;
    get strategy(): string;
    set strategy(value: WasmRedactionStrategy);
}

/**
 * One user-supplied regex pattern to redact.
 *
 * The pattern is compiled with the Rust `regex` crate (no look-around). Case
 * sensitivity is encoded in the pattern via the `(?i)` inline flag when
 * `Self.case_sensitive` is `false`.
 */
export class WasmRedactionPattern {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRedactionPattern;
    /**
     * Build a pattern with the given label (case-insensitive by default).
     */
    static labeled(label: string, pattern: string): WasmRedactionPattern;
    constructor(label: string, pattern: string, caseSensitive: boolean);
    caseSensitive: boolean;
    label: string;
    pattern: string;
}

/**
 * Audit report describing what the redaction processor found and how it replaced it.
 *
 * The redactor returns this alongside the rewritten content so compliance, replay, and
 * audit-log consumers can see exactly what fired. Offsets are relative to the *original*
 * pre-redaction `content` and are intended for audit reconstruction only — the original
 * bytes are dropped at the end of the pipeline.
 */
export class WasmRedactionReport {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRedactionReport;
    constructor(findings?: WasmRedactionFinding[] | null, totalRedacted?: number | null);
    findings: WasmRedactionFinding[];
    totalRedacted: number;
}

/**
 * Strategy applied when a PII match is rewritten.
 */
export enum WasmRedactionStrategy {
    Mask = 0,
    Hash = 1,
    TokenReplace = 2,
    Drop = 3,
}

/**
 * One user-supplied literal term to redact.
 *
 * Matched as a regex-escaped substring (so callers do not need to escape
 * metacharacters themselves). Case-insensitive by default — set
 * `Self.case_sensitive` to `true` for exact byte-match semantics.
 */
export class WasmRedactionTerm {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRedactionTerm;
    /**
     * Build a term with a custom label.
     */
    static labeled(label: string, value: string): WasmRedactionTerm;
    /**
     * Build a term whose label is the literal value itself (case-insensitive).
     */
    static literal(value: string): WasmRedactionTerm;
    constructor(label: string, value: string, caseSensitive: boolean);
    caseSensitive: boolean;
    label: string;
    value: string;
}

/**
 * One rejection-reason tally emitted by the redaction engine's
 * post-detection validators (see
 * `EntityValidator`).
 */
export class WasmRejectionCount {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRejectionCount;
    constructor(reason: string, count: number);
    count: number;
    reason: string;
}

/**
 * Counter for rejections, keyed by validator reason string.
 */
export class WasmRejectionCounts {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
}

/**
 * Semantic kind of a relationship between document elements.
 */
export enum WasmRelationshipKind {
    FootnoteReference = 0,
    CitationReference = 1,
    InternalLink = 2,
    Caption = 3,
    Label = 4,
    TocEntry = 5,
    CrossReference = 6,
}

/**
 * Configuration for the reranking pipeline.
 *
 * Controls which model to use, how many results to return, and download/cache
 * behavior for local ONNX models.
 *
 * Since v5.0.0.
 */
export class WasmRerankerConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRerankerConfig;
    constructor(model?: any | null, batchSize?: number | null, showDownloadProgress?: boolean | null, topK?: number | null, cacheDir?: string | null, acceleration?: WasmAccelerationConfig | null, maxRerankDurationSecs?: bigint | null);
    get acceleration(): WasmAccelerationConfig | undefined;
    set acceleration(value: WasmAccelerationConfig | null | undefined);
    batchSize: number;
    get cacheDir(): string | undefined;
    set cacheDir(value: string | null | undefined);
    get maxRerankDurationSecs(): bigint | undefined;
    set maxRerankDurationSecs(value: bigint | null | undefined);
    model: any;
    showDownloadProgress: boolean;
    get topK(): number | undefined;
    set topK(value: number | null | undefined);
}

/**
 * Reranker model types supported by Xberg.
 *
 * Since v5.0.0.
 */
export class WasmRerankerModelType {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRerankerModelType;
    constructor();
    get additionalFiles(): string[] | undefined;
    set additionalFiles(value: string[] | null | undefined);
    get llm(): WasmLlmConfig | undefined;
    set llm(value: WasmLlmConfig | null | undefined);
    get maxLength(): bigint | undefined;
    set maxLength(value: bigint | null | undefined);
    get modelFile(): string | undefined;
    set modelFile(value: string | null | undefined);
    get modelId(): string | undefined;
    set modelId(value: string | null | undefined);
    get name(): string | undefined;
    set name(value: string | null | undefined);
    type: string;
}

/**
 * Result-shape selection for extraction results.
 *
 * Distinct from `OutputFormat` (which controls rendering — Plain, Markdown,
 * HTML, etc.). `ResultFormat` controls the *shape* of the result: a unified content
 * blob vs. an element-based decomposition.
 */
export enum WasmResultFormat {
    Unified = 0,
    ElementBased = 1,
}

/**
 * Best-effort document location for a revision.
 */
export class WasmRevisionAnchor {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRevisionAnchor;
    constructor();
    get col(): number | undefined;
    set col(value: number | null | undefined);
    get index(): number | undefined;
    set index(value: number | null | undefined);
    get name(): string | undefined;
    set name(value: string | null | undefined);
    get row(): number | undefined;
    set row(value: number | null | undefined);
    get tableIndex(): number | undefined;
    set tableIndex(value: number | null | undefined);
    type: string;
}

/**
 * The content changes that make up a single revision.
 *
 * For insertions and deletions the `content` field carries the added/removed
 * lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
 * changes, `content` is empty — the property diff is left as a TODO for a
 * later enrichment pass.
 */
export class WasmRevisionDelta {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmRevisionDelta;
    constructor(content?: any | null, tableChanges?: WasmCellChange[] | null);
    content: any;
    tableChanges: WasmCellChange[];
}

/**
 * Semantic classification of a tracked change.
 */
export enum WasmRevisionKind {
    Insertion = 0,
    Deletion = 1,
    FormatChange = 2,
    Comment = 3,
}

/**
 * Configuration for security limits across extractors.
 *
 * All limits are intentionally conservative to prevent DoS attacks
 * while still supporting legitimate documents.
 */
export class WasmSecurityLimits {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmSecurityLimits;
    constructor(maxArchiveSize?: number | null, maxCompressionRatio?: number | null, maxFilesInArchive?: number | null, maxNestingDepth?: number | null, maxEntityLength?: number | null, maxContentSize?: number | null, maxIterations?: number | null, maxXmlDepth?: number | null, maxTableCells?: number | null);
    maxArchiveSize: number;
    maxCompressionRatio: number;
    maxContentSize: number;
    maxEntityLength: number;
    maxFilesInArchive: number;
    maxIterations: number;
    maxNestingDepth: number;
    maxTableCells: number;
    maxXmlDepth: number;
}

/**
 * SSRF policy configuration.
 */
export class WasmSsrfPolicy {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmSsrfPolicy;
    constructor(denyPrivate?: boolean | null, maxRedirects?: number | null);
    denyPrivate: boolean;
    maxRedirects: number;
}

/**
 * Structured data (Schema.org, microdata, RDFa) block.
 */
export class WasmStructuredData {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmStructuredData;
    constructor(dataType: WasmStructuredDataType, rawJson: string, schemaType?: string | null);
    get dataType(): string;
    set dataType(value: WasmStructuredDataType);
    rawJson: string;
    get schemaType(): string | undefined;
    set schemaType(value: string | null | undefined);
}

/**
 * Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).
 */
export class WasmStructuredDataResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmStructuredDataResult;
    constructor(content: string, format: string, metadata: any, textFields: string[]);
    content: string;
    format: string;
    metadata: any;
    textFields: string[];
}

/**
 * Structured data type classification.
 */
export enum WasmStructuredDataType {
    JsonLd = 0,
    Microdata = 1,
    RDFa = 2,
}

/**
 * Configuration for LLM-based structured data extraction.
 *
 * Sends extracted document content to a VLM with a JSON schema,
 * returning structured data that conforms to the schema.
 *
 * # Example
 *
 * ```toml
 * [structured_extraction]
 * schema_name = "invoice_data"
 * strict = true
 *
 * [structured_extraction.schema]
 * type = "object"
 * properties.vendor = { type = "string" }
 * properties.total = { type = "number" }
 * required = ["vendor", "total"]
 *
 * [structured_extraction.llm]
 * model = "openai/gpt-4o"
 * ```
 */
export class WasmStructuredExtractionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmStructuredExtractionConfig;
    constructor(schema: any, schemaName: string, strict: boolean, llm: WasmLlmConfig, schemaDescription?: string | null, prompt?: string | null);
    llm: WasmLlmConfig;
    get prompt(): string | undefined;
    set prompt(value: string | null | undefined);
    schema: any;
    get schemaDescription(): string | undefined;
    set schemaDescription(value: string | null | undefined);
    schemaName: string;
    strict: boolean;
}

/**
 * One vault match — either direction of lookup.
 */
export class WasmSubjectMatch {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmSubjectMatch;
    constructor(token: string, original: string, category?: string | null);
    get category(): string | undefined;
    set category(value: string | null | undefined);
    original: string;
    token: string;
}

/**
 * Configuration for the summarisation post-processor.
 */
export class WasmSummarizationConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmSummarizationConfig;
    constructor(strategy?: WasmSummaryStrategy | null, maxTokens?: number | null, llm?: WasmLlmConfig | null);
    get llm(): WasmLlmConfig | undefined;
    set llm(value: WasmLlmConfig | null | undefined);
    get maxTokens(): number | undefined;
    set maxTokens(value: number | null | undefined);
    get strategy(): string;
    set strategy(value: WasmSummaryStrategy);
}

/**
 * Summarisation strategy.
 */
export enum WasmSummaryStrategy {
    Extractive = 0,
    Abstractive = 1,
}

/**
 * A supported document format entry.
 *
 * Represents a file extension and its corresponding MIME type that Xberg can process.
 */
export class WasmSupportedFormat {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmSupportedFormat;
    constructor(extension: string, mimeType: string);
    extension: string;
    mimeType: string;
}

/**
 * Extracted table structure.
 *
 * Represents a table detected and extracted from a document (PDF, image, etc.).
 * Tables are converted to both structured cell data and Markdown format.
 */
export class WasmTable {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTable;
    constructor(cells?: any | null, markdown?: string | null, pageNumber?: number | null, boundingBox?: WasmBoundingBox | null);
    get boundingBox(): WasmBoundingBox | undefined;
    set boundingBox(value: WasmBoundingBox | null | undefined);
    cells: any;
    markdown: string;
    pageNumber: number;
}

/**
 * Individual table cell with content and optional styling.
 *
 * Future extension point for rich table support with cell-level metadata.
 */
export class WasmTableCell {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTableCell;
    constructor(content?: string | null, rowSpan?: number | null, colSpan?: number | null, isHeader?: boolean | null);
    colSpan: number;
    content: string;
    isHeader: boolean;
    rowSpan: number;
}

/**
 * Controls how markdown tables are handled when they exceed the chunk size limit.
 *
 * Only applies when `chunker_type` is `Markdown`.
 *
 * # Variants
 *
 * * `Split` - Default behavior: tables are split at row boundaries like any
 *   other block element. Continuation chunks contain only data rows without
 *   the header, which can break downstream consumers that need column context.
 * * `RepeatHeader` - Prepend the table header (header row + separator row) to
 *   every continuation chunk that contains data rows from the same table.
 *   Adds a small amount of duplicate text but ensures each chunk is
 *   self-contained for extraction, search, and LLM consumption.
 */
export enum WasmTableChunkingMode {
    Split = 0,
    RepeatHeader = 1,
}

/**
 * Structured table grid with cell-level metadata.
 *
 * Stores row/column dimensions and a flat list of cells with position info.
 */
export class WasmTableGrid {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTableGrid;
    constructor(rows?: number | null, cols?: number | null, cells?: WasmGridCell[] | null);
    cells: WasmGridCell[];
    cols: number;
    rows: number;
}

/**
 * Tesseract OCR configuration.
 *
 * Provides fine-grained control over Tesseract OCR engine parameters.
 * Most users can use the defaults, but these settings allow optimization
 * for specific document types (invoices, handwriting, etc.).
 */
export class WasmTesseractConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTesseractConfig;
    constructor(language?: string[] | null, psm?: number | null, outputFormat?: string | null, oem?: number | null, minConfidence?: number | null, enableTableDetection?: boolean | null, tableMinConfidence?: number | null, tableColumnThreshold?: number | null, tableRowThresholdRatio?: number | null, useCache?: boolean | null, classifyUsePreAdaptedTemplates?: boolean | null, languageModelNgramOn?: boolean | null, tesseditDontBlkrejGoodWds?: boolean | null, tesseditDontRowrejGoodWds?: boolean | null, tesseditEnableDictCorrection?: boolean | null, tesseditCharWhitelist?: string | null, tesseditCharBlacklist?: string | null, tesseditUsePrimaryParamsModel?: boolean | null, textordSpaceSizeIsVariable?: boolean | null, thresholdingMethod?: boolean | null, preprocessing?: WasmImagePreprocessingConfig | null);
    classifyUsePreAdaptedTemplates: boolean;
    enableTableDetection: boolean;
    language: string[];
    languageModelNgramOn: boolean;
    minConfidence: number;
    oem: number;
    outputFormat: string;
    get preprocessing(): WasmImagePreprocessingConfig | undefined;
    set preprocessing(value: WasmImagePreprocessingConfig | null | undefined);
    psm: number;
    tableColumnThreshold: number;
    tableMinConfidence: number;
    tableRowThresholdRatio: number;
    tesseditCharBlacklist: string;
    tesseditCharWhitelist: string;
    tesseditDontBlkrejGoodWds: boolean;
    tesseditDontRowrejGoodWds: boolean;
    tesseditEnableDictCorrection: boolean;
    tesseditUsePrimaryParamsModel: boolean;
    textordSpaceSizeIsVariable: boolean;
    thresholdingMethod: boolean;
    useCache: boolean;
}

/**
 * Inline text annotation — byte-range based formatting and links.
 *
 * Annotations reference byte offsets into the node's text content,
 * enabling precise identification of formatted regions.
 */
export class WasmTextAnnotation {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTextAnnotation;
    constructor(start: number, end: number, kind: any);
    end: number;
    kind: any;
    start: number;
}

/**
 * Text direction enumeration for HTML documents.
 */
export enum WasmTextDirection {
    LeftToRight = 0,
    RightToLeft = 1,
    Auto = 2,
}

/**
 * Plain text and Markdown extraction result.
 *
 * Contains the extracted text along with statistics and,
 * for Markdown files, structural elements like headers and links.
 */
export class WasmTextExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTextExtractionResult;
    constructor(content: string, lineCount: number, wordCount: number, characterCount: number, headers?: string[] | null);
    characterCount: number;
    content: string;
    get headers(): string[] | undefined;
    set headers(value: string[] | null | undefined);
    lineCount: number;
    wordCount: number;
}

/**
 * Text/Markdown metadata.
 *
 * Extracted from plain text and Markdown files. Includes word counts and,
 * for Markdown, structural elements like headers and links.
 */
export class WasmTextMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTextMetadata;
    constructor(lineCount?: number | null, wordCount?: number | null, characterCount?: number | null, headers?: string[] | null);
    characterCount: number;
    get headers(): string[] | undefined;
    set headers(value: string[] | null | undefined);
    lineCount: number;
    wordCount: number;
}

/**
 * Token reduction configuration.
 */
export class WasmTokenReductionOptions {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTokenReductionOptions;
    constructor(mode?: string | null, preserveImportantWords?: boolean | null);
    mode: string;
    preserveImportantWords: boolean;
}

/**
 * Translation of the extracted content.
 *
 * Holds the translated rendition of `ExtractedDocument.content` and (when
 * `preserve_markup` was requested) the translated `formatted_content`. Chunks
 * are translated in place inside `ExtractedDocument.chunks[*].content` rather
 * than duplicated here.
 */
export class WasmTranslation {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTranslation;
    constructor(targetLang: string, content: string, sourceLang?: string | null, formattedContent?: string | null);
    content: string;
    get formattedContent(): string | undefined;
    set formattedContent(value: string | null | undefined);
    get sourceLang(): string | undefined;
    set sourceLang(value: string | null | undefined);
    targetLang: string;
}

/**
 * Configuration for the translation post-processor.
 */
export class WasmTranslationConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmTranslationConfig;
    constructor(targetLang: string, preserveMarkup: boolean, llm: WasmLlmConfig, sourceLang?: string | null);
    llm: WasmLlmConfig;
    preserveMarkup: boolean;
    get sourceLang(): string | undefined;
    set sourceLang(value: string | null | undefined);
    targetLang: string;
}

/**
 * Semantic classification of an extracted URI.
 */
export enum WasmUriKind {
    Hyperlink = 0,
    Image = 1,
    Anchor = 2,
    Citation = 3,
    Reference = 4,
    Email = 5,
}

/**
 * URL ingestion and crawl configuration.
 */
export class WasmUrlExtractionConfig {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmUrlExtractionConfig;
    constructor(mode?: WasmUrlExtractionMode | null, crawl?: WasmCrawlConfig | null, allowLocalFileInputs?: boolean | null, allowFileUris?: boolean | null, documentUrlPattern?: string | null, maxDocumentUrlsPerResult?: number | null, maxTotalUrls?: number | null);
    allowFileUris: boolean;
    allowLocalFileInputs: boolean;
    crawl: WasmCrawlConfig;
    get documentUrlPattern(): string | undefined;
    set documentUrlPattern(value: string | null | undefined);
    get maxDocumentUrlsPerResult(): number | undefined;
    set maxDocumentUrlsPerResult(value: number | null | undefined);
    get maxTotalUrls(): number | undefined;
    set maxTotalUrls(value: number | null | undefined);
    get mode(): string;
    set mode(value: WasmUrlExtractionMode);
}

/**
 * URL extraction mode.
 */
export enum WasmUrlExtractionMode {
    Auto = 0,
    Document = 1,
    Crawl = 2,
}

/**
 * Outcome of a single validator on a single candidate match.
 */
export enum WasmValidationResult {
    Accept = 0,
    Reject = 1,
}

/**
 * Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.
 *
 * This knob is syntactic sugar over the explicit `OcrPipelineConfig` stage
 * ordering. When `vlm_fallback` is set and `pipeline` is `None`, an equivalent
 * pipeline is synthesised at extraction time:
 *
 * - `VlmFallbackPolicy.Disabled` — no synthesis; single-backend mode (default).
 * - `VlmFallbackPolicy.OnLowQuality` — tries the classical backend first; if the
 *   result scores below `quality_threshold`, tries VLM.
 * - `VlmFallbackPolicy.Always` — skips the classical backend and sends every page
 *   to the VLM.
 *
 * When `OcrConfig.pipeline` is explicitly set, `vlm_fallback` is ignored — the
 * explicit pipeline takes precedence.
 *
 * # Errors
 *
 * Both `OnLowQuality` and `Always` require `OcrConfig.vlm_config` to be `Some`.
 * Constructing an `OcrConfig` with one of these policies but no `vlm_config` is
 * detected by `OcrConfig.validate` and will surface as a
 * `Validation` error at extraction time, not a panic.
 *
 * # Example
 */
export class WasmVlmFallbackPolicy {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmVlmFallbackPolicy;
    constructor();
    mode: string;
    get qualityThreshold(): number | undefined;
    set qualityThreshold(value: number | null | undefined);
}

/**
 * Application properties from docProps/app.xml for XLSX
 *
 * Contains Excel-specific document metadata.
 */
export class WasmXlsxAppProperties {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmXlsxAppProperties;
    constructor(worksheetNames?: string[] | null, application?: string | null, appVersion?: string | null, docSecurity?: number | null, scaleCrop?: boolean | null, linksUpToDate?: boolean | null, sharedDoc?: boolean | null, hyperlinksChanged?: boolean | null, company?: string | null);
    get appVersion(): string | undefined;
    set appVersion(value: string | null | undefined);
    get application(): string | undefined;
    set application(value: string | null | undefined);
    get company(): string | undefined;
    set company(value: string | null | undefined);
    get docSecurity(): number | undefined;
    set docSecurity(value: number | null | undefined);
    get hyperlinksChanged(): boolean | undefined;
    set hyperlinksChanged(value: boolean | null | undefined);
    get linksUpToDate(): boolean | undefined;
    set linksUpToDate(value: boolean | null | undefined);
    get scaleCrop(): boolean | undefined;
    set scaleCrop(value: boolean | null | undefined);
    get sharedDoc(): boolean | undefined;
    set sharedDoc(value: boolean | null | undefined);
    worksheetNames: string[];
}

/**
 * XML extraction result.
 *
 * Contains extracted text content from XML files along with
 * structural statistics about the XML document.
 */
export class WasmXmlExtractionResult {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmXmlExtractionResult;
    constructor(content: string, elementCount: number, uniqueElements: string[]);
    content: string;
    elementCount: number;
    uniqueElements: string[];
}

/**
 * XML metadata extracted during XML parsing.
 *
 * Provides statistics about XML document structure.
 */
export class WasmXmlMetadata {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmXmlMetadata;
    constructor(elementCount?: number | null, uniqueElements?: string[] | null);
    elementCount: number;
    uniqueElements: string[];
}

/**
 * Year range for bibliographic metadata.
 */
export class WasmYearRange {
    free(): void;
    [Symbol.dispose](): void;
    static default(): WasmYearRange;
    constructor(years: Uint32Array, min?: number | null, max?: number | null);
    get max(): number | undefined;
    set max(value: number | null | undefined);
    get min(): number | undefined;
    set min(value: number | null | undefined);
    years: Uint32Array;
}

/**
 * Stateful engine handle exposed to JS.
 *
 * Constructed via `XbergEngine.new(config, injection)` where `config` may
 * contain optional settings (e.g. `bridgeTimeoutMs`) and `injection` is a
 * plain object with optional `embedder`, `store`, `ner`, and `ocr` keys.
 */
export class XbergEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return aggregate statistics for the WASM extraction cache.
     */
    cache_stats(): any;
    /**
     * Decrypt an encrypted blob back into a token→original map.
     */
    decrypt_map(blob: Uint8Array, passphrase: string): any;
    /**
     * Detect PII in `text`. Returns an array of `{ start, end, category, text }`.
     */
    detect_pii(text: string, categories?: string[] | null): any;
    /**
     * Encrypt a rehydration map with `passphrase`.
     *
     * Returns the raw ciphertext bytes (`XPII\x01` wire format).
     */
    encrypt_map(map: any, passphrase: string): Uint8Array;
    /**
     * Extract content from a single bytes or URI input.
     */
    extract(input: any, config: any): Promise<any>;
    /**
     * Search a decrypted rehydration map for `query`, matching either the
     * token (exact) or the original value (case-insensitive substring).
     *
     * Returns an array of `{ token, original, category }`.
     */
    find_subject(map: any, query: string): any;
    /**
     * Remove every mapping in `map` whose token or original value matches
     * `query`. Mutates a copy and returns
     * `{ removed: [{ token, original, category }], remaining: { token: original } }` —
     * the caller re-encrypts `remaining` with [`XbergEngine::encrypt_map`]
     * and persists it; this method does not touch disk.
     */
    forget_subject(map: any, query: string): any;
    /**
     * Ingest a single document into the RAG vector store.
     *
     * Requires an `embedder` and a `store` to have been injected. For PII+NER
     * redaction (mandatory when `pipeline-redaction` is enabled), the engine
     * resolves NER in this order:
     * 1. **Injected JS NER bridge** — the `ner` object from the constructor
     *    injection, if present. This is the preferred path in browser contexts.
     * 2. **Candle backend** — the in-binary GLiNER2 model loaded via
     *    `initCandleNer`. Used as fallback when no JS bridge is injected.
     *
     * If neither is available, ingestion fails with a clear error.
     *
     * `config` is an optional object; only `chunking.maxCharacters` and
     * `chunking.overlap` are currently supported. All other fields are
     * ignored.
     *
     * Returns `{ document_id, rehydration_map, pii_category_counts }`. The
     * caller decides whether/how to persist or encrypt `rehydration_map` —
     * this method never does so itself (use `encryptMap` separately).
     */
    ingest(doc: any, collection: string, config?: any | null): Promise<any>;
    /**
     * Invalidate all cached extraction results.
     */
    invalidate_cache(): void;
    /**
     * Perform Named Entity Recognition on `text`.
     *
     * Returns entities as a JSON-serializable JsValue array.
     */
    ner(text: string, opts: any): Promise<any>;
    /**
     * Create a new engine with injected bridges.
     *
     * `config` may contain:
     * - `bridgeTimeoutMs` — timeout in milliseconds for JS bridge calls
     *   (defaults to 30,000ms if not provided)
     *
     * `injection` may contain:
     * - `embedder` — object with `embed(texts: string[]): Promise<number[][]>`
     * - `store`    — object implementing the VectorStore JS protocol
     * - `ner`      — object with `ner(text, categories): Promise<...>`
     *                **NOTE:** this injected NER bridge is ONLY used by
     *                `XbergEngine::ner()`. It does NOT satisfy `ingest()`'s
     *                NER requirement — `ingest()` uses the Candle backend
     *                via `initCandleNer()`, which must be called separately.
     * - `ocr`      — object with `ocr(imageBytes, opts): Promise<{ text: string, lines?: Array<{ text: string, confidence: number, bbox?: { x: number, y: number, w: number, h: number } }> }>`
     */
    constructor(config: any, injection: any);
    /**
     * Perform OCR on image bytes, returning extracted text with per-line
     * confidence and bounding-box geometry (when the backend provides it).
     */
    ocr(bytes: Uint8Array, opts: any): Promise<any>;
    /**
     * Query the RAG vector store with `q` in `collection`, returning top `k` results.
     *
     * Requires a `store` injection. If an `embedder` is also available, the query
     * text will be embedded for vector similarity; otherwise full-text mode is used.
     */
    query(q: string, collection: string, k: number): Promise<any>;
    /**
     * Redact PII from `text` using the given `strategy`.
     *
     * Returns `{ redacted: string, rehydrationMap: { token: original } }`.
     *
     * NOTE: This method reimplements redaction logic inline rather than delegating
     * to `xberg::text::redaction::redact`. In a future pass this should be replaced
     * with a direct call to the core redaction API to avoid duplication.
     */
    redact(text: string, strategy?: string | null, categories?: string[] | null): any;
    /**
     * Decrypt a rehydration map and substitute tokens in `doc`.
     *
     * Returns the dehydrated text with original PII values restored.
     */
    rehydrate(doc: string, map_bytes: Uint8Array, passphrase: string): string;
}

export function clearDocumentExtractors(): void;

export function clearEmbeddingBackends(): void;

export function clearOcrBackends(): void;

export function clearPostProcessors(): void;

export function clearRenderers(): void;

export function clearRerankerBackends(): void;

export function clearValidators(): void;

/**
 * Compresses multiple entries into a 7z archive in WebAssembly environment.
 *
 * This function creates a compressed archive from multiple file entries,
 * designed specifically for WASM targets.
 *
 * # Arguments
 * * `entries` - Vector of JavaScript strings representing file names/paths
 * * `datas` - Vector of Uint8Arrays containing the file data corresponding to entries
 */
export function compress(entries: string[], datas: Uint8Array[]): Uint8Array;

/**
 * Decompresses a 7z archive in WebAssembly environment.
 *
 * This function is specifically designed for WASM targets and uses JavaScript interop
 * to handle the decompression process with a callback function.
 *
 * # Arguments
 * * `src` - Uint8Array containing the compressed archive data
 * * `pwd` - Password string for encrypted archives (use empty string for unencrypted)
 * * `f` - JavaScript callback function to handle extracted entries
 */
export function decompress(src: Uint8Array, pwd: string, f: Function): void;

/**
 * Pick the highest-priority match among overlapping spans.
 *
 * Strategy: walk matches in (start, -length) order; keep a match only if its
 * start is at or after the previously-kept end. This is a standard interval
 * dedupe that prefers earlier and longer spans.
 */
export function dedupeOverlaps(matches: WasmPatternMatch[]): WasmPatternMatch[];

/**
 * Extract content from a single bytes or URI input.
 */
export function extract(input: any, config: any): Promise<WasmExtractionResult>;

/**
 * Extract content from multiple bytes or URI inputs.
 */
export function extractBatch(inputs: WasmExtractInput[], config: any): Promise<WasmExtractionResult>;

/**
 * Initialize the in-binary Candle NER fallback from in-memory model bytes.
 * JS calls this once, after downloading the pinned PII model's
 * `model.safetensors`, `tokenizer.json`, and `encoder_config/config.json`.
 * Calling this more than once replaces the previously-loaded model.
 */
export function initCandleNer(safetensors: Uint8Array, tokenizer_json: Uint8Array, encoder_config_json: Uint8Array): void;

/**
 * List names of all registered document extractors.
 */
export function listDocumentExtractors(): string[];

/**
 * List the names of all registered embedding backends.
 *
 * Used by `xberg-cli`, the api/mcp endpoints, and generated language
 * bindings.
 */
export function listEmbeddingBackends(): string[];

/**
 * List all registered OCR backends.
 *
 * Returns the names of all OCR backends currently registered in the global registry.
 *
 * # Returns
 *
 * A vector of OCR backend names.
 *
 * # Example
 */
export function listOcrBackends(): string[];

/**
 * List all registered post-processor names.
 *
 * Returns a vector of all post-processor names currently registered in the
 * global registry.
 *
 * # Returns
 *
 * - `Ok(Vec<String>)` - Vector of post-processor names
 * - `Err(...)` if the registry lock is poisoned
 *
 * # Example
 */
export function listPostProcessors(): string[];

/**
 * List names of all registered renderers.
 *
 * # Errors
 *
 * Returns an error if the registry lock is poisoned.
 */
export function listRenderers(): string[];

/**
 * List the names of all registered reranker backends.
 *
 * Used by `xberg-cli`, the api/mcp endpoints, and generated language
 * bindings.
 *
 * Since v5.0.0.
 */
export function listRerankerBackends(): string[];

/**
 * List all supported document formats.
 *
 * Returns every file extension Xberg recognizes together with its
 * corresponding MIME type, derived from the central format registry.
 * Formats that have no registered file extension (such as source code,
 * which is detected dynamically) are not included.
 *
 * The list is sorted alphabetically by file extension.
 *
 * # Returns
 *
 * A vector of `SupportedFormat` entries sorted by extension.
 *
 * # Example
 */
export function listSupportedFormats(): WasmSupportedFormat[];

/**
 * List names of all registered validators.
 */
export function listValidators(): string[];

export function registerDocumentExtractor(backend: any): void;

export function registerEmbeddingBackend(backend: any): void;

export function registerOcrBackend(backend: any): void;

export function registerPostProcessor(backend: any): void;

export function registerRenderer(backend: any): void;

export function registerRerankerBackend(backend: any): void;

export function registerValidator(backend: any): void;

export function unregisterDocumentExtractor(name: string): void;

export function unregisterEmbeddingBackend(name: string): void;

export function unregisterOcrBackend(name: string): void;

export function unregisterPostProcessor(name: string): void;

export function unregisterRenderer(name: string): void;

export function unregisterRerankerBackend(name: string): void;

export function unregisterValidator(name: string): void;
