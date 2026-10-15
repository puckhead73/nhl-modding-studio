# Receipt 02 — Xbox 360 disc-image route (`p1.1-xbox360-iso-member-route`)

> **R2 correction (18 September, [OVERNIGHT-DEV-R2](../../validation/overnight-2026-09-18-review-and-r2-dispatch.md) F1/F2):** the range writer below reported "nothing had been written" for a range whose write had been attempted and could leave a partially written range that Restore refused (its header claim that Restore handles every state was false); the panel applied a plan recomputed after the review, with no project/revision binding, no serialization against other operations on the image and no running-emulator exclusion. All of that is corrected in receipt [08](08-r2-write-recovery-and-image-ownership.md) and awaits Codex review; **no live image write is authorized** until then. The measurements and plan-only preflights below stand.

Overnight developer lane, 18 September 2026. Claim: `local/coordination/overnight-2026-09-18/claims/p1.1-xbox360-iso-member-route.json`.
Status: **implementation complete / automated verified / user review pending.** No game or
emulator launched; **no real image was written** — every real-image check is read-only (opening,
listing, extracting members into temp folders, planning). Runtime source precedence (does Xenia
launch the `.iso` or the extracted folder beside it?) is not decided here; it is a card for
tomorrow and a research question (`r0`).

## Plain-language summary

Every Xbox 360 title on `X:\Games\Emulation\Roms\XBOX360` from NHL 10 to Legacy ships as a disc
image (`.iso`) with an extracted folder beside it. Studio could already open the folders; it had
no way to read or edit what is *inside* an image, which is what the emulator reads when it
launches the image. Tonight Studio gained that route end to end: it reads the image's file system
(a new `tdb-xiso` crate, verified member-for-member against the research lane's independent
listings of all seven images), finds the database and the dictionary inside — loose on NHL 10,
inside `cacheboot.big` / `cache.big` / `fe.big` + `boot.big` on the others — reads them out into a
staging folder, opens that folder as a normal project (edit, save, pending review all unchanged),
and can write the saved files back into the image as same-length range patches with a verified
backup per range and an idempotent restore. Every database and dictionary inside the seven images
is byte-identical to the committed Xbox 360 anchor of its title, and every one is stored
uncompressed, so a field edit or a dictionary re-point is exactly a write of that file's bytes at
one absolute offset of the image. Nothing else in the image moves. The UI has an "open the disc
image" entry on the Xbox folder step and a "Disc image" panel on Pending edits with an explicit
confirm, the plan (ranges and identities), the records and per-record restore.

## What was measured (observed tonight, read-only)

| Title | Layout | Database inside the image | Dictionary inside the image | = committed anchor? |
|---|---|---|---|---|
| NHL 10 | XGD2 | loose `db/nhlng.db` (sector 1761907, 1 998 232 B) | loose `fe/loc/nhl_eng_us.db` (sector 1763231, 667 996 B) | both yes |
| NHL 11 | XGD2 | `fe.big` #2 (348 MB EB archive) at +2 718 832, 2 739 736 B | **`boot.big`** #96 at +3 883 984, 811 364 B | both yes |
| NHL 12 | XGD2 | `cache.big` #34 at +11 401 504, 3 043 876 B | `cache.big` #3 at +1 857 312, 936 780 B | both yes |
| NHL 13 | XGD2 | `cache.big` #32 at +12 993 600, 2 695 068 B | `cache.big` #3 at +2 139 632, 1 153 044 B | both yes |
| NHL 14 | XGD2 | `cacheboot.big` #50 at +17 517 536, 2 820 656 B | `cacheboot.big` #3 at +1 898 912, 1 703 204 B | both yes |
| NHL 15 | XGD3 | `cacheboot.big` #50 at +17 763 936, 2 890 040 B | `cacheboot.big` #3 at +1 928 944, 1 729 660 B | both yes |
| NHL Legacy | XGD3 | `cacheboot.big` #50 at +18 177 856, 2 901 504 B | `cacheboot.big` #3 at +1 959 616, 1 759 044 B | both yes |

Stored length = decoded length on every row (raw, not `chunkzip`). Identities:
`local/runs/overnight-development-2026-09-18/xiso-locate-run{1,2}.txt`; anchor comparison in
this receipt's session log (`sha256sum anchors/xbox360/<t>/databases/{nhlng.db,nhl_eng_us.db}`).
NHL 07/08/09 are extracted folders (no image) and are not covered by this route.

Plan-only preflight on the **real** images (dictionary re-point per the parity matrix's test
label; staging in a temp folder; image never opened for writing; sizes and mtimes unchanged —
`xiso-plan-real-all.txt`, `xiso-plan-real-nhl14.txt`):

| Title | Key ← donor | Image range written by an apply | Prior → replacement |
|---|---|---|---|
| NHL 10 | `TXT_LEAGUE_SWISS_LEAGUE` ← `CUSTOM_TEAMS` | `[3876976640, 3877644636)` | `f1a9a1dd…` → `f97aced5…` |
| NHL 11 | `PROSPECT_TEAMS` ← `TXT_ROW_LEAGUE` | `[3896064976, 3896876340)` (in `boot.big`) | `82520ab3…` → `4b01f1c1…` |
| NHL 12 | `TXT_LEGEND_TEAMS` ← `TXT_ROW_LEAGUE` | `[499701536, 500638316)` | `354a1905…` → `71fcec76…` |
| NHL 13 | same | `[476474864, 477627908)` | `b93fa83f…` → `bd7fa83c…` |
| NHL 14 | same | `[3845237152, 3846940356)` | `e5aff828…` → `3af0b8a5…` |
| NHL 15 | same | `[4317081328, 4318810988)` | `86167614…` → `b94c951c…` |
| NHL Legacy | `TXT_LEAGUE_NORWAY` ← `TXT_ROW_LEAGUE` | `[4316217024, 4317976068)` | `f00926ce…` → **`6e80ecf3…`** (= the batch's Legacy Xbox 360 loose dictionary candidate) |

## Implementation

- **`crates/tdb-xiso`** (new, dependency-free): XDVDFS reader — probes XSF/XGD3/XGD2/XGD1 bases
  for the `MICROSOFT*XBOX*MEDIA` volume sector, walks every directory table as the binary tree it
  is (cycle/bounds/name checks, sector-padding aware), exposes members, case-insensitive lookup,
  absolute member ranges and a `Read + Seek` window per member. Never writes. A synthetic image
  builder (`synth`) lays out nested directories for tests.
- **`studio-domain::xiso_source`**: locate `db/nhlng.db` and `fe/loc/nhl_eng_us.db` in an image —
  loose first, then every root `.big`: EB archives by header + tables only (no payload read, any
  size), `BIGF` family ≤ 96 MB via the whole-buffer reader; read a located file (chunkzip
  inflated, chunklzx refused); plan a same-length write-back.
- **`studio-domain::iso_patch`**: hash-bound range patches — plan (range from the located entry,
  prior identity must match the image now, replacement must be the exact length), apply (backup
  `prior.bin` + `record.json` written and flushed *before* the range write, read-back verified,
  record directory must not pre-exist), restore (idempotent: `AlreadyPrior` / `Restored`; unknown
  bytes refused with both identities named, backup kept), status.
- **`studio-domain::iso_staging`**: stage (loose-install layout + `nhlng-meta.xml` + `ISO-SOURCE.json`;
  reuse for the same image, refuse for another), plan/apply (changed files only, coordinated —
  a later failure restores the earlier patches), restore (newest first), records.
- **`studio-api`**: `Registry::{iso_inspect, iso_stage, iso_open, iso_plan, iso_apply, iso_restore, iso_records}`
  (`registry/iso.rs`), commands of the same names (`iso_open` resolves the anchor's
  `nhlng-meta.xml` for the title when none is given — in-repo during development, a packaged
  copy when anchors ship as a resource), six new DTO roots, bindings exported and in sync.
- **App**: `IsoOpenPanel` on the Xbox folder step (inspect → stage and open; generation-bound
  completions), `DiscImagePanel` on Pending edits (plan table, "Write to disc image…" with explicit
  confirm and refused while dirty, apply/restore outcomes, records with per-record Restore),
  `ProjectProvider.iso` carries the staging binding.

Design limits stated in code: no atomicity against a crash mid-range-write (the range is then
neither identity; restore refuses until inspected; the backup is intact); no member relocation or
resizing (length-changing edits are refused at plan); no claim about what the game does with the
bytes.

## Checks (developer evidence)

| Check | Result |
|---|---|
| `cargo test -p tdb-xiso` | 4 passed, 1 ignored (real-image) |
| `cargo test -p tdb-xiso -- --ignored` with `NHL_STUDIO_XISO`/`_LISTING` on each of the 7 images vs research's `data/iso/*-xdvdfs.json` | 7/7 ok — every member's path/sector/size/attr, base and root sector match |
| `cargo test -p studio-domain --lib xiso_source iso_patch iso_staging` | 8 passed, 1 ignored |
| `studio-domain xiso_source::real_image…` on the 7 images | 7/7 ok — DB parses (CRC ok), dictionary verifies |
| `cargo test -p studio-api --test iso_route` | 1 passed (full registry flow on a synthetic packed image with the Legacy anchors), 1 ignored |
| `iso_route::real_image_plan_only_never_writes` on the 7 images | 7/7 ok, images unchanged (size, mtime) |
| `export-bindings --check`, `tsc --noEmit` | in sync / clean |
| Broader checkpoint (`cargo test -p studio-domain -p studio-api -p tdb-xiso`, vitest) | see `local/runs/overnight-development-2026-09-18/checkpoint2-*.txt` |

Not done: a desktop run of the new panels (jsdom tests for `DiscImagePanel`/`IsoOpenPanel` are
not written yet — queued), any live apply, any decision on Xenia's launch source.

## Changed paths

New: `crates/tdb-xiso/{Cargo.toml,src/lib.rs}`, `crates/studio-domain/src/{xiso_source.rs,iso_patch.rs,iso_staging.rs}`,
`crates/studio-api/src/registry/iso.rs`, `crates/studio-api/tests/iso_route.rs`,
`app/src/components/{IsoOpenPanel.tsx,DiscImagePanel.tsx}`, 11 `app/src/bindings/Iso*.ts`.
Modified: `Cargo.toml` (workspace member), `crates/studio-domain/{Cargo.toml,src/lib.rs}`,
`crates/studio-api/{Cargo.toml,src/{dto.rs,bindings.rs,commands.rs,lib.rs,registry.rs}}`,
`app/src/lib/{client.ts,project.tsx}`, `app/src/pages/{OpenProject.tsx,PendingEdits.tsx}`.
Scratch (owned, small): `local/runs/overnight-development-2026-09-18/xiso-*.txt`.

## For tomorrow (user-gated)

1. Research's `r0` handoff: which source each Xbox title launches from (image vs folder). If the
   image: the route above is the placement path — open the image in Studio, hold the rename,
   save, "Write to disc image" (records under `<staging>/records/0001/`), boot, observe, Restore.
2. Xenia may cache nothing about an image's contents, but it may keep the image open while
   running: apply and restore only with the emulator closed (the panel does not check for a
   running emulator — the deploy helper's `EmuRunning` gate is the model if that is wanted).
3. Cleanup after each experiment: the staging folder and its `records/` are disposable once the
   restore verified `prior`.
