```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final languageDetection = LanguageDetectionConfig(
    enabled: true,
    minConfidence: 0.5,
    detectMultiple: false,
  );

  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,    languageDetection: languageDetection,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);

  final detected = result.detectedLanguages;
  if (detected != null && detected.isNotEmpty) {
    print('Primary language: ${detected.first}');
  } else {
    print('No language detected');
  }
}
```
