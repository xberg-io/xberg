```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  print('Hello from xberg!');
  final result = await XbergBridge.extract('document.pdf', null);
  print(result.content);
}
```
