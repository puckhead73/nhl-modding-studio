# Receipt 01 — C0: R1 verification, foundation audit, scratch cleanup

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`,
branch `feat/phase5-asset-tooling`, dirty tree preserved.

## Plain-language summary

The two fixes from the previous review (the archive-texture dialog that could act on the wrong project,
and the undo that threw away an earlier logo replacement) were re-run by their tests and read through
line by line: they hold, including the real NHL 21 member case. No new defect was found in that route.
The rest of the previous campaign's foundations were inspected for concrete failure risks; one real
one was found and is fixed in the next receipt: every league-logo / portrait package written back to a
loose file was being *rebuilt* in a layout no shipped package has, instead of being patched in place.
The two obsolete scratch scripts of the R1 package were removed.

## 1. R1 verification (executed, not just read)

| Check | Command | Result |
|---|---|---|
| A → B → undo restores A; interleaved members unwind in order; every undo discards held reviews; reload clears | `cargo test -p studio-domain --test asset_member_undo` | **3 passed** |
| Route: preview → apply → pending → undo → supersede → copied-target review/apply (private copy) → reopen → restore; A→B→undo→save installs A; refusals; **real NHL 21 `l0.gnf` member** | `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test asset_member_route -- --include-ignored` | **4 passed** (incl. the private fixture) |
| Dialog ownership: exact preview/apply args, summary adoption for the owning project only, late picker / preview / apply after switch or unmount, install-root-only change, triple-click Apply, Cancel inert while applying, refused apply drops the plan, server refusal of the preview | `cd app && npx vitest run src/components/ArchiveMemberReplace.test.tsx` | **8 passed** |

Source review of `ArchiveMemberReplace.tsx`, `workspace.rs` (`asset_member_apply_review`,
`undo_asset_member_edit`, `rebind_assets_after_copied_save`, `asset_members_identity`),
`stage.rs::plan_with` (multi-append advances `staged_eof`; shared spans never fit in slot under
`PrivateCopy`) and `copied_target.rs::member_replacements` (stored identity of the copied archive checked
against the plan; stored form and independent decode checked): no reproducible defect. Points noted, not
defects: `invalidateQueries()` is global (refreshes everything, correct but broad); the dialog's pick
button is offered even when the project has no install root (the hook returns silently — the cards only
render the archive route when the server reports an `archive`, so unreachable in practice).

## 2. Foundation audit (D1–D7) — what was inspected, what was tested, what remains unreviewed

| Foundation | Inspected | Tested here | Finding |
|---|---|---|---|
| D6 texture codecs / package writer | `assets.rs::replace_portrait_texture` (BIGF placement), `tdb-rx2::texlib` fixes | receipt 02's sweeps (77 packages, 34 members, 64 imagery packages, 22 PS4 imagery members) | **Defect (fixed in C1):** the BIGF package writer rebuilt the package through `ArchiveBuilder::build` (unaligned members, no table trailer, different `archive_size` rule) — research 02 §2 already recorded that the builder reproduces none of the shipped layout. Every loose logo / portrait / imagery write therefore produced a layout no shipped package exhibits. Replaced by the span-preserving re-plan (receipt 02). Also found: research 02 §2's "16 zero bytes" tail is **not zero** — see receipt 02 §3 |
| D4 native chunkzip + paired CSV | receipt 04 claims vs `settings_csv_edit.rs` / `tdb-archive-codec` | not rerun (unchanged since R1; 6/2/6 suites passed under R1) | no new risk identified; the loader question remains a card |
| D5 LOC-W2/W3 preserving serializer | receipt 05 | not rerun (unchanged) | left for its own package (C-LOC, Priority 3) |
| D1/D2/D3 settings decoding, catalogue, NHL 15 editor | receipts 01–03 | not rerun (unchanged; 9+1 / 11 / 3 suites passed under R1) | no new risk identified from the receipts; **independently unreviewed** |
| D7 test-gap closures | receipt 07 §5 | — | matrix wording only |
| D8 archive-member route | §1 above | yes | holds |

Explicitly **not** independently reviewed here: the D1 site decoding's four offset-binary sites, the D3
single-cell CRC proof, the D5 collision multisets and class-B refusals. They are unchanged code; their
suites are in the aggregate checkpoint at closeout.

## 3. Scratch cleanup (R1 statement checked)

- R1's `r1_matrix.py` / `r1_receipts.py` were in the R1 session's scratchpad
  `H:\TEMP\claude\e--Repositories-nhl-modding-studio\7c000168-0cbe-49f8-8b67-8fd4b8df546d\scratchpad\` —
  resolved as literal paths, plain directory (no reparse point), no process holding them; **removed**
  (2 files). Directory left empty (harness-owned).
- `local/runs/overnight-development-2026-09-19/*.py` (5 fixture/defs generators, 21 KB) — retained:
  regenerable generators that document how the private fixtures and generated defs were produced
  (successor dependency: receipt 02 reuses the same fixture root and the research readers they import).
- `%TEMP%` (`H:\TEMP`): no `nhlms-*` directories from prior packages found; this campaign's tests remove
  theirs by `Drop`.

## 4. Checks executed

See §1. Additionally `cargo test -p studio-domain --lib assets` (55 passed) and `cargo test -p studio-api
--test registry` (6 passed) after the C1 writer change (receipt 02).

## 5. Temporary artifacts / processes

Scratchpad `pkgs.py` (survey listing) — disposable. No processes left.

## 6. Next

Receipt 02 (C1): loose-route closure — span-preserving package writer, loose-route ownership,
changed-pixel sweeps on every family, non-logo imagery fixtures and sweeps.
