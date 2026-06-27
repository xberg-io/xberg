<!-- snippet:syntax-only -->

```dart title="Dart"
import 'dart:convert';

import 'package:xberg/xberg.dart';

Future<void> main() async {
  final schema = jsonEncode(<String, Object?>{
    'type': 'object',
    'properties': <String, Object?>{
      'title': <String, Object?>{'type': 'string'},
      'authors': <String, Object?>{
        'type': 'array',
        'items': <String, Object?>{'type': 'string'},
      },
      'date': <String, Object?>{'type': 'string'},
    },
    'required': <String>['title', 'authors', 'date'],
    'additionalProperties': false,
  });

  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    structuredExtraction: StructuredExtractionConfig(
      schema: schema,
      schemaName: 'paper_metadata',
      strict: true,
      llm: const LlmConfig(model: 'openai/gpt-4o-mini'),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  final result = await XbergBridge.extract('paper.pdf', null, config);
  final structured = result.structuredOutput;
  if (structured != null) {
    print(structured);
  }
}
```
