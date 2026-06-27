```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    ocr: const OcrConfig(
      enabled: true,
      backend: 'tesseract',
      language: 'eng+deu',
      autoRotate: false,
      tesseractConfig: TesseractConfig(
        language: 'eng+deu',
        psm: 6,
        outputFormat: 'text',
        oem: 3,
        minConfidence: 0.0,
        enableTableDetection: false,
        tableMinConfidence: 0.5,
        tableColumnThreshold: 20,
        tableRowThresholdRatio: 0.5,
        useCache: true,
        classifyUsePreAdaptedTemplates: false,
        languageModelNgramOn: false,
        tesseditDontBlkrejGoodWds: false,
        tesseditDontRowrejGoodWds: false,
        tesseditEnableDictCorrection: false,
        tesseditCharWhitelist: '',
        tesseditCharBlacklist: '',
        tesseditUsePrimaryParamsModel: false,
        textordSpaceSizeIsVariable: false,
        thresholdingMethod: false,
      ),
    ),
    chunking: const ChunkingConfig(
      maxCharacters: 1000,
      overlap: 200,
      trim: true,
      chunkerType: ChunkerType.text,
      sizing: ChunkSizing.characters(),
      prependHeadingContext: false,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);
  print('Content length: ${result.content.length}');
}
```
