```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
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

  final result = await XbergBridge.extract('scanned_document.pdf', null, config);
  final score = result.qualityScore;
  if (score != null) {
    if (score < 0.5) {
      print('Warning: Low quality extraction (${score.toStringAsFixed(2)})');
    } else {
      print('Quality score: ${score.toStringAsFixed(2)}');
    }
  }
  for (final warning in result.processingWarnings) {
    print('Warning: $warning');
  }
}
```
