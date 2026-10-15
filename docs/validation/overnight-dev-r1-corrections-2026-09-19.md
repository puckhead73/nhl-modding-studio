# OVERNIGHT-DEV-R1 — corrections to the 19 September developer campaign

Developer receipt for the bounded correction package in
[overnight-development-2026-09-19-review-r1.md](overnight-development-2026-09-19-review-r1.md).
**Developer-verified; stopped for Codex review.** No launches, live writes, staging, commits or pushes;
private fixtures read-only; the pre-existing dirty tree preserved. Baseline `8d60a6d`, branch
`feat/phase5-asset-tooling`.

## Plain-language summary

Two defects in the new PS4 "replace a logo inside its archive" route are fixed. First, the dialog that
picks the image, previews the rewrite and applies it did not know which project it belonged to: a picker,
preview or apply that came back after switching projects (or members) could show a stale plan, fire the
wrong callback, or leave the project's global "unsaved changes" state untouched after the first texture
edit. It now binds every step to the project, install root and the pick it was started for, ignores late
answers, adopts the returned project summary only for the project it belongs to, never sends Apply twice,
and says plainly that closing the dialog would not cancel an apply already sent. Second, replacing the
same archive member twice and then pressing undo threw away *both* replacements; undo now reverts only
the latest apply, so the earlier replacement is pending again and is what the save installs. Finally the
campaign's coverage wording was narrowed to what its tests actually show: codec round trips are not
end-to-end UI flows, a per-title fixture set is not every installed build, a file-name scan is not
"not applicable", and the archive route is proven on one real NHL 21 member inside a synthetic archive.

## 1. Findings and what changed

### F1 — archive replacement UI ownership and summary refresh (P1)

`app/src/components/ArchiveMemberReplace.tsx` rewritten on the pattern of `StoredDefaultEditor` /
`LeagueScheduleCard`:

- **Context and generation.** `ctxRef` = `{projectId, installRoot}` read after every `await`; `genRef` is
  the pick generation, bumped by a new pick, Cancel, a project/install-root change (effect, which also
  clears picked/preview/result/error) and unmount. `live(gen, project, root)` gates every state write.
- **Picker.** `pick()` captures project/root/generation *before* `openDialog`; a path returned after a
  context switch, a newer pick or a cancel opens nothing and dispatches nothing. Refused while an apply
  runs (the plan under the apply must not be replaced).
- **Preview.** The command is dispatched with the captured `projectId`/`installRoot` (never the current
  render's); the result is kept only if still live. The dialog exposes `data-ticket` so tests can assert
  which plan Apply would send.
- **Apply.** `applyingRef` prevents a second dispatch; the ticket is the displayed plan's. On success:
  `invalidateQueries()` always (the server did apply it); `update(r.summary)` only when the owning project
  is still open (`ctxRef.current.projectId === forProject`, mounted); result banner + `onApplied` only when
  live. On refusal (foreign / consumed / stale) the plan leaves the dialog and Apply stays disabled until a
  new pick previews again.
- **Cancel / close during apply.** Not offered: the Cancel button and the backdrop are disabled while
  `applying`, and the dialog states "the request has been sent to the project and completes there;
  closing this dialog would not cancel it". A project switch during apply still leaves the mutation on
  the server; queries are refreshed and the summary adopted only if the project matches.
- The pending query (`["pending", projectId, undo_depth, redo_depth, dirty]`) refetches through the
  summary update and the invalidation, so Pending Edits shows the replacement without a manual reload.

Backend ticket protection is unchanged (foreign / consumed / stale still refuse; asserted again in
`asset_member_undo.rs`).

### F2 — undo of a superseded texture replacement (P2)

`crates/studio-domain/src/workspace.rs`: `asset_members` stays the **effective** list (one plan per
`(archive, member)`, first-seen order — what the copied-target save installs). A new operation log
`asset_member_ops: Vec<AssetMemberOp { position, superseded: Option<AssetMemberPlan> }>` records every
accepted apply. Apply on an already-pending member replaces it **in place** (position kept) and remembers
the superseded plan; a fresh member appends. `undo_asset_member_edit` pops the newest op: it puts the
superseded plan back at its position, or truncates the fresh insert (always the last element at that
point because later ops were undone first). `asset_member_undo_depth()` exposes the log depth. Reload and
the post-commit rebind clear both structures. The revision identity is still computed over the effective
list, so A → B → undo yields exactly the revision after A. No redo is offered (none existed; not
requested). Pending Edits button: "Undo the last texture replacement" with a title explaining the return
to the earlier replacement.

### F3 — coverage claims reconciled

See §3 for the cell-by-cell narrowings applied to
[phase-1-coverage-matrix-2026-09-18.md](phase-1-coverage-matrix-2026-09-18.md); receipts 06, 07 and 08 and
MORNING-REVIEW carry correction notes (historical text left as written; receipt 07 §5's closing sentence
struck). Nothing from the research review's refuted universal RefPack relocation claim was imported; the
P1-F row now carries the opposite constraint (fit decided per actual encoded length against the measured
slot and verified capacity; unexplained gap bytes preserved; the four declared-size-mismatched archives not
accepted from counts).

## 2. Changed paths (SHA-256 prefixes after the change)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/studio-domain/src/workspace.rs` | `7235787f5d3f7eff…` | F2: `AssetMemberOp`, `asset_member_ops`, in-place coalescing in `asset_member_apply_review`, history-based `undo_asset_member_edit`, `asset_member_undo_depth`, cleared on reload / rebind |
| `crates/studio-domain/tests/asset_member_undo.rs` | `c94258f21fd7d650…` | **new** — 3 cases: A→B→undo restores A (same revision, still dirty; undo A → clean; nothing left); five interleaved applies over three members unwind in apply order with positions and revisions restored; every undo discards held reviews (stale apply refuses without mutation), foreign ticket refused, reload clears the log |
| `crates/studio-api/tests/asset_member_route.rs` | `1567167e6dd5ef18…` | +1 case `undoing_a_superseding_replacement_saves_the_earlier_one`: A→B→undo leaves A pending (summary dirty, pending listing = A), the copied-target save installs A's pixels (not B's), the non-target shared row keeps its bytes, the log is closed by the commit, restore byte-exact |
| `app/src/components/ArchiveMemberReplace.tsx` | `922e60c5d80c8586…` | F1 rewrite (above) |
| `app/src/components/ArchiveMemberReplace.test.tsx` | `9eb303cafabd64cb…` | **new** — 8 jsdom cases with deferred promises: exact preview/apply arguments + summary adoption + invalidation + callback; picker after project change and after a member switch; preview after context switch / cancel / newer pick (ticket asserted); apply after project switch and after unmount (no summary, no callback, no result; queries still refreshed); install-root-only change (summary adopted, nothing shown); triple-click Apply dispatches once, Cancel disabled and inert, new pick refused, completion lands; refused apply drops the plan; server refusal of the preview |
| `app/src/components/LeagueImageryCard.test.tsx` | `8b4e3228b89835a0…` | type error in the D8 case fixed (`tsc --noEmit` had failed on it); the mocked apply now returns a `project_id`/`dirty` summary |
| `app/src/pages/PendingEdits.tsx` | `3daec03d2b25c19a…` | "Undo the last texture replacement" (+ doc comment on the effective list vs the log) |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `10e1547ed895a575…` | F3 narrowings (§3) |
| `docs/receipts/overnight-development-2026-09-19/{06,07,08}*.md`, `MORNING-REVIEW.md`, `STATUS.md` | — | correction notes at the top; receipt 07 §5 sentence withdrawn; card wording; STATUS R1 entry |

No DTO changed; bindings not regenerated (`app/src/bindings` untouched).

## 3. Matrix cells narrowed (F3)

| Cell | Was | Now | Why |
|---|---|---|---|
| 1.4 replace loose package, same dimensions | one row, impl+tested on every family ("every package") | split: **codec** impl+tested (same-pixel round trip on the index-0 package per family of a fixture set from one extracted build per title; different pixels on synthetic RW4ps3/DXT5 only) / **API route + UI flow** impl, test missing on every family | `import_texture` is tested on a GNF portrait only; the league cards' jsdom cases assert the preview call, never an apply on a logo container |
| 1.4 archive placement, PS4 16–21 | impl+tested (D8) across 16–21 | impl+tested on **NHL 21** (real member bytes in a synthetic EB archive; R1 F1/F2 cases added); **NHL 16–20 impl, test missing** | no test opens a real `inxp_10*.big` of any title |
| 1.4 archive placement, PS4 15 | impl+tested (synthetic) | unchanged level; "no real NHL 15 archive opened" stated | precision |
| 1.4 non-logo imagery | one row impl+tested; tiles/banners n/a (registries) | split: **listing** impl+tested / **preview–replace** impl, test missing on every family; tiles/banners **research unknown** | no non-logo file decoded on any family (receipt 06 §5); only the named registries were searched |
| 1.5 separately named league bank | n/a (evidence) | **research unknown** — researched absence by member-name scan; league-keyed records inside generic banks disprove the blanket reading | a name scan cannot establish n/a; audio identifiers do not establish sample replacement (row 3 note added) |
| P1-E / P1-F / P1-H package rows | — | remaining items restated; P1-F carries the research-review constraints and the R1 correction | reconciliation with `overnight-research-2026-09-19-review.md` |

Cells **not** changed and why: 1.4 thumbnail preview (already said "index-0 package … fixture set");
1.1–1.3 (outside this review's findings; still awaiting their own independent review — §5).

## 4. Checks executed

| Command | Result |
|---|---|
| `cargo test -p studio-domain --test asset_member_undo` | **3 passed** (would fail on the pre-R1 code: the first case asserts A pending after undoing B) |
| `cargo test -p studio-domain --test copied_target --test txn_coordination --test settings_csv_edit --test pending --test workspace` | 6 / 3 / 6 (+1 ignored) / 3 / 10 passed |
| `cargo test -p studio-api --test asset_member_route` | **3 passed**, 1 ignored |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --ignored` | **1 passed** (real NHL 21 `l0.gnf`: private copy, `_fr` twin unchanged, native framing, BC7 MAD < 6, restore byte-exact) |
| `cargo test -p studio-api --test copied_target --test dictionary_growth --test settings_csv_route` | 15 / 3 / 2 passed |
| `cargo test -p tdb-archive-big --lib stage::tests` | 29 passed |
| `cd app && npx tsc --noEmit -p .` | clean (after the `LeagueImageryCard.test.tsx` fix; failed before it) |
| `cd app && npx vitest run` | **16 files, 120 passed** (was 15 / 112) |

Not run: full workspace suite; `zz_diag`; clippy/fmt (recorded baseline defects); bindings check (no DTO
change); desktop; any game. Pre-existing compiler warnings remain.

## 5. Areas the first review did not cover (for the next review)

The R1 review prioritized the new write routes (D8 archive members, D4 CSV route, D5 growth). Still
awaiting independent review, with the receipts that describe them:

- D1 scoped settings decoding (`tdb-defs` settings_sites; receipt 01) — the four sites, `qbPU.CkjP`
  unchanged, NVBd structural-only.
- D2 read-only catalogue (receipt 02) — row identity, duplicate/missing targets, UI binding.
- D3 NHL 15 stored-default editor (receipt 03) — single-cell bit/CRC proof, refusals, copied-target restore.
- D4 native chunkzip encoder and the paired NHL 18–21 edit (receipt 04) — byte-faithful claims (53/53,
  2/2), the real NHL 16 `inxp_00.big` round trip.
- D5 LOC-W2 locale coverage sweep and the LOC-W3 preserving serializer (receipt 05) — class-A order,
  collision multisets, sharing scope, `needs-rebuild` refusals.
- D6 codecs (receipt 06) — `RW4ps3` reader/writer, the two `tdb-rx2::texlib` fixes (macro-tile DXT read,
  tiled ARGB write) which affect every consumer of Xbox texlibs, not only logos; the uncompressed-DDS
  writer; whole-file RefPack re-wrap.
- D7 test-gap closures (receipt 07) — the local-bits sweep and the NHL 07 inferred index.
- The remaining desktop gates are user cards (MORNING-REVIEW), unchanged.

## 6. Omissions and limits

- No redo for texture replacements (none existed; not requested). Undo depth is not surfaced in a DTO;
  the pending card shows the effective list and one undo button.
- The UI does not offer to hide the dialog during an apply; a stuck apply keeps it open until the
  command settles (the command has no client-side timeout, as elsewhere in the app).
- The F3 narrowing is a wording reconciliation; it adds no tests for the cells now marked "test missing"
  (loose-route API/UI on logo containers, non-logo files, NHL 16–20 real archives).
- The pre-existing `tsc` failure in `LeagueImageryCard.test.tsx` is fixed here because it belongs to the
  D8 route under correction; no other pre-existing defect was touched.

## 7. Cleanup

- Scratchpad (outside the repo): `r1_matrix.py`, `r1_receipts.py` — disposable.
- Test temp dirs `nhlms-r1f2-*` and `nhlms-d8-api-*` are removed by their `Drop` guards (verified absent).
  Older `nhlms-*` directories in `%TEMP%` predate this package and were left alone.
- No background processes; nothing staged (`git diff --cached` empty).

## 8. Runnable commands

```sh
cargo test -p studio-domain --test asset_member_undo
cargo test -p studio-api --test asset_member_route
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --ignored
cd app && npx tsc --noEmit -p . && npx vitest run src/components/ArchiveMemberReplace.test.tsx src/components/LeagueImageryCard.test.tsx
```
