```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    images: const ImageExtractionConfig(
      extractImages: true,
      targetDpi: 300,
      maxImageDimension: 4096,
      injectPlaceholders: false,
      autoAdjustDpi: true,
      minDpi: 150,
      maxDpi: 600,
      classify: false,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('document.pdf', null, config);
  final images = result.images ?? const [];
  print('Extracted images: ${images.length}');
}
```
