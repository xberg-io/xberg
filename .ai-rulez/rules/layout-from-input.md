---
type: Decision
title: Layout From Input
description: "Derive layout decisions from the geometry and raster properties of the input."
paths:
  - "crates/xberg/src/extractors/image.rs"
  - "crates/xberg/src/extractors/pdf/**"
  - "crates/xberg/src/layout/**"
  - "crates/xberg/src/ocr/**"
  - "crates/xberg/src/pdf/**"
  - "crates/xberg-native-pdf/src/extractors/**"
  - "crates/xberg-native-pdf/src/rendering/**"
  - "crates/xberg-native-pdf/src/structure/**"
x-ai-rulez:
  kind: rule
  id: layout-from-input
  metadata:
    priority: high
---

- Derive layout decisions from the input's orientation, frame size, coordinate origin, and colour model. Transform evidence into a common frame before comparing positions or bounds.
- When a page needs an inferred threshold, use robust statistics over that page's relevant measurements, such as a median of comparable line heights. Apply a fallback only when the input cannot support the inference.
- Verify the decision with an input that differs from the default geometry and assert the resulting position, orientation, or reading order against the source.
