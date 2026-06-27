```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    chunking: const ChunkingConfig(
      maxCharacters: 1000,
      overlap: 200,
      trim: true,
      chunkerType: ChunkerType.markdown,
      sizing: ChunkSizing.characters(),
      prependHeadingContext: true,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);
  final chunks = result.chunks ?? const [];
  print('Chunks: ${chunks.length}');
}
```
