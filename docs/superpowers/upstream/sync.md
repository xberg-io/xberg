# Selective upstream sync (xberg-io/xberg)

Upstream is `xberg-io/xberg` (remote `upstream`). `origin` is the fork
`jamon8888/xberg`. Our additions are fenced: Tier-1 overlay (upstream never
touches) and Tier-2 carry-patches (tracked in `carry-patches.tsv`).

## Steps
1. `git fetch upstream main`
2. `bash scripts/upstream-diff.sh` — lists the Tier-2 files upstream changed.
   That list is your entire manual merge surface. Tier-1 needs no action.
3. Sync core:
   - single fix: `git cherry-pick <sha>`
   - broader: `git merge upstream/main` (resolve only reported Tier-2 files)
4. Resolve each reported Tier-2 file (see `carry-patches.md` per-file notes).
5. `cargo check -p <affected-crate>` for each crate whose files changed;
   run the crate's targeted tests.
6. If a carry-patch was absorbed upstream, remove its row from
   `carry-patches.tsv` and its note from `carry-patches.md` in the same commit.

## Not covered here
Full `rc.24` catch-up is a separate, deliberate project. The fence makes it
affordable; do not do it as a side effect of a routine sync.

## Building the fork's process/rehydrate endpoints

The `/v1/process` + rehydrate API (fenced in `crates/xberg/src/api/rag/`) is
NOT enabled by the plain `api` feature — that stays upstream-clean by
design. To build a server/CLI that serves these fork-only endpoints, add
`--features process-api` alongside `api`:

```sh
cargo build -p xberg-cli --features api,process-api
```
