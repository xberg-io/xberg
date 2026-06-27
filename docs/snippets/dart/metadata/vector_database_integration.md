```dart title="Dart"
import 'package:xberg/xberg.dart';

class VectorRecord {
  final String id;
  final List<double> embedding;
  final String content;
  final Map<String, Object?> metadata;

  const VectorRecord({
    required this.id,
    required this.embedding,
    required this.content,
    required this.metadata,
  });
}

void storeInVectorDatabase(List<VectorRecord> records) {
  for (final record in records) {
    if (record.embedding.isEmpty) {
      continue;
    }
    print(
      'Storing ${record.id}: ${record.content.length} chars, '
      '${record.embedding.length} dims',
    );
  }
}

Future<List<VectorRecord>> extractAndVectorize(
  String documentPath,
  String documentId,
) async {
  final embedding = EmbeddingConfig(
    model: EmbeddingModelType.preset(name: 'balanced'),
    normalize: true,
    batchSize: 32,
    showDownloadProgress: false,
  );

  final chunking = ChunkingConfig(
    maxCharacters: 512,
    overlap: 50,
    trim: true,
    chunkerType: ChunkerType.text,
    embedding: embedding,
    sizing: ChunkSizing.characters(),
    prependHeadingContext: false,
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
    useLayoutForMarkdown: false,    chunking: chunking,
  );

  final result = await XbergBridge.extract(documentPath, null, config);
  final chunks = result.chunks ?? const <Chunk>[];

  final records = <VectorRecord>[];
  for (var index = 0; index < chunks.length; index++) {
    final chunk = chunks[index];
    final embeddingValues = chunk.embedding?.toList() ?? const <double>[];

    records.add(
      VectorRecord(
        id: '${documentId}_chunk_$index',
        content: chunk.content,
        embedding: embeddingValues,
        metadata: {
          'document_id': documentId,
          'chunk_index': index,
          'content_length': chunk.content.length,
        },
      ),
    );
  }

  storeInVectorDatabase(records);
  return records;
}

Future<void> main() async {
  await extractAndVectorize('document.pdf', 'doc-1');
}
```
