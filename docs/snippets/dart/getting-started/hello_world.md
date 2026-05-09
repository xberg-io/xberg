```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  print('Hello from kreuzberg!');
  final result = await KreuzbergBridge.extractFile('document.pdf', null);
  print(result.content);
}
```
