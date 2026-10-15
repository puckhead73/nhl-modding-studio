# Receipt 07 — Priority 2.7 (PS4): the goal horn reads as a club, not a number

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Real extracted PS4 trees and committed anchors read **only**. Nothing
staged, committed, pushed, launched or written live. Claim: `dev-p2-goalhorn-catalogue-join`.
Research consumed (unedited): 2026-09-21 pm handoff
[08](../../research/overnight-2026-09-21-pm/08-ps4-goalhorn-catalogue-join.md) (`research-r50-ps4-goalhorn-catalogue`).

## 1. Plain-language summary

On the PS4 games an arena's goal horn was a bare number in the venue list — `1`, `41`, `690` — with
nothing to say what any of them meant. The games ship a catalogue of horns (`goalhorns.csv`: the 30
club horns, 85 generic ones, and on NHL 19+ the 63 elements and 97 flairs a custom horn is built
from). Studio now reads it and shows the horn's name next to the number, with the rule it used
available on hover.

The rule has one wrinkle the research measured: a club horn is stored **one higher** than its
catalogue entry, while generic horns are stored as-is. Two shipped gaps are reported honestly rather
than smoothed over — the New York Rangers have no row in the catalogue on any of the five games, and
the values 690–693 mean "nothing selected". Both say so in words instead of showing a wrong horn.
Across the two real installs checked, **every single arena** either names a horn or carries a named
reason; none was left as an unexplained number.

This names a catalogue row. It does **not** say which sound the console plays — the catalogue-to-bank
link is still open research — and it is not a writer.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/goalhorn_catalogue.rs` | **new** — the CSV reader (columns by name, so NHL 17 / 18's smaller column set parses without positional guessing), `HornClass` (none / team / generic / element / flair) and `resolve` implementing R50 §2's combined rule with its two named refusals. |
| `crates/studio-domain/src/arena_venue.rs` | `VenueFieldView.resolved` / `.resolved_note`; `build` takes the catalogue and joins **`goalhorn` only**. |
| `crates/studio-domain/src/{view,session,lib}.rs` | the catalogue is loaded from the attached install root when there is one; its absence leaves the horn a number (not an error). |
| `crates/studio-api/src/{dto,convert,registry,commands}.rs` | the two new DTO fields and the `installRoot` argument. |
| `app/src/components/ArenaVenueCard.tsx` (+ test), `app/src/lib/client.ts`, `app/src/bindings/VenueFieldDto.ts` | the resolved name, the "not in the catalogue" marker and the rule in the tooltip. |
| `crates/studio-api/tests/arena_venue.rs` | the real-tree join case. |

## 3. Evidence

`cargo test -p studio-api --test arena_venue -- --include-ignored`
([r1-p2-goalhorn-join.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p2-goalhorn-join.log)):

| Title | Arenas | Club horns | Generic | Sentinels | Rangers gap | **Unresolved** |
|---|---|---|---|---|---|---|
| NHL 21 | 450 | **150** | 286 | **11** | **3** | **0** |
| NHL 17 | 256 | **46** | 209 | 0 | **1** | **0** |

Every figure matches R50 §2 independently: its "rows resolving" row reads `46 / 49 / 150 / 134 / 150`
for NHL 17–21 (this run: 46 on NHL 17, 150 on NHL 21), its Rangers residual `1 / 1 / 4 / 3 / 3`
(1 and 3), and its sentinel band "1–11 arenas" (0 and 11). **Zero** arenas on either title resolved
to neither a row nor a named reason — the assertion the test makes.

Unit cases (`goalhorn_catalogue`, 3): the club rule, the generic rule, all four sentinels, the
Rangers gap, an id no row carries, element / flair classification, the NHL 17 / 18 column set
(a missing `tone` reports −1 rather than shifting the parse), and refusals for a header without the
measured columns, an empty file and a header with no rows.

`arena_venue` (3 domain cases): the horn resolves with its rule; **`periodhorn` deliberately does
not** — it shares the id space but R50 did not join it, so it stays a number rather than being
resolved by assumption; without a catalogue nothing resolves and nothing pretends to.

UI (4 cases): the club name with the rule in the tooltip, and "not in the catalogue" with the reason
for a value that names no row.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-api --test arena_venue -- --include-ignored` | **3 passed** |
| `cargo test -p studio-domain --lib goalhorn` / `--lib arena_venue` | 3 / 3 passed |
| `npx vitest run` (full) | **27 files, 186 passed** |
| `npx tsc --noEmit` | clean |
| `export-bindings --check` | **in sync** (247 files) |

## 5. Coverage effect (Phase 2 matrix 2.7, PS4)

| Row | Before | After |
|---|---|---|
| PS4 `goalhorn` | a raw id — "DB `goalhorn` → hornbank id open" | **→ catalogue closed: impl+tested (dev)** on NHL 17 and NHL 21 real installs; the club / generic / element / flair row is named, the two shipped gaps are named refusals, and the rule is auditable on the row |
| PS4 `goalhorn` → **sound** | open | **unchanged, still open** (R50 §3: catalogue → bank not established) |
| `periodhorn` | raw id | **unchanged, deliberately** — same id space, not joined by the research |
| Reassignment (writing a new horn id) | not implemented | **unchanged** — this is a read-side join; no write path, no runtime card |
| NHL 15 / 16 | no catalogue shipped | unchanged — the horn stays a raw id and the card says the catalogue was not found |

## 6. Open / not done

- Catalogue → bank → the actual sound (research-side, R50 §3).
- `periodhorn`, and the NHL 19–21 horn-builder columns (`goalhornelement1..3`, `goalhornflair`, …):
  classified by the venue map, value space known (catalogue rows 500–686), **not** joined or edited.
- No reassignment: choosing a horn from the catalogue would be a validated cell edit and needs its
  own package (and a runtime card, since it changes what the game is asked to do).

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `p2_horn_join.py` | deleted at checkpoint |
| Evidence `r1-p2-goalhorn-join.log` | retained |
| Generated binding `VenueFieldDto.ts` | rewritten by `--write`, verified by `--check` |
| Temp roots / background processes | none |

## 8. Next action

The gen7 kit-selector join (research handoff 14: `stockteamjerseys` row → jersey / pant / sock
textures by `(style, teamartid, variant)`, 100 % on NHL 11–Legacy both consoles) — Phase 2.6's
static join.
