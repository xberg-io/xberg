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
      backend: 'paddleocr',
      language: 'en',
      autoRotate: false,
      elementConfig: OcrElementConfig(
        includeElements: true,
        minLevel: OcrElementLevel.word,
        minConfidence: 0.0,
        buildHierarchy: false,
      ),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('scanned.pdf', null, config);
  final elements = result.ocrElements ?? const <OcrElement>[];
  for (final element in elements) {
    print('Text: ${element.text}');
    print('Confidence: ${element.confidence.recognition.toStringAsFixed(2)}');
    print('Level: ${element.level}');
    print('Page: ${element.pageNumber}');
  }
}
```
