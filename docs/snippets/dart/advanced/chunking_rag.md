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
      embedding: EmbeddingConfig(
        model: EmbeddingModelType.preset(name: 'balanced'),
        normalize: true,
        batchSize: 32,
        showDownloadProgress: false,
      ),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('research_paper.pdf', null, config);
  final chunks = result.chunks ?? const [];
  for (final chunk in chunks) {
    final index = chunk.metadata.chunkIndex;
    final total = chunk.metadata.totalChunks;
    final start = chunk.metadata.byteStart;
    final end = chunk.metadata.byteEnd;
    final preview = chunk.content.length > 100
        ? chunk.content.substring(0, 100)
        : chunk.content;
    print('Chunk ${index + 1}/$total');
    print('Position: $start-$end');
    print('Content: $preview...');
    final embedding = chunk.embedding;
    if (embedding != null) {
      print('Embedding: ${embedding.length} dimensions');
    }
  }
}
```
