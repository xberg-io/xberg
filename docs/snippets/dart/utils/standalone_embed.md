```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  const config = EmbeddingConfig(
    model: EmbeddingModelType.preset(name: 'balanced'),
    normalize: true,
    batchSize: 32,
    showDownloadProgress: false,
  );

  final texts = <String>['Hello, world!', 'Kreuzberg is fast'];
  final embeddings = await KreuzbergBridge.embedTexts(texts, config);

  print('Vectors: ${embeddings.length}');
  print('Dimensions: ${embeddings.first.length}');
}
```
