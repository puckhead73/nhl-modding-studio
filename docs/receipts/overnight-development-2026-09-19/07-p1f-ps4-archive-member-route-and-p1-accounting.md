# Receipt 07 — D7 Phase 1 accounting + D8 / P1-F (PS4) archive-member texture route

> **Correction note (OVERNIGHT-DEV-R1, 19 Sep).** Left as written below except §5's closing sentence.
> **F1** — the dialog in `ArchiveMemberReplace.tsx` had no ownership of its async steps and ignored the
> returned summary (the global project state could stay clean after the first texture edit); corrected —
> picker / preview / apply are bound to project, install root and pick generation, the summary is adopted
> for the owning project only, Apply cannot be dispatched twice, Cancel is not offered while an apply is in
> flight (8 jsdom cases). **F2** — "a later apply supersedes the earlier; undo pops the latest" discarded the
> earlier replacement of the same member (A → B → undo left nothing pending); corrected with an operation
> log beside the coalesced list (undo restores A at its position and revision; 3 domain cases + 1 route
> case; the Pending Edits button now reads "Undo the last texture replacement"). **F3** — "real NHL 21
> `l0.gnf` member" means the real member bytes in a **synthetic** EB archive; no real `inxp_10*.big` of any
> title was opened, so the matrix carries NHL 16–20 as impl, test missing. §4's "`tsc --noEmit` clean" did
> not hold when rerun under R1: the D8 case in `LeagueImageryCard.test.tsx` had a type error (fixed under R1). The R1 record:
> [overnight-dev-r1-corrections-2026-09-19.md](../../validation/overnight-dev-r1-corrections-2026-09-19.md).

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

On PS4 the league logos and presentation imagery are never loose files — the game reads them out of
`databigs/inxp_10.big` (NHL 15: `inxp10.big`, NHL 19: `inxp_10-010.big`) — so a replaced logo only
reaches the game when that archive member is rewritten. Studio can now do that through the same
copied-target save the databases and the settings members use: pick an image on the logo or imagery
card ("Replace in archive…"), Studio plans the member rewrite (same texture container and size,
re-encoded in the member's own stored form), holds it as a pending change with undo, and the next
copied-target save stages the archive with backup and byte-exact restore. Deduplicated members —
most PS4 logos share their bytes with their French twin, another league id or a scouting-region logo —
get a **private copy**: the new bytes are appended and only the chosen row is re-pointed, the siblings
keep the shipped bytes, and the review names them. Proven offline on a real NHL 21 `l0.gnf` member.
Separately, two Phase 1 test gaps were closed (local-bits group numbers on every Xbox/PS3 08–11 anchor;
NHL 07 logo index/listing). Whether the console loads the rewritten archive is a user card.

## 1. Package, disposition, scope

- **D7** — Phase 1 offline accounting: matrix cells that were "impl, test missing" and closable
  without new research are now impl+tested; the remaining open cells are named with their next
  action in §5. **D8 / P1-F (PS4 side)** — delivered, developer-verified, pending independent
  review. Not built: gen7 package placement into BIG4 / `cache.big` / `.bh` (research 02 §4.2 inner
  BIGF re-plan); different-dimension replace; whole-group replace of a dedup group.

## 2. Changed paths and hashes (baseline `8d60a6d`; SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-big/src/stage.rs` | `449ee2f35d31f0ba…` | `SharedSpanRule {Refuse, PrivateCopy}`, `plan_with`, `PlannedEdit.shared_with` (siblings by span intersection); a shared entry under `PrivateCopy` is never placed in slot |
| `crates/tdb-archive-big/src/stage/tests.rs` | `a457896b4a6dffac…` | +1 case: private copy appended, sibling row/bytes untouched, re-bound no longer shared; default rule still refuses |
| `crates/studio-domain/src/asset_member_edit.rs` | `ea6ccd6c348492e5…` | **new** — `plan_asset_member_edit` (EB table read, member decode, `replace_texture_in_file`, native chunkzip / raw re-encode with decode check, sibling detection, placement prediction), `AssetMemberPlan`, `AssetMemberRefusal` |
| `crates/studio-domain/src/workspace.rs` | `d3c0524dbcc8440c…` | pending `asset_members` (+ held reviews, revision identity `+asset:`, dirty, reload clears), `asset_member_preview/review/apply_review/edits`, `undo_asset_member_edit`, `rebind_assets_after_copied_save` |
| `crates/studio-domain/src/copied_target.rs` | `8f711fffa593a566…` | `CopiedTargetPlan.asset_members` (grouped per archive), `member_replacements(.., allow_shared)` |
| `crates/studio-domain/src/league_logos.rs` / `league_imagery.rs` | `bf047291836f21bf…` / `2aabd0a6d0c1eabe…` | `archive` on every PS4 member (`ps4_logo_archive` per title) |
| `crates/studio-domain/tests/membership.rs` / `league_logos.rs` | `ea4e0ad99128056e…` / `17d7bf3833f0dc3c…` | D7: local-bits sweep on 8 anchors; NHL 07 inferred index + existence |
| `crates/studio-api/src/registry/copied_target.rs` | `3991b5e8f1878734…` | `stage_asset_members` (per archive: bind → `plan_with(PrivateCopy)` → stage → `archive-asset-<n>`), reservation of those archives, post-apply rebind |
| `crates/studio-api/src/{registry,dto,convert,commands,lib,bindings}.rs` | `8c25b5be…` / `21e58446…` / `292812c4…` / `0b4b27c0…` / `62114a91…` / `cb184eea…` | `AssetMemberPlanDto/PreviewDto/ApplyDto`, `PendingReviewDto.asset_members`, `archive` on the logo/imagery member DTOs; commands `asset_member_preview` / `asset_member_apply_review` / `asset_member_undo`; export roots 114 → 116 |
| `crates/studio-api/tests/asset_member_route.rs` | `650bf988fa535b28…` | **new** — 2 default cases (PNG flow with a shared member → private copy, pending, undo, supersede, copied-target review/apply/restore; refusals: not-found / invalid path / non-texture / missing archive) + 1 `--ignored` real NHL 21 GNF member |
| `app/src/bindings/*` | generated | `--write`; `--check` clean |
| `app/src/lib/client.ts` | `1c4f4b121c50a8d9…` | typings |
| `app/src/components/ArchiveMemberReplace.tsx` | `e9833dc19609065e…` | **new** — pick → plan dialog (container, sizes, placement, siblings, refusal) → Apply → pending result |
| `app/src/components/LeagueLogoCard.tsx` / `LeagueImageryCard.tsx` (+test) | `3067e045…` / `2afa9823…` / `b2a1f2ff…` | "Replace in archive…" / "French in archive…" when the member ships in an archive; +1 jsdom case |
| `app/src/pages/PendingEdits.tsx` | `b8141c313ec84edf…` | "Archive textures" pending card with "Drop the last texture replacement" |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | (updated) | 1.2 group-names, 1.4 index/listing/placement rows |

## 3. Behavior and decisions

- **Private copy (research 02 §1.4/§2).** The stager never writes a shared span in place, even when
  the new bytes would fit; the chosen row alone is retargeted at an appended copy, the shared block and
  every sibling row are byte-identical afterwards, and the edit lists the sibling paths. The D4 text
  route keeps the refusing rule. Whole-group replace is not offered.
- **Plan = the loose-file writer on the decoded member.** The member is decoded (chunkzip / raw),
  rewritten by `replace_texture_in_file` (GNF BC7 re-encode, PNG re-encode; RW4/BIGF-packaged members
  are refused — the gen7 package route is not built), re-encoded with the native-framed chunkzip
  encoder when the source was chunkzip (decode-checked), and bound to the entry's stored identity; the
  copied root's archive must hold that identity or the review refuses before staging.
- **Pending model.** One pending replacement per (archive, member); a later apply supersedes the
  earlier; undo pops the latest; the revision carries an `+asset:` identity so held reviews go stale
  on any change; a reload clears them; the copied-target save reserves every targeted archive, stages
  each as `archive-asset-<n>`, and the session drops the pending set only after the commit.
- **UI.** The archive route sits beside the loose-file Replace on both cards, only for members whose
  title ships them in an archive (server-provided `archive`); the dialog states that nothing is written
  until the copied-target save and shows the siblings.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big --lib stage::tests` | 29 passed |
| `cargo test -p studio-api --test asset_member_route` | **2 passed**, 1 ignored |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --ignored` | **1 passed** (real NHL 21 `l0.gnf`: private copy at the old EOF, `_fr` twin bytes unchanged, native framing, BC7 round trip MAD < 6, restore byte-exact) |
| `cargo test -p studio-domain --test membership every_local_bits` / `--test league_logos nhl07` | 1 / 1 passed |
| `cargo test -p studio-api --lib bindings` / `export-bindings --check` | 25 passed (roots 116) / in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run` | clean / **15 files, 112 passed** |
| broader affected suites (`tdb-archive-big`, studio-domain copied_target/settings_csv_edit/txn_coordination/pending/workspace, studio-api settings_csv_route/copied_target/dictionary_growth) | see the final receipt's tally (run at receipt time) |

Not run: full workspace suite; `zz_diag`; clippy/fmt (baseline defects); desktop; any game.

## 5. Phase 1 accounting — what remains open (D7)

| Cell | State | Next action |
|---|---|---|
| 1.2 league row creation / resizing | not impl (engine tolerance research unknown) | research question; no Studio change until answered |
| 1.2 rules/settings on gen7 / PS3 / 07–09 | blocked (compiled table / no representation) | research; executable patching not authorized |
| 1.3 NHL 07 dictionary labels | blocked (labels not in the dictionary) | research (`CHTM_LEAGUE_*` source) |
| 1.3 canonical tree rebuild (O-G) | not impl | serializer C + proofs C1/C2 (own package) |
| 1.4 different-dimension replace | not impl | Apt-record float rewrite + container resize; runtime unknown → card first |
| 1.4 gen7 package placement into BIG4 / `cache.big` / `.bh` when the length changes | not impl | span-preserving inner BIGF re-plan (research 02 §4.2) + BIG4 writer |
| 1.4 runtime precedence loose vs archive vs image | research unknown | user cards |
| 1.5 cue replacement | blocked (format) | SND10 bank index (shared with 2.7) |

~~Everything else in 1.1–1.5 is impl+tested per target or n/a by evidence (matrix).~~ **Withdrawn under R1 F3:** the sentence was not a per-operation statement. The matrix is the only such statement; the cells R1 narrowed (loose-replace API/UI flow, non-logo preview/replace, PS4 16–20 archive route, 1.5 bank absence) are listed in the R1 record §3.

## 6. Temporary artifacts / processes

- Scratchpad `d8_*.py` patch scripts — disposable. Test temp dirs `nhlms-d8-api-*` removed by `Drop`.
  No processes left.

## 7. Runnable commands

```sh
cargo test -p tdb-archive-big --lib stage::tests
cargo test -p studio-api --test asset_member_route
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --ignored
cargo test -p studio-domain --test membership --test league_logos
cd app && npx vitest run src/components/LeagueImageryCard.test.tsx
```
