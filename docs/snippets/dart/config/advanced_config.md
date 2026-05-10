```dart title="Dart"
import 'dart:typed_data';

import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    ocr: const OcrConfig(
      enabled: true,
      backend: 'tesseract',
      language: 'eng',
      autoRotate: false,
    ),
    chunking: const ChunkingConfig(
      maxCharacters: 1000,
      overlap: 200,
      trim: true,
      chunkerType: ChunkerType.text,
      sizing: ChunkSizing.characters(),
      prependHeadingContext: false,
      embedding: EmbeddingConfig(
        model: EmbeddingModelType.preset(name: 'balanced'),
        normalize: true,
        batchSize: 32,
        showDownloadProgress: false,
      ),
    ),
    languageDetection: const LanguageDetectionConfig(
      enabled: true,
      minConfidence: 0.8,
      detectMultiple: false,
    ),
    keywords: KeywordConfig(
      algorithm: KeywordAlgorithm.yake,
      maxKeywords: 10,
      minScore: 0.1,
      ngramRange: Int64List.fromList(<int>[1, 3]),
      language: 'en',
    ),
    tokenReduction: const TokenReductionOptions(
      mode: 'moderate',
      preserveImportantWords: true,
    ),
    postprocessor: const PostProcessorConfig(enabled: true),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('document.pdf', null, config);
  print('Content: ${result.content}');
  if (result.detectedLanguages != null) {
    print('Languages: ${result.detectedLanguages}');
  }
  final chunks = result.chunks ?? const [];
  print('Chunks: ${chunks.length}');
}
```
