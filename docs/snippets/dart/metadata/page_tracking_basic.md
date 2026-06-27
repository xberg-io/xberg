```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final pageConfig = PageConfig(
    extractPages: true,
    insertPageMarkers: false,
    markerFormat: '<!-- page {page} -->',
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
    useLayoutForMarkdown: false,    pages: pageConfig,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);

  final pages = result.pages;
  if (pages == null) {
    print('No per-page content available');
    return;
  }

  for (final page in pages) {
    print('Page ${page.pageNumber}:');
    print('  Content: ${page.content.length} chars');
    print('  Tables: ${page.tables.length}');
    print('  Images: ${page.images.length}');
  }
}
```
