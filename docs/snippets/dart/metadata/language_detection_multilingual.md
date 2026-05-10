```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final languageDetection = LanguageDetectionConfig(
    enabled: true,
    minConfidence: 0.3,
    detectMultiple: true,
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
    languageDetection: languageDetection,
  );

  final result = await KreuzbergBridge.extractFile('multilingual.pdf', null, config);

  final detected = result.detectedLanguages;
  if (detected == null || detected.isEmpty) {
    print('No languages detected');
    return;
  }

  print('Detected ${detected.length} language(s):');
  for (final language in detected) {
    print('  - $language');
  }
}
```
