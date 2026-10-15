# Receipt 10 — R3: kit completeness corrected (coordinator F1), the read-only kit surface on Uniforms, and coverage to ten real trees

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 / PS3 trees and committed
anchors read **only**; every test tree is a disposable temp directory that removes itself. Nothing
staged, committed, pushed, launched or written live. Claim: `dev-r3-kit-completeness-correction`.
Dispatch: [the coordinator review of 2026-09-22](../../validation/overnight-development-20260922-review.md),
fresh-session items 1 and 2. Research consumed (unedited): 2026-09-21 pm handoff 14 (R56).

## 1. Plain-language summary

The coordinator found a real defect in the kit join delivered in receipt 08: Studio decided *what a
kit needs* by looking at the folder. If a game tree had no `sock/` folder, socks silently stopped
being required, so a jersey and a pair of pants counted as a complete kit — and as a "complete spare"
that a future re-pointing could target. Missing evidence was being read as good news.

That is fixed at the root. What a kit needs now comes from the **title and platform**, measured by
the research and pinned in code: NHL 13 → Legacy need one sock file; NHL 11 / 12 need **two** (and
both are listed separately); NHL 09 / 10 have no selector table; PS4 and NHL 07 / 08 are not
measured, and on those nothing is ever called complete. The folder only supplies evidence, and every
required piece reports what was found: **present**, **missing** (the folder was read, or does not
exist, and the file is not there), or **unknown** (the folder could not be read, no tree is attached,
or the contract is not measured). One rule turns those into a verdict — complete only when every
piece is present, incomplete when any is definitely missing, unknown otherwise — and the same rule
judges the spare candidates, which are now listed *with* their incomplete members instead of
pretending only complete ones exist.

The coordinator's exact reproduction (jersey and pants, no sock folder) is now a permanent test at
three layers — the resolver, the API seam and the card — and it yields *incomplete*, never complete.

On top of the fix, the join is now **visible**: the Uniforms tab shows a read-only "Kit textures"
card for the selected kit — the title's contract, how each folder read, every required piece with a
thumbnail when present and the reason when not, the row's verdict, the spare candidates with their
own verdicts, and why re-pointing a row is not offered (there is no write contract, and on NHL 11 /
12 nobody knows what the two sock files' 0 / 1 selects). A late answer for a team the user has already
left is never painted as the current team's evidence.

Coverage grew from three real trees to **ten**: Xbox 360 NHL 11 / 12 / 13 / 14 / 15 / Legacy and PS3
NHL 11 / 14 / 15 / Legacy (`.rpsgl`), each resolving 100 % of its rows under the contract rule. PS3
NHL 12 / 13 keep their kits inside an archive and are recorded as not covered.

Three earlier receipts were corrected with dated notes as the review asked (§6).

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/kit_files.rs` | **rewritten** — `KitContract` / `SockGrammar` / `kit_contract(game_id, platform)`; `RenderingListing` (one read per family directory, `DirEvidence::{Listed, Absent, Unreadable}`); `KitComponent` with `ComponentStatus::{Present, Missing, Unknown}`; the single `completeness()` rule; `KitView.components` (both sock files on the two-file grammar); `SpareKit` with its own verdict; `reassignment_supported = false` with the reason; `parse_stem` as the exact inverse of the templates. 14 unit cases (was 3). |
| `crates/studio-domain/src/session.rs` | `Session::team_kit_rows` — the team's own `artid` and its rows together. |
| `crates/studio-domain/tests/kit_selector_join.rs` | ten real trees; asserts the contract, every required component, the shared rule on rows and spares, real files with the platform extension, and that no complete spare lacks a file. |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `TeamKitsDto` / `KitDto` / `KitComponentDto` / `SpareKitDto` / `KitDirectoryDto`; `Registry::team_kits` (contract from the project's title and platform, evidence from the attached tree, a note when the install's declared platform disagrees); the `team_kits` command; export roots 128 → **133**. |
| `crates/studio-api/tests/team_kits.rs` | **new** — the registry seam: the F1 reproduction end-to-end, the two-file grammar, the NHL 10 / PS4 contracts, PS3 `.rpsgl`, the platform-disagreement note. |
| `app/src/bindings/{TeamKitsDto,KitDto,KitComponentDto,SpareKitDto,KitDirectoryDto}.ts` | **generated** (`--write`; `--check` in sync). |
| `app/src/components/TeamKitsCard.tsx` (+ `.test.tsx`), `app/src/components/UniformsTab.tsx`, `app/src/lib/client.ts` | the card, mounted under the jersey metadata in the kit-details column; the command typing. |
| `docs/receipts/overnight-development-2026-09-22/{02,05,08,09}-*.md` | dated correction notes (§6). |
| `docs/receipts/overnight-development-2026-09-22/OBSERVATIONS-FOR-RESEARCH.md` | **new** — the durable observations file (dispatch item 3). |
| `docs/validation/phase-2-coverage-matrix-2026-09-20.md` | row 2.6 gen7 updated. |

## 3. Evidence

### 3a. Red / green for F1

The red is the coordinator's reproduction: with the receipt-08 resolver, a tree of
`jersey/texlib_0_7_0.rx2` + `pant/texlib_0_7_0.rx2` and no `sock/` returned that triple as a complete
spare (the old `shipped_triples` only intersected socks when `sock_template_is_measured` found one).
That code path no longer exists; the following cases encode the scenario and its neighbours and are
green against the rewrite (`cargo test -p studio-domain --lib kit_files`: **14 passed**):

| Case | Scenario | Verdict asserted |
|---|---|---|
| `f1_no_sock_directory_never_yields_a_complete_kit_or_spare` | no `sock/` at all, both grammars | row **incomplete**, socks *missing* with "absent", jersey / pants still *present*; the unused pair listed as an **incomplete** spare; the directory evidence says absent |
| `f1_an_empty_sock_directory_is_missing_evidence_not_complete` | `sock/` exists, empty | incomplete; "not in the listed directory"; directory *listed, 0 files* |
| `f1_the_two_file_form_with_one_sock_file_is_incomplete_and_names_the_missing_file` | NHL 11 grammar, only `sock_0_0_7_0_cm` | incomplete; `socks (file 1)` missing with its expected stem; the same half-kit as a spare gets the same verdict |
| `f1_files_in_the_other_sock_grammar_are_observed_but_not_joined` | a single-file title whose tree holds two-file names (and the reverse) | incomplete; a note counts the files in the other grammar |
| `f1_an_unreadable_sock_directory_is_unknown_not_absent` | `sock` is a regular file (read_dir fails, not NotFound) | **unknown**, not missing; the spare is unknown; the note says "not proven absent" |
| `a_missing_component_beats_an_unknown_one_in_the_verdict` | pants absent + socks unreadable | incomplete |
| `without_a_rendering_tree_the_kits_list_but_claim_nothing` | no tree | unknown; three unknown components; no spares |
| `an_unmeasured_or_tableless_title_lists_rows_with_no_components_and_no_spares` | NHL 08, no title, NHL 21 PS4, NHL 10 | unknown; no components; **no spares even though files exist** |
| `a_complete_single_file_kit_…`, `a_complete_two_file_kit_…`, `a_ps3_tree_…` | the supported layouts | complete; both sock files listed on NHL 11 / 12; `.rpsgl` preferred on PS3 |
| `the_contract_follows_the_title_and_platform_not_the_tree`, `stem_parsing_is_the_exact_inverse_of_the_templates`, `custom_team_rows_…` | contract table; parser; refusals | — |

Through the seam (`cargo test -p studio-api --test team_kits`: **3 passed**): the F1 tree against the
real NHL 15 anchor yields an incomplete row, an incomplete spare and no complete spare, then turns
complete when the sock is added; NHL 11 lists `socks (file 0)` / `socks (file 1)` and refuses to call
one file complete; NHL 10 reports `no_selector_table`, NHL 21 PS4 `unmeasured` with rows but no
components; a PS3 project expects `.rpsgl` and names an install declared as the wrong platform.

In the card (`npx vitest run src/components/TeamKitsCard.test.tsx`: **5 passed**): the sock-less kit
renders *incomplete* with the missing reason and the summary never says "1 complete"; the spare is
listed as incomplete with "socks missing"; a selected kit with no row shows as a spare candidate; a
late answer for team 1 after moving to team 2 is not painted; a payload naming another team is
treated as still loading; the backend error is shown.

### 3b. Real trees (`cargo test -p studio-domain --test kit_selector_join -- --ignored`, 47 s) —
[r3-f1-kit-join-ten-trees.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-f1-kit-join-ten-trees.log)

| Tree | Teams with rows | Rows (R56) | Complete | Sock components | Spare candidates complete / incomplete / unknown |
|---|---|---|---|---|---|
| NHL 11 X360 | 212 | **627** (627) | **627** | 1 254 (2 per kit) | 97 / 20 / 0 |
| NHL 12 X360 | 213 | **676** (676) | **676** | 1 352 | 255 / 22 / 0 |
| NHL 13 X360 | 218 | **684** (684) | **684** | 684 | 207 / 1 / 0 |
| NHL 14 X360 | 216 | **704** (704) | **704** | 704 | 315 / 2 / 0 |
| NHL 15 X360 | 216 | **659** (659) | **659** | 659 | 54 / 0 / 0 |
| Legacy X360 | 222 | **703** (703) | **703** | 703 | 228 / 58 / 0 |
| NHL 11 PS3 (`.rpsgl`) | 212 | 627 | **627** | 1 254 | 97 / 20 / 0 |
| NHL 14 PS3 | 216 | 704 | **704** | 704 | 316 / 2 / 0 |
| NHL 15 PS3 | 216 | 659 | **659** | 659 | 54 / 0 / 0 |
| Legacy PS3 | 222 | 703 | **703** | 703 | 228 / 58 / 0 |

Every row on every tree resolves every required component; the complete spare counts equal
receipt 08's figures where they overlap (97 / 54 / 228), and the incomplete candidates are new,
honest information. Observations for the research lane from this run (teams-with-rows definition,
incomplete spares, PS3 12 / 13 archives) are rows O-3, O-4 and O-6 of
[OBSERVATIONS-FOR-RESEARCH.md](OBSERVATIONS-FOR-RESEARCH.md).

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib kit_files` | **14 passed** |
| `cargo test -p studio-domain --test kit_selector_join -- --ignored` | **1 passed** (10 trees) |
| `cargo test -p studio-api --test team_kits` | **3 passed** |
| `cargo test -p studio-api --lib bindings` | 25 passed (`live_bindings_match_the_dto_contract` included) |
| `export-bindings --write` then `--check` | 5 written, in sync — **133 roots, 252 files** |
| `npx tsc --noEmit` | clean |
| `npx vitest run` (full) — [r3-frontend-full.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-frontend-full.log) | **28 files, 191 passed** (was 27 / 186) |
| `cargo test -p studio-domain --lib` — [r3-studio-domain-lib.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-studio-domain-lib.log) | **198 passed, 0 failed, 2 ignored** (recorded after receipt 11's route change as well) |
| `cargo test -p studio-api` (lib + every target) — [r3-studio-api-all.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-studio-api-all.log) | **30 targets: 115 passed, 0 failed, 32 ignored** (private-fixture / real-tree cases), exit 0 |

`zz_diag` was not invoked (the domain crate was run as `--lib` and named targets only).

## 5. Coverage effect (Phase 2 matrix 2.6, gen7)

| Row | Before (receipt 08) | After |
|---|---|---|
| Completeness rule | from the directory listing — absent socks lowered the bar | **from the title / platform contract**; the tree is evidence only; one rule for rows and spares; unreadable = unknown |
| Components shown | one path per family; the two-file sock form collapsed to "the first file" | every required component (two sock entries on NHL 11 / 12), each with present / missing / unknown and a reason |
| Spare pool | complete triples only | every candidate with its verdict (complete / incomplete / unknown); only complete ones could ever be targets |
| Real-tree coverage | X360 11 / 15 / Legacy | X360 11 / 12 / 13 / 14 / 15 / Legacy + PS3 11 / 14 / 15 / Legacy; PS3 12 / 13 recorded as not covered (in-archive) |
| UI | none | read-only card on Uniforms, bound to project / team / install root / platform; late results owned by the team they name |
| Reassignment | "the spare pool is the target set" | **unsupported**, with the reason on the payload and the card |

## 6. Reporting corrections (coordinator review)

| Receipt | Correction (dated note added in place) |
|---|---|
| 09 §1 | the kit bullet now says backend join implemented / tested, **not user-facing at closeout**; the GNF bullet says *API route the dialog calls*, and that header reproduction says nothing about pixel survival |
| 02 §1 | the same two distinctions added to the summary paragraph (§3b already had them) |
| 05 §1 | "editing a constant per-arena would be meaningless" withdrawn — constancy is observed data, folding is a presentation choice; the card's read-only marking is scoped to the card and imposes no write prohibition on the existing Misc drawer |
| 08 (top) | superseded-in-part note: the listing-driven completeness rule is withdrawn; the counts stand and are re-measured here |

## 7. Not done / limitations

- **PS3 NHL 12 / 13**: kits live in `cacherender.big` / `nocacherender.big`; the resolver reads loose
  trees only, so those two targets are *not covered* (observation O-6 asks whether the members are
  byte-identical to the Xbox 360 files).
- **No writer.** Re-pointing a row, new kits and texture replacement through this card are not
  offered; each needs a write contract, validation against the *complete* candidates, the pending-
  review path and a runtime card. On NHL 11 / 12 the sock sub-index meaning is a prerequisite.
- The card shows thumbnails through the existing `texture_preview` route; it does not decode
  anything new. Desktop review of the card is card **V-30** in MORNING-REVIEW.
- The "teams with rows" figures differ from R56 on three titles while the row totals agree (O-3);
  not treated as a defect on either side.

## 8. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `wire_kits_api.py`, `mount_and_receipts.py` | deleted at this checkpoint |
| Temp roots `nhlms-kit-*`, `nhlms-kitapi-*` | removed on drop by the tests; verified absent |
| Background processes | none left running |
| Evidence logs `r3-f1-kit-join-ten-trees.log`, `r3-frontend-full.log`, `r3-studio-api-all.log`, `r3-studio-domain-lib.log` | retained |

## 9. Next action

Item 4 of the dispatch: the Xbox 360 mip-layout audit — receipt 11 and
[xbox360-mip-layout-audit-2026-09-22.md](../../validation/xbox360-mip-layout-audit-2026-09-22.md).
