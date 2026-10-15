# Receipt 06 — LOC-W2: free text in place (Phase 1.3 / 2.3 prerequisite)

> **R2 correction (18 September, F3):** LOC-W2 is one bounded in-place operation (byte-exact tests on Legacy, NHL 10 and PS4 only). Phase 1.3 is not closed by it: per-title round trips, other locales, growth/new keys and consumer tracing remain — [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md) §1.3 (package P1-D).

Overnight developer lane, 18 September 2026. Consumes research handoff P1.3-X
(`docs/research/overnight-2026-09-18/13-localization-free-text-capacity.md`; claim
`p1.3-localization-capacity`). Status: **implementation complete / automated verified / user
review pending.** Nothing launched, nothing live written.

## Plain-language summary

Until tonight Studio could only rename a league by pointing its dictionary key at *another* key's
existing text (LOC-W1: "Norway" can become "Rest Of World" because that string exists). Research
measured why free text was deferred: every dictionary is full (no spare rows), its text pool is
packed to the end, and each title's compression tree only knows the characters its own strings
use — NHL 10's tree has no lowercase letters at all. The cheapest useful free-text edit is
therefore to rewrite a key's *own* string in place with the title's own tree, when the new text's
compressed bits fit the bytes the old string already occupies. That is what LOC-W2 does: the
file length, every pointer, every other string and the tree stay identical; only that string's
bytes and the checksums change. Studio refuses precisely — it names each character the tree
cannot encode and tells you how many bits the text needs against how many are available — and it
discloses which other keys point at the same string (they change too) and which Studio-known
consumers read the key (leagues, competition groups). Because nothing moves, every existing
placement route works unchanged: in-place save, copied-target transaction, same-length archive
member, disc-image range patch. "Norway" → "Nordic" on Legacy, "NATIONAL LEAGUE" → "SWISS
LEAGUE" on NHL 10 (upper-case only), and "Rest Of World" → "Rest Of Earth" on PS4 are verified on
the committed dictionaries. Longer text, new keys and new characters remain the growth path
(not built).

## Implementation

- `crates/tdb-loc/src/retext.rs` (new): `plan_retext(bytes, key, text)` (resolve key exact/by-hash,
  refuse null pointer / same text / unsupported characters / too long / over capacity; compute old
  and new bit lengths and byte extents; list sharers and hash collisions), `apply_retext`
  (fresh-plan equality, round-trip invariant, write 2-byte BE length + bits + zero slack, recompute
  CRCs, verify every scope, verify every row: changed rows read the new text, all others identical),
  `tree_alphabet`. `HuffmanTree::codes` / `decode_with_extent` added.
- `crates/studio-domain/src/dictionary_edit.rs`: `DictionaryPlan { Repoint | Retext }` with
  `reapply`, `detail`, `producer`; `DictionaryRetextPreview`; `preview_retext` / `apply_retext`;
  `PendingDictionary.kind`. `txn::dictionary_artifact` re-applies whichever plan is held
  (producer `tdb-loc::repoint` / `tdb-loc::retext`). `Workspace::dictionary_retext_preview` /
  `set_dictionary_retext` (consumer disclosure appended, as for re-points).
- `studio-api`: `Registry::{dictionary_retext_preview, dictionary_retext_set}`, commands,
  `DictionaryRetextPreviewDto`, `PendingDictionaryDto.kind`; bindings (96 roots).
- App: the rename panel has two modes — "text of another key" (LOC-W1) and "free text (fits in
  place)" (LOC-W2) — with the bits-used/available accounting in the preview; the pending review
  labels the kind.

## Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-loc` (incl. new `tests/retext.rs`: Legacy Norway→Nordic byte-exact + refusals; NHL 10 upper-case tree; PS4 little-endian; every committed tree lacks `~`) | 7 unit + 5 + 4 passed (private fixtures ignored) |
| `cargo test -p studio-api --test dictionary_rename` (incl. `free_text_retext_flows_through_pending_review_copied_target_and_in_place_save`) | 5 passed |
| `tsc --noEmit`; `vitest run` | clean; 68 passed |
| Checkpoint 5 (`tdb-loc`, `studio-domain` in_place_save/copied_target/pending, `studio-api` full, bindings) | `local/runs/overnight-development-2026-09-18/checkpoint5-rust.txt` |

## What this unlocks and what stays open

- 1.3: league labels can now be *arbitrary* text where the new text fits (most short labels do)
  on every title with a dictionary; on PS4 both routes exist (native key reassignment, and this).
- 2.3 (team text): the per-team key families (`NHLTeamName_{artabbr}`, `NHLCityName_`, `TEAMLINE1/2_`,
  `Abbr3_`, `TXT_NICKNAME_`, `X_XLA_TEAM_`) become editable in place with the same constraint —
  the Studio surface for that (a team-text card) is the next step, not yet built.
- Growth path (longer text, new keys, tree rebuild) remains out of scope; the game's tolerance of
  a grown dictionary member is a runtime question research flagged.

## Changed paths

New: `crates/tdb-loc/src/retext.rs`, `crates/tdb-loc/tests/retext.rs`, bindings
`DictionaryRetextPreviewDto.ts`. Modified: `crates/tdb-loc/src/{lib.rs,repoint.rs}`,
`crates/studio-domain/src/{dictionary_edit.rs,txn.rs,workspace.rs}`,
`crates/studio-api/src/{dto.rs,convert.rs,registry.rs,commands.rs,lib.rs,bindings.rs}`,
`crates/studio-api/tests/dictionary_rename.rs`, `app/src/lib/client.ts`,
`app/src/components/DictionaryRenamePanel.tsx`, `app/src/pages/PendingEdits.tsx`,
bindings `PendingDictionaryDto.ts`.
