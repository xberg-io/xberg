---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Reuse verified ONNX Runtime optimised graphs across cold starts

## Context and Problem Statement

Every process that builds a layout or inference session runs ONNX Runtime's
`GraphOptimizationLevel::All` transformers over the model again, including the
169 MB RT-DETR detector, and throws the optimised graph away. ONNX Runtime can
serialise that graph (`SetOptimizedModelFilePath`) and load it later with
optimisation disabled, which removes the work from every cold start after the
first. The saved graph is a new on-disk artifact that is parsed and executed
in place of the model. Models are trusted only because each process hashes
them against a SHA-256 pinned in the source before loading them; a saved
graph must not become a way to execute bytes that never passed that check.

## Decision Drivers

* Cold-start latency of layout extraction, which is dominated by session
  construction for single-document CLI and serverless runs.
* No weakening of the pinned-digest guarantee: what runs must derive from
  bytes this process verified, and must be what was saved.
* `All`-level graphs are valid only for the ONNX Runtime build and CPU that
  produced them (NCHWc block size, MLAS kernels), so reuse must be exact.
* Concurrent processes share one cache directory and may crash mid-write.
* Every failure must degrade to today's behaviour, never to an error.

## Considered Options

* Save the graph in the xberg cache, keyed and verified as below (chosen).
* No cache: optimise online in every process (status quo).
* Ship pre-optimised graphs built at release time.
* Sign saved graphs with a local key.

## Decision Outcome

Chosen option: "Save the graph in the xberg cache, keyed and verified", because
it removes the repeated optimisation on every later cold start while every
graph it executes is derived from verified bytes and checked against its own
digest on each load.

Scope: CPU-only sessions built from a model file, through the inference seam
(`inference::ort_backend`) and the bare layout sessions (`layout::session`).
Sessions with CoreML, CUDA or TensorRT never use it, because compiled
execution-provider nodes cannot be serialised and provider partitioning is not
in the key. On macOS the default `Auto` provider registers CoreML, so the cache
applies there only with `provider = "cpu"`. Non-Unix platforms do not use it:
the ownership checks below rely on Unix ownership and modes.

Rules:

* **Provenance.** A graph is saved and reused only for a source file that
  `model_download::verify_sha256` hashed and matched in the same process. The
  record of that match lives in memory, keyed by device, inode, size and
  nanosecond mtime and ctime, and is consulted only while those are
  unchanged. It is never written to disk, so the model is still hashed on
  every process start, as before. If the model's identity changes while ONNX
  Runtime reads it to produce the graph, the graph is not published.
* **Key.** SHA-256 over a cache schema version, the ONNX Runtime build string
  (`ort::info()`), target architecture and OS, the CPU features that select
  MLAS kernels and the NCHWc block size, the optimisation level, the
  execution provider set (CPU only, default options), a recipe string naming
  the session options of the building route, and the source model's verified
  SHA-256. Any difference is a different key, so a mismatch rebuilds and
  never reuses. Publishing a key keeps the four newest keys for the same
  model path and removes older ones, so hosts with different CPUs or ONNX
  Runtime builds that share a cache directory do not delete each other's
  entries on every save.
* **Integrity.** A sidecar records the schema, key, size and SHA-256 of the
  graph; the graph file is named by its digest. Before every load the sidecar
  and graph are opened without following symlinks, the size must match and be
  at most 2 GiB (the protobuf limit), and the full graph is re-hashed. A
  mismatch or load failure deletes the entry and rebuilds online from the
  verified model.
* **Filesystem hygiene.** The directory is `<cache base>/ort-optimized`,
  resolved like every other cache (`XBERG_CACHE_DIR` or the platform cache),
  created with mode 0700 (an existing owner-only-writable one is narrowed to
  0700); files are created with `O_CREAT|O_EXCL|O_NOFOLLOW` and mode 0600. If
  the directory is a symlink, not owned by the effective user, or writable by
  group or others, or an entry is not a regular file with the same
  properties, the cache is not used for that build and a warning is logged.
* **Atomic publish.** ONNX Runtime writes into a uniquely named staging file
  in the same directory; it is fsynced, hashed, renamed to its digest name,
  and only then is the sidecar written to its own staging file, fsynced and
  renamed into place, followed by a directory fsync. A reader sees either no
  sidecar or a complete graph. Racing writers each publish a complete,
  digest-named graph and the last sidecar rename wins; every racer keeps the
  session it built.
* **Bounds.** At most one load and one save per session build; staging files
  and unreferenced graphs older than one hour are swept on publish; at most 16
  entries are kept. The sidecar read is capped at 4 KiB, the graph read at
  its recorded size.
* **Default and switches.** On by default, because the protections above make
  a saved graph no easier to subvert than the model cache beside it for
  anyone except the cache owner (see the threat model), and the benefit
  applies to every user without configuration. `XBERG_ORT_OPTIMIZED_CACHE=0`
  (or `false`, `off`, `no`) disables it. Pointing `XBERG_CACHE_DIR` at a
  directory the process cannot write disables it implicitly. It is not tied to
  `use_cache` / `XBERG_CACHE_ENABLED`, which govern extraction results, not
  model artifacts.

### Threat model

* **Cache owner (same effective user, or root).** Can write both the graph and
  its sidecar, so the digest proves nothing against them; they can make xberg
  execute an arbitrary ONNX graph, producing wrong detections or reaching
  any parser or kernel defect in ONNX Runtime. Not defended. Before this
  change the same principal could not substitute a model, because the pinned
  hash covers the bytes that run, so this is a narrowing; it is accepted
  because that principal can already replace the xberg binary or library,
  set `ORT_DYLIB_PATH` or `LD_PRELOAD`, or edit the user's shell profile.
* **Other local users.** Defended: the directory and entries must be owned by
  the effective user and not group- or world-writable, symlinks are not
  followed, and files are created exclusively. A shared `XBERG_CACHE_DIR`
  (for example under `/tmp`) is refused rather than trusted.
* **Corruption and mix-ups.** Defended: torn writes, crashes, disk errors,
  concurrent writers, a different model, ONNX Runtime build, CPU or session
  recipe all fail the digest or key check and fall back to the verified
  model.
* **A model replaced after verification.** Defended for the cache: the graph is
  not published. The in-process window between hashing a model and ONNX
  Runtime opening it is unchanged from before this decision.
* **Not covered.** A principal who controls an ancestor directory of the cache
  root can swap the root between the check and the open; the checks cover
  the root and its entries only. The default root is under the user's own
  cache directory; an `XBERG_CACHE_DIR` in shared space must sit below a
  directory other users cannot rename. The window between re-hashing a graph and
  ONNX Runtime opening it needs write access to the 0700 directory, which
  only the cache owner has. Network filesystems with weak rename or fsync
  semantics may lose an entry, which only costs a rebuild.

### Consequences

* Good, because every cold start after the first skips graph optimisation for
  CPU sessions.
* Good, because the cache cannot execute bytes that were not derived from a
  model verified in the same process, and detects any change to a saved graph.
* Good, because every failure, including a refused directory, degrades to the
  previous online build.
* Bad, because each load re-hashes the saved graph (about 70-80 ms for a
  169 MB graph with hardware SHA-256) on top of the existing source-model hash.
* Bad, because the cache owner can substitute a graph, which the pinned model
  hash alone prevented.
* Bad, because a saved graph roughly doubles the on-disk footprint of each
  cached CPU model, bounded by 16 entries.

### Confirmation

`inference::ort_optimized` unit tests cover the key inputs, owner-only modes,
refusal of group/world-writable and symlinked directories and entries,
tampered, truncated, oversized, malformed and foreign sidecars, discard,
eviction, sweeping, the entry cap and racing publishers. ORT-backed tests on a
model built in memory prove the save-then-load cycle, a forged entry that
fails to load being replaced, a model changed after verification not being
saved, and bit-identical outputs. `saved_layout_graphs_reproduce_online_outputs_bit_for_bit`
runs RT-DETR through the inference-seam route and TATR through the layout
route with their production session builders and requires bit-identical
outputs; it fails rather than skips under `XBERG_REQUIRE_TRACT_PARITY`, and the
CI parity step runs it. `model_download` tests cover the in-memory verification
record. Cold-start time with `XBERG_ORT_OPTIMIZED_CACHE=0` and `1` is measured
before acceptance.

## Pros and Cons of the Options

### Save the graph in the xberg cache, keyed and verified

* Good, because it adapts to the host's ONNX Runtime and CPU automatically.
* Good, because it needs no build or release change.
* Bad, because it adds an executed artifact whose integrity rests on directory
  ownership against the cache owner.

### No cache

* Good, because nothing new is executed from disk.
* Bad, because every process repeats the optimisation, the largest fixed cost
  of a layout cold start.

### Ship pre-optimised graphs built at release time

* Good, because graphs could be pinned by digest like models.
* Bad, because `All`-level graphs depend on the host CPU's vector width and the
  exact ONNX Runtime build, including `ort-dynamic` builds that load a system
  library, so one artifact per CPU class and runtime would be needed and a
  mismatch would still fall back online.
* Bad, because it adds hundreds of megabytes of artifacts per release, and an
  `Extended`-level portable graph forgoes the NCHWc transforms.

### Sign saved graphs with a local key

* Good, because a signature binds graph and metadata together.
* Bad, because the key would live where the cache owner can read it, so it
  adds nothing against the only principal the ownership checks do not stop.
* Bad, because a key held in an OS keychain is platform-specific and still
  readable by that principal.

## More Information

* [MADR](https://adr.github.io/madr/)
* ONNX Runtime graph optimisations, "Online/Offline mode": an `All`-level
  saved model is only valid on hardware matching the producing environment.
* Code: `crates/xberg/src/inference/ort_optimized.rs`,
  `crates/xberg/src/model_download.rs` (`pinned_file`).
