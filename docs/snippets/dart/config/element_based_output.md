```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    resultFormat: ResultFormat.elementBased,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('document.pdf', null, config);
  final elements = result.elements ?? const [];
  for (final element in elements) {
    print('Type: ${element.elementType}');
    final preview = element.text.substring(
      0,
      element.text.length < 100 ? element.text.length : 100,
    );
    print('Text: $preview');
    print('---');
  }
}
```
