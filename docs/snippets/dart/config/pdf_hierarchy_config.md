```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    pdfOptions: const PdfConfig(
      extractImages: false,
      extractMetadata: true,
      extractAnnotations: false,
      allowSingleColumnTables: false,
      hierarchy: HierarchyConfig(
        enabled: true,
        kClusters: 5,
        includeBbox: true,
        ocrCoverageThreshold: 0.8,
      ),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('document.pdf', null, config);
  final pages = result.pages ?? const [];
  print('Pages with hierarchy: ${pages.where((p) => p.hierarchy != null).length}');
}
```
