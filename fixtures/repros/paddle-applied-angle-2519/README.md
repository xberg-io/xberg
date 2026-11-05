# Paddle detected orientation and applied correction (#2519)

Public reference: [issue #2519](https://github.com/xberg-io/xberg/issues/2519),
[paddle-rotate-90.pdf](https://github.com/user-attachments/files/34433941/paddle-rotate-90.pdf).
The attachment is reference-only: it has no explicit redistribution license and its bytes are not committed.

The 72,288-byte PDF has SHA-256
`4cf1882b3fe5af4cf3683f3de3f43bacd183381d586728aa5bcdb7095d4d93d6`.
It has three pages, each with `/Rotate 90`, one 2000 × 1080 RGB image, and no native text.
Independent display rendering shows `ALPHA FIRST`, `BRAVO NEXT`, `OMEGA LAST` in that order:
stacked vertically on page 1, left to right on page 2, and in staggered rows on page 3.

The backend applies a 90° clockwise pixel correction for a detected orientation of 270°.
The corrected raster is upright, so its ordering residual against `/Rotate 90` must be zero.
Treating the detected 270° as the applied clockwise correction instead gives a 180° residual
and reverses all three arrangements.

`crates/xberg/src/paddle_ocr/backend/rotation_order_tests.rs` constructs independent asymmetric
pixel markers and the three source arrangements, invokes the production pixel rotation and
block ordering, and checks exact label order and unchanged box geometry. `/Rotate 0`, 180 and
270, low-confidence detections, and unsupported angles are controls. These tests do not run
Paddle inference or establish public PDF-route behavior.

For real qualification, verify the original attachment hash, use the issue's PP-OCRv6 small
Chinese configuration with `auto_rotate: true`, disable result caches, and compare both the
whole document and all three requested page outputs with the six ordered source words.
Retain an `auto_rotate: false` control and the actual orientation/model revision diagnostics.
