```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  // Dart bindings do not expose config-file discovery. Build a default
  // ExtractionConfig in code and pass it explicitly to XbergBridge.extract.
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);
  print(result.content);
}
```
