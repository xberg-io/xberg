<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: the Dart binding does not expose `registerPostProcessor`. A
  // Dart implementation of the `PostProcessor` trait that maintains
  // mutable state (counters, caches) across invocations cannot be
  // plugged into the global post-processor registry from Dart.
  //
  // Implement the stateful plugin in Rust as `Plugin + PostProcessor`
  // (using `AtomicUsize` / `Mutex` for shared state) and register it via
  // `register_post_processor` in a Rust shim crate that links kreuzberg
  // before the Dart host process loads the dynamic library.
}
```
