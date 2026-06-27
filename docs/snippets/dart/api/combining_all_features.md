```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    // OCR: Tesseract on English text
    forceOcr: false,
    disableOcr: false,
    ocr: const OcrConfig(
      enabled: true,
      backend: 'tesseract',
      language: 'eng',
      autoRotate: false,
    ),
    // Chunking: ~800-character markdown chunks with 100-char overlap
    chunking: const ChunkingConfig(
      maxCharacters: 800,
      overlap: 100,
      trim: true,
      chunkerType: ChunkerType.markdown,
      sizing: ChunkSizing.characters(),
      prependHeadingContext: true,
    ),
    // Image extraction
    images: const ImageExtractionConfig(
      extractImages: true,
      targetDpi: 150,
      maxImageDimension: 4096,
      injectPlaceholders: false,
      autoAdjustDpi: true,
      minDpi: 72,
      maxDpi: 300,
      classify: false,
    ),
    // Output: markdown with full document structure
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.markdown(),
    includeDocumentStructure: true,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('report.pdf', null, config);

  print('Content (${result.content.length} chars):');
  final preview = result.content.substring(
    0,
    result.content.length < 200 ? result.content.length : 200,
  );
  print(preview);

  if (result.chunks != null) {
    print('\nChunks: ${result.chunks!.length}');
  }
  print('Tables: ${result.tables.length}');
  if (result.detectedLanguages != null) {
    print('Languages: ${result.detectedLanguages}');
  }
  if (result.extractionMethod != null) {
    print('Extraction method: ${result.extractionMethod}');
  }
}
```
