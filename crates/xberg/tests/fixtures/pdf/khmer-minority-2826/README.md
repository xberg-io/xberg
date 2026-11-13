# Latin-majority native page

`fragmented.pdf` is synthetic. It reuses the Khmer paint operators and embedded font from
`../khmer-cluster-2536/cluster-complete-mapped.pdf` and the English prose from
`../khmer-cluster-2536/english_page.py`. English words are split into one- and two-letter
tokens. The page contains no images and no private data. The original Khmer font license
is in `../khmer-cluster-2536/OFL.txt`.

`khmer-in-english-page.pdf` is the public reproducer from
[#2826](https://github.com/xberg-io/xberg/issues/2826). Run the issue's
`derive_khmer_in_english_page.py` against the same `cluster-complete-mapped.pdf`.
The resulting PDF has SHA-256
`74587828c84a1f5ca14093d47e8fe753519b5c920e354e545ea4e81bc8f8ec67`.
