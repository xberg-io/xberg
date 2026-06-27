```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    pdfOptions: const PdfConfig(
      extractImages: true,
      passwords: <String>['password123'],
      extractMetadata: true,
      extractAnnotations: false,
      allowSingleColumnTables: false,
      hierarchy: HierarchyConfig(
        enabled: true,
        kClusters: 4,
        includeBbox: false,
      ),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('encrypted.pdf', null, config);
  print('Title: ${result.metadata.title}');
}
```
