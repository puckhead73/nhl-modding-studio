# OVERNIGHT-DEV-20260920-R1 — corrections to the 20 September campaign

Package record and receipt. Dispatch: [overnight-development-2026-09-20-review-r1.md](overnight-development-2026-09-20-review-r1.md).
Developer: Claude. **Developer-verified; not Codex-accepted; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, baseline `8d60a6d`; nothing staged, committed or pushed; no launch, no
live write; the unrelated dirty tree untouched.

## 1. Plain-language summary

The reviewer found two real defects in last night's work and both are fixed. First, the archive
dialog's "whole group" choice could accept two plans that contradict each other — for example
"replace this whole block" for one logo and "replace only this member" for a twin that lives in the
same block — and the contradiction only surfaced as a crash inside the staging code. Now such a pair
is refused in three places (the archive engine, the project's pending list, and the API), with both
members named, and nothing already pending is disturbed. Second, a colour you had started typing on
one team could follow you to the next team when the two teams stored the same colour, and would then
be written to the wrong team on blur. The colour box is now tied to exactly one team, project and
field; switching drops the draft and any error, and a write that is already under way finishes for
the team it was started on, even if you navigate away. Third, the campaign's closing paperwork was
tidied: its stale "blocked" wording is reconciled against the newest research (which decoded the
sound-bank index and parsed the PS4 vault links — neither of which is Studio code yet), an
overstated NHL 07 statement is corrected, the seven sweep logs it left in a temporary folder are now
durable evidence with a manifest, the temporary edit scripts are deleted, and a provisional Phase 2
coverage matrix lists what was delivered without claiming acceptance.

## 2. Fixes

### F1 — conflicting whole-group / private scopes (P1)

| Layer | Change | Behaviour |
|---|---|---|
| Engine — `crates/tdb-archive-big/src/stage.rs` | new `StageError::ConflictingOwnership { index, path, other_index, other_path, contested }`; `plan_scoped` computes, **before any placement**, the file-table rows each replacement owns (its own row; plus every sibling row of the block under the whole-group rule, plan-wide or by index list) and refuses the first row claimed twice. `shared_siblings()` factored out of the placement loop and shared by both. | Whole vs whole (identical or different bytes, fit or append), and whole vs private sibling in either request order, are refused at planning with both paths and the contested row; two private copies of one block, two separate blocks under any mix of scopes, and unshared entries plan/stage/verify as before. The duplicate-index refusal still comes first. |
| Domain — `crates/studio-domain/src/workspace.rs` | `Workspace::asset_member_scope_conflict()` — the members a plan owns in its archive (its member; plus `shared_with` when `whole_group`) against every *other* pending plan of the same archive (the same member supersedes, never conflicts). Called in `asset_member_preview` (refusal kind **`scope-conflict`**, plan never held) and in `asset_member_apply_review` (before the pending list is touched; the held review is kept). | A contradictory plan never joins `asset_members`; pending list, operation log, revision and held reviews are exactly as before the refusal. The apply-side check is defensive: every apply changes the revision and discards held reviews, so a review that conflicts with a *later* apply is refused as stale first. |
| API / UI | no change needed: `AssetMemberPreviewDto.refusal_kind`/`refusal` carry the new kind; `ArchiveMemberReplace.tsx` already renders any refusal kind with its reason. | The dialog shows the refusal text naming both members. |

No winner is chosen, no scope is widened or narrowed, and the engine guard stays even though the
domain refuses first. Private-copy behaviour, coalescing, undo (`AssetMemberOp`), exact copied-target
restoration and `rebind_assets_after_copied_save` are unchanged.

### F2 — colour draft bound to its target (P1)

| File | Change |
|---|---|
| `app/src/components/ColorField.tsx` | `ColorField` now renders an inner `ColorEditor` keyed on `targetKey(projectId, record, components)` = `project/table/record/field+field+…`. A change of project, table, record or component-field set unmounts the old editor (draft, error, pending state gone) instead of carrying the draft over — also when the two targets store the same colour. Inside the editor: the draft re-seeds only when the stored colour of the *same* target changes (refetch after a commit, undo); the mutation takes its full arguments at dispatch (`{projectId, table, record, writes}`) and `onSuccess` refreshes `project_summary` for **`args.projectId`** (the shared `ProjectProvider.update` then adopts the summary only if that project is still open — that guard already existed and is relied on, not duplicated); an `inFlight` ref blocks a second dispatch of the same hex (Enter then blur; blur twice before the refetch lands) and is released on error so a retry re-dispatches. |
| `app/src/components/FieldProfile.tsx` | `cellKey(projectId, cell)` = `label|project/table/record/fields`; `ProfileSections` keys every cell on it rather than on the label, so `EditableField`/`TeamSelect`/`ArenaSelect` cells get the same target binding for free. |

### F3 — reconciliation (see §4), evidence move and cleanup (see §6)

## 3. Regression evidence (developer)

| Case | Where | What it asserts |
|---|---|---|
| `stage::tests::contradictory_whole_group_and_private_scopes_on_one_block_are_refused_at_planning` | `crates/tdb-archive-big/src/stage/tests.rs` | the reviewer's construction (entry 1 aliasing entry 0's rows; `noise(40,3)`/`noise(40,5)` chunkzip payloads) plus the eight refusal combinations (whole/whole identical & different, fit & append, index-list & plan-wide rule; whole/private both orders, fit & append) → `ConflictingOwnership` with indices, paths and contested row; group + unrelated entry stages and re-binds with the sibling following; two private copies stage, verify (`untouched_bytes_compared > 0`), block untouched, re-bound unshared, other entries byte-equal; two separate blocks with whole+whole and private+whole stage and verify; `DuplicateReplacement` unchanged |
| `contradictory_scopes_on_one_block_are_refused_and_leave_pending_and_history_unchanged` | `crates/studio-domain/tests/asset_member_undo.rs` | preview refusals (`scope-conflict`, both scopes) with `pending()`, `revision()` and `asset_member_undo_depth()` unchanged; stale-review refusal unchanged; unrelated member accepted; same-member re-plan supersedes in place (position kept); two private copies accepted; reverse-order conflict refused with the snapshot unchanged; undo re-enables the group scope, which applies |
| `a_contradictory_scope_on_a_pending_block_is_refused_and_the_accepted_plan_still_installs_and_restores` | `crates/studio-api/tests/asset_member_route.rs` | Registry seam: `refusal_kind == "scope-conflict"` naming both members, `pending_changes` unchanged (one member, dirty) for both scopes; unrelated member accepted; copied-target apply installs image A on both paths of the block and B on `l0`; restore byte-exact |
| `ColorField.test.tsx` (13 cases; 9 new) | `app/src/components/` | key shape; the reviewer's reproduction (record 3 → 8, same colour: draft dropped, blur dispatches nothing, a new commit goes to record 8 with `{projectId 7, table kOtt, record 8, writes […]}`); project 7 → 9 same colour (dispatch to 9 only on a new commit; `project_summary` and `update` for 9 only); field-set switch same colour (draft dropped) vs an identical re-render (draft kept); in-flight completion after a target change (held promise; new editor enabled and undrafted; on resolve exactly one `set_cells_int` with the original args, `project_summary {projectId: 7}`, one `update`; a blur on the new team adds nothing); in-flight completion after unmount (same assertions); Enter+blur+blur → one dispatch, a second blur after success → none, a different value → a second dispatch; refusal shown, retry re-dispatches, refusal and draft cleared on a same-colour target switch; re-seed on a stored-colour change |
| `FieldProfile.test.tsx` (1 case, new) | `app/src/components/` | `cellKey` distinguishes record, project, field set and label; triple and derived shapes |

Runs (this session, default modes; `--ignored` fixture sweeps **not** rerun — no engine byte path
changed for the sweeps' inputs beyond the refusal, and the retained logs stand as they were):

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big -p studio-domain -p studio-api` (default mode, all test binaries incl. doc-tests) | **565 passed, 0 failed, 46 ignored** — per crate: `tdb-archive-big` 70 / 0 / 6 (the 31 `stage::tests` incl. the new case), `studio-domain` 387 / 0 / 20, `studio-api` 108 / 0 / 20; `exit 0` |
| `cargo test -p tdb-archive-big --lib stage::tests` | 31 passed |
| `cargo test -p studio-domain --test asset_member_undo` | 4 passed |
| `cargo test -p studio-api --test asset_member_route` | 5 passed, 2 ignored (the real-fixture cases) |
| `cd app && npx vitest run` | **20 files, 146 tests passed** (was 19 / 136: +9 `ColorField`, +1 `FieldProfile`) |
| `cd app && npx tsc --noEmit -p .` | clean |
| `cargo run -p studio-api --bin export-bindings -- --check` | "Bindings are in sync" (119 roots, 227 files) — no DTO changed |

Ignored counts are ignored, not passing. The `studio-domain` mount-gated Xbox 360 probes that return
early on an absent mount remain the pre-existing silent skips of C0.2 §5.4.

## 4. F3 reconciliation — Phase 1 gaps and stale blockers

Sources: research handoffs of 2026-09-18 (`40`), 2026-09-19 (`02` rev. 2, `05`, `06` rev. 2, `07`) and
2026-09-20 (`02`, `04`, `06`, `07`, `11`, `13`), the R19/R20 research review of 20 September, and the
code as it is in this tree. **Exact handoff paths are used below because both the 19 and the 20
September research campaigns have a handoff 07** (`docs/research/overnight-2026-09-19/07-ps4-vault-collections-and-slug-binding.md`
vs `docs/research/overnight-2026-09-20/07-teams-reference-census-vaults-and-audio-keywords.md`);
receipt 07 §6 / receipt 10 §2/§9 "research 07 §1" is the **2026-09-20** file, receipt 10 §2 "research
06 rev. 2 / 11" is 2026-09-19 `06` rev. 2 and 2026-09-20 `11`.

### 4.1 Phase 1 remaining implementation gaps (Phase 1 is open)

| Gap | State | Missing contract / next input |
|---|---|---|
| BIG4 ordered reflow (append breaks index-order == offset-order; recorded, not refused) | **not impl** | research 2026-09-20 handoff 03 (`r10-placement-oracle.json`) gives fit/append/reflow word patches and whole-file hashes for the six `.bh` archives — a developer package can consume it; loader tolerance of append vs reflow is card R2-BIG4-1 |
| NHL 07 `dataN.big` growth (`.bh` mirrored inside `boot.big`) | **blocked by policy** (same-length only) | the two-level cascade is pinned with its slack in handoff 03; not built — an explicit decision to plan the cascade is needed, then a package |
| Different-dimension logos on RW4xb2 (X360 07–11), RW4ps3 (PS3 08–11), GNF (PS4 16–21) | **blocked** (descriptor regeneration) | 2026-09-20 handoff 05 gives the size words per family (incl. the two Apt member-0 floats); the descriptor writers themselves are the missing piece |
| Disc-image placement beyond same-length | **not impl** | no contract |
| Audio (1.5) | **blocked (format) — reworded**, see 4.2 | encoders; gen7 event → sentence routing; `goalhorn` → hornbank id |
| Competition rules / settings / mode initialization (1.2) | **blocked / research unknown** as before | handoff 2026-09-20 `06` §3 tables the obstacles; no new contract |
| League row creation / resizing (1.2) | **not impl** | engine tolerance of a grown table unknown; no consumer contract |
| Localization runtime gates (rebuilt tree, grown pool, glyphs, which copy loads) | **research unknown → cards** | R-A / R-B / R11-LOC-1 |
| Tournament stem bindings, PS4 ids 77/78 | **research unknown** | handoff 2026-09-20 `06` §3 |

### 4.2 Stale blocker statements corrected

| Statement (where) | Correction |
|---|---|
| "labels exist only as literals in `default.xex` (research 05 §1)" — receipt 06 §6 NHL 07 row | **Overstated.** The bounded search found the three `CHTM_LEAGUE_*` **keys** in the executable's `FE::CLeagueIonAdapter` literal block and **no label text anywhere** in the inspected set (2026-09-19 handoff 05 §1; the size exclusion closed with 0 hits in 2026-09-20 handoff 04 §4 — remaining exclusions: nesting deeper than two levels, non-RefPack compression, non-ASCII/UTF-16 encodings). What the game shows is unknown (card R5-07-1). Correct wording: *no located text source; the keys are executable literals; rename needs a class-B key insertion (refused by contract) or an out-of-scope executable edit.* |
| "SND10 bank index and per-platform sample codecs undecoded … no contract sufficient for even a read-only catalogue" — receipt 06 §6 audio row; Phase 1 matrix 1.5 | **Superseded at the research level.** 2026-09-20 handoff 02 decodes the bank index (`SBle` all 418 PS4 banks; `SBKR`, `ABKC` gen7) with offsets validated against sample headers, and the NHL 21 sentence → sample chain; 2026-09-20 handoff 07 §3 binds team/arena/chant audio ids to `pbp_keywords.bin` ids; handoff 10 decodes the gen7 music box; handoff 13 parses `EVT0`. **Still blocked for Studio:** no Studio reader consumes any of it (the probes are research Python), no encoder for EA-MP3 / Opus-shaped / EALayer3 / XMA, codec `0x09` unidentified, DB `goalhorn` → hornbank id open, and the gen7 event → sentence link is a *name join, not a traced selection* (R19/R20 review F3). A read-only catalogue contract now exists on paper; it has not been entered. |
| "the stock division-weighted format is not established; inferring it from stock seasons is forbidden" — receipt 06 §6 schedule row | **Partly superseded.** 2026-09-20 handoff 06 measures the per-era NHL quotas by opponent class on all 25 anchors and proposes a "stock-shaped" generation contract; the AHL tables have no class rule. The contract is **proposed, not accepted**; generation stays blocked until Codex accepts it. |
| "PS4 vault slug binding unresolved (research 06 rev. 2 / 11)" — receipt 10 §2, §9 | **Mis-stated.** The typed links are **research-parsed**: arena packs by `ArenaID`, centre/spotlight/BAP logos by `Team`, jersey triples → `ge_texture` slug (2026-09-19 handoff 07; 2026-09-20 handoffs 01, 11), read by the research probe `vlt_collections.py`. What is unresolved is different: no Studio reader or writer of the vault, no complete runtime selector (`IsTopScorer` discriminator, board override groups, GameMode names), and collection-key derivation for *new* entries (Q-R7-1). Decoder-level research ≠ promotion into Studio ≠ an accepted writer. |
| "gen7 `rendering/*` members are chunklzx — decoder unpinned, no encoder … the chunklzx decoder promoted (its oracle test is mount-gated …) and an encoder or a loader-accepted substitute codec (the boot-write probes crashed on raw substitution)" — receipt 10 §2, §9 | **Three things conflated.** (a) *Framing research:* pinned by 2026-09-18 handoff 40 — 50 members from six titles decode byte-exact **by the research probe**. (b) *Promotion into Studio:* **not done** — `tdb-archive-codec::decompress_chunklzx` still carries the walker defects handoff 40 lists, the public `decompress_if_wrapped` returns `UnsupportedLzx`, `eb::decode_payload` returns `Payload::UnsupportedLzx`, and the oracle `eb.rs::chunklzx_decodes_against_extracted_oracle` is `#[ignore]` and mount-gated (returns early on an absent mount — a skip, not a pass). (c) *Encoder / substitute:* none; loader tolerance of `chunkzip`-for-`chunklzx` is **unknown** (2026-09-19 handoff 02 §8, card R2-LZX-1). The clause "the boot-write probes crashed on raw substitution" has **no recorded evidence** in any receipt, research handoff or test in this tree (the only "boot write probes" in the repository are the two `QA_ASSET_OUT`-gated PNG dumps in `head_preview.rs`, unrelated); it is withdrawn as unestablished. |
| "which screen consumes which family / colour / name is research unknown everywhere" — receipt 10 §2 | Stands, with one narrowing: 2026-09-20 handoff 11 names the arena-keyed and team-keyed vault packs on PS4 (which pack, not which screen); the screen question remains the user cards T-1 … T-3. |
| Receipt 10 §2 "PS4 real archives 7/7" | The retained log holds **six** titles (16–21); NHL 15 is recorded in receipt 03 §4 only (evidence manifest). |

### 4.3 Provisional Phase 2 matrix

[phase-2-coverage-matrix-2026-09-20.md](phase-2-coverage-matrix-2026-09-20.md) — an accounting of
receipts 07–09 plus the pre-existing team surfaces, every cell carrying `dev` / `listing` /
`by identity`, none `acc` or `game`. Listing, capability and identity cases are marked as such and
are not parity evidence. No feature was added to fill a cell.

## 5. Changed identities

| Path | Change |
|---|---|
| `crates/tdb-archive-big/src/stage.rs` | `StageError::ConflictingOwnership`; ownership check in `plan_scoped`; `shared_siblings()` |
| `crates/tdb-archive-big/src/stage/tests.rs` | +1 case |
| `crates/studio-domain/src/workspace.rs` | `asset_member_scope_conflict()`; preview + apply guards |
| `crates/studio-domain/tests/asset_member_undo.rs` | +1 case |
| `crates/studio-api/tests/asset_member_route.rs` | +1 case |
| `app/src/components/ColorField.tsx` | keyed editor, bound mutation, in-flight guard, `targetKey` |
| `app/src/components/ColorField.test.tsx` | rewritten mock (gated / failing commands), +9 cases |
| `app/src/components/FieldProfile.tsx` | `cellKey`; cells keyed on target |
| `app/src/components/FieldProfile.test.tsx` | new |
| `docs/validation/overnight-dev-20260920-r1-corrections.md` | this record |
| `docs/validation/phase-2-coverage-matrix-2026-09-20.md` | new (provisional) |
| `docs/validation/evidence/overnight-development-2026-09-20/MANIFEST.md` + `logs/*.log` (7) | moved evidence + manifest |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | R1 pointer paragraph in the header |
| `docs/receipts/overnight-development-2026-09-20/06-…`, `10-…`, `STATUS.md` | correction notes at the top; bodies left as written |

No DTO changed; the bindings check is expected unchanged (§3). No `defs/`, anchors, fixtures or
installs touched.

## 6. Cleanup

- **Moved (copy, hash-compared on both sides, then source deleted):** the seven logs from
  `H:\TEMP\claude\e--Repositories-nhl-modding-studio\6e40fb65-…\scratchpad\logs\` →
  `docs/validation/evidence/overnight-development-2026-09-20/logs/` (manifest with SHA-256s).
- **Deleted with verified containment** (every path's realpath checked under that package's
  scratchpad root, no symlinks, 43 files): `c1_texture_replace.py`, `c4_*.py` (3), `c5_*.py` (5),
  `c6_*.py` (2), `c7_*.py` (5), `c8_*.py` (8), `c9_*.py` (7), `c10_*.py` (3), `ed.py`, `pkgs.py`, and
  the seven log copies. The scratchpad directory is empty.
- **Not touched:** shared `target/` build cache, `app/node_modules`, `E:\nhl-studio-fixtures`
  (`imagery/`, `loc-ps4/`, `teamlogos/` — inputs of the `--ignored` sweeps), the pre-existing
  `H:\TEMP\nhlms-*` roots and every other session's scratchpad, the untracked root-level files.
- **This session's scratchpad:** `vitest_all.log`, `cargo_affected.log` (this package's run logs);
  disposable, deleted at close.
- Processes: none left running.

## 7. Omissions and remaining independent-review areas

- The `--ignored` fixture sweeps (PS4 / gen7 EB / BIG4 / team logos / PS4 locales / loose routes)
  were **not** rerun: the engine change adds a refusal before placement and changes no byte of an
  accepted plan; the retained logs remain the evidence and are dated before this package.
- No desktop or game launch; F2 is jsdom evidence only; the F1 UI path (refusal text in the
  archive dialog) is covered by the existing generic refusal rendering, not by a new component case.
- `cargo fmt --check` / `clippy -D warnings`: repo-wide baseline defects (C0.2) — not repaired;
  the new code was not reformatted by `cargo fmt` to avoid touching unrelated lines.
- The apply-side domain guard is unreachable through the public preview → apply sequence (every
  apply invalidates held reviews); it is retained as defence in depth and documented as such.
- Not certified here (unchanged from the review's own scope): the canonical dictionary serializer,
  the BIG4 writer, `replan`, the C6 dimension writers, and every Phase 2 cell's parity.
- Codex review areas: the ownership rule (rows, not bytes — two private copies of one block are
  accepted by design; identical-bytes whole/whole is refused by design), the `scope-conflict`
  wording, the keyed-editor approach in `ColorField` (remount vs. reconcile), the `cellKey` shape,
  §4.2's withdrawals (NHL 07 label source; the substitution-crash clause), and whether the
  provisional Phase 2 matrix's evidence classes are the vocabulary to keep.

Stop for Codex review.
