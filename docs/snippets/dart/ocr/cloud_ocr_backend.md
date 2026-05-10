```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Cloud OCR backends are registered in the Rust core. From Dart, select a
  // registered backend by name. Use `KreuzbergBridge.listOcrBackends()` to
  // discover available backends at runtime.
  final backends = await KreuzbergBridge.listOcrBackends();
  print('Available OCR backends: $backends');

  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    ocr: const OcrConfig(
      enabled: true,
      backend: 'cloud',
      language: 'en',
      autoRotate: false,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('scanned.pdf', null, config);
  print(result.content);
}
```
