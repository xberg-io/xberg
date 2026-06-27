```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    languageDetection: const LanguageDetectionConfig(
      enabled: true,
      minConfidence: 0.8,
      detectMultiple: true,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('multilingual_document.pdf', null, config);
  print('Detected languages: ${result.detectedLanguages}');
}
```
