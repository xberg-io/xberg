# .NET transport memory replay

After integrating the workflow into `main`, dispatch it explicitly against the current full main SHA.

```sh
sha=$(git ls-remote https://github.com/xberg-io/xberg.git refs/heads/main | cut -f1)
gh workflow run ci-dotnet-transport-memory.yaml --repo xberg-io/xberg --ref main \
  -f verify-3152=true -f revision="$sha"
```

If main advances before dispatch, both jobs stop before toolchain setup. The workflow builds an uncached,
locked release FFI on each x64 runner and compiles the managed probe against the same Git revision.
The evidence artifacts contain native and staged dependency digests, hashes of loaded modules seen after
extraction, native and managed build logs, SDK and runner provenance, and the case and summary data.
Each worker's module inventory records whether it mapped core ONNX Runtime. A mapping must match the
staged core library path and digest, including when another copy has identical bytes.
The CI artifact contains receipts and logs only.

Each job completes 27 cases across three fresh processes per fixture/mode pair. Synthetic 40 and 90 MiB PDFs
and a 90 MiB DOCX are pinned by digest. They contain inert padding and a fixed text marker. OCR and caching are
disabled, and the byte array is loaded before the baseline. The report checks exact content and MIME parity.
It compares the median paired byte-minus-path private memory growth and the maximum paired managed allocation
against the replay budgets. The factory allowance includes its intentional extra copy.

Windows uses process-lifetime peak private commit growth after warm-up. The report includes ready current and
peak counters so warm-up masking is visible. Linux uses sampled private resident memory from
`/proc/PID/smaps_rollup`. The supervisor takes a final sample while the worker waits at completion and includes it in the peak.
The report includes the sampling cadence. Spikes shorter than a sampling gap can still escape detection.
Assess each platform separately. The synthetic fixtures exercise large input transport with a small result.
