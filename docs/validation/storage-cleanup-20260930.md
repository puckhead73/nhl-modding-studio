# Storage cleanup — 30 September 2026

User authorized cleanup following the read-only storage audit. Removed 147.4 GiB of logical build-output data; the repository fell from approximately 165.7 GiB to 18.3 GiB. E: reports approximately 132 GiB free after cleanup. Logical file lengths and physical free space differ.

Exact removals and byte/file counts: `storage-cleanup-20260930.json`. Removed only:

- `E:\Repositories\nhl-modding-studio\target`
- `E:\Repositories\nhl-modding-studio\local\runs\codex\as6-review-20260916\target`
- `E:\Repositories\nhl-modding-studio\local\runs\codex\as7-review-20260916\target`

Before deletion, verified exact resolved paths within the repository, scanned for reparse points, and checked for cargo/rustc/link/Studio processes. None were running. Used native PowerShell literal-path removal. Each successful removal was recorded immediately. No source, original game data, anchors, corpus, Git history, external bundles or recovery backups were removed.

## Retained after audit

`local/runs/league-parity-batch` remains approximately 9.72 GiB. The completion record explicitly retains candidates for manual validation, copied roots as secondary rollback sources, source members, manifests and deployment/restore helpers. Its backups are not assumed disposable merely because old transactions were reported restored. Releasing these artifacts requires reconciling the outstanding runtime dependencies and source/recovery identities.

The two September 16 reviews retain source snapshots, baseline/close-check tar files and test logs (approximately 2.84 GiB combined after their build caches were removed). Their closeouts refer to committed AS6/AS7 work, but this audit did not establish that every archived baseline/candidate byte is recoverable from those commits. Build caches were safely regenerable; historical snapshot deletion was not justified by that fact alone.

A known junction under `local/reviews/s2b-junction-probe/copy/escape` was skipped by the size audit and untouched. No broad cleanup of `local` occurred.

## Growth prevention

Added `[profile.dev]` and `[profile.test]` with `debug = 0` and `incremental = false` to Cargo.toml. Existing workspace changes were preserved; release remains `opt-level = 3`. This avoids routine full PDB generation and incremental caches, at the cost of slower rebuilds and reduced source-level debugging. A deliberate debugging session can set `CARGO_PROFILE_DEV_DEBUG=2` / `CARGO_PROFILE_TEST_DEBUG=2`; remove the override afterward. Do not alternate profiles unnecessarily, which creates more artifact variants.

At development-session start, check free space. If the main target exceeds 40 GiB or E: free space falls below 20 GiB, inventory build output and coordinate cleanup at a build/test idle checkpoint. These are maintenance triggers, not permission to delete automatically while another session may be using it. Avoid parallel duplicate target directories unless isolation requires them. Retire owned package build directories at closeout; keep receipts and necessary reproducible inputs. Never apply this policy to arbitrary `local` content, backups or external review bundles.

Validation: Python TOML parsing asserted both profiles and the unchanged release profile; `cargo metadata --no-deps --format-version 1` succeeded. No compilation was run merely to recreate the cache, and no app/game was launched. Nothing staged or committed.

## Addendum — 12 October 2026

With E: at 3.6 GB free, the user authorized removing two build-output folders they confirmed are no longer needed: `target\overnight-20261003` (copied per-title game trees and review packages from the 3–4 October overnight QA run, ~111.4 GiB logical) and `target\internal-candidates` (~23.7 GiB). Both were checked for reparse points (none) and running cargo/rustc/link/Studio processes (none), then removed with literal-path deletion. E: free space afterwards: 139.2 GB. Nothing else under `target` or `local` was touched.
