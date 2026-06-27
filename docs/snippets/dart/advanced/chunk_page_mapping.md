```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    chunking: const ChunkingConfig(
      maxCharacters: 500,
      overlap: 50,
      trim: true,
      chunkerType: ChunkerType.text,
      sizing: ChunkSizing.characters(),
      prependHeadingContext: false,
    ),
    pages: const PageConfig(
      extractPages: true,
      insertPageMarkers: false,
      markerFormat: '',
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);
  final chunks = result.chunks ?? const [];
  for (final chunk in chunks) {
    final first = chunk.metadata.firstPage;
    final last = chunk.metadata.lastPage;
    if (first != null && last != null) {
      final preview = chunk.content.length > 50
          ? chunk.content.substring(0, 50)
          : chunk.content;
      final pageRange = first == last ? 'Page $first' : 'Pages $first-$last';
      print('Chunk: $preview... ($pageRange)');
    }
  }
}
```
