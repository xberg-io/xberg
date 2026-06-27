```dart title="Dart"
import 'dart:typed_data';

import 'package:xberg/xberg.dart';

class VectorRecord {
  VectorRecord({
    required this.id,
    required this.content,
    required this.embedding,
    required this.metadata,
  });

  final String id;
  final String content;
  final Float64List embedding;
  final Map<String, String> metadata;
}

Future<List<VectorRecord>> extractAndVectorize(
  String documentPath,
  String documentId,
) async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    chunking: const ChunkingConfig(
      maxCharacters: 512,
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

  final result = await XbergBridge.extract(ExtractInput(kind: ExtractInputKind.uri, uri: documentPath), config: config);
  final records = <VectorRecord>[];
  final chunks = result.chunks ?? const [];
  for (var index = 0; index < chunks.length; index++) {
    final chunk = chunks[index];
    final embedding = chunk.embedding;
    if (embedding == null) {
      continue;
    }
    records.add(VectorRecord(
      id: '${documentId}_chunk_$index',
      content: chunk.content,
      embedding: embedding,
      metadata: <String, String>{
        'document_id': documentId,
        'chunk_index': index.toString(),
        'content_length': chunk.content.length.toString(),
      },
    ));
  }
  return records;
}

Future<void> main() async {
  final records = await extractAndVectorize('document.pdf', 'doc-001');
  print('Vector records: ${records.length}');
}
```
