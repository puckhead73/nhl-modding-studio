# Receipt 05 — Priority 2.5: the arena's venue parameters, inventoried and classified per title

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Committed anchors read only; nothing written outside the repository's
own new files. Nothing staged, committed, pushed, launched or written live.
Claim: `dev-p2-arena-venue-inventory`. Research consumed (unedited): 2026-09-21 pm handoff
[09](../../research/overnight-2026-09-21-pm/09-arena-venue-parameter-inventory.md) (`research-r51-arena-venue-inventory`).

## 1. Plain-language summary

The Arena page used to end in a drawer called "Misc fields" — every column of the arena table that
the curated sections above did not show, listed as raw names and numbers with no indication of what
any of them were or whether changing one meant anything.

That drawer now has a proper card above it. Every venue field of the open game is grouped by what
kind of thing it looks like (seating and attendance, horns, lighting, scoreboard and logos, the
building's surfaces, and so on), each group saying in plain words what the grouping means. The
colour fields the PS4 games use for custom arenas are shown as actual colour swatches instead of
numbers like `14671319`. Fields that never differ between arenas in your file are folded away behind
a "show 33 constant fields" button — a presentation choice: a constant is an *observation* about
this file's rows, not a finding about how the game consumes the field, and it says nothing about
whether a per-arena value would be honoured. And fields nobody can explain are listed but **not**
offered for editing *on this card*, with the reason on the row.

> **Correction note (22 Sep 2026, coordinator review):** this paragraph originally said editing a
> constant field per-arena "would be meaningless". That was unsupported — constancy is observed
> data, not a consumer result — and is withdrawn. Likewise the card's read-only marking is a
> presentation decision on the card only: it imposes no new write prohibition anywhere else, and the
> existing raw Misc drawer (§5) still edits every field it always did, constant or unclassified.

Two deliberate choices about honesty. First, the grouping is a *guess from the field's name*, not a
discovery about what the game does — no arena field has a traced effect, and the card says so, every
time, in its own text. Second, which fields your game has and which of them never vary are worked
out **from your open file**, not from a table of what the research lane saw, so a patched or
modified database reports itself correctly instead of being described by someone else's copy.

## 2. Files changed

| Path | Change |
|---|---|
| `defs/arena/venue-fields.toml` | **new** — the developer-owned class map: 159 fields, each with its candidate class and, for 41 of them, the measured 24-bit-colour flag. Transcribed from R51's JSON; Studio never reads the research lane's data at runtime. |
| `crates/studio-domain/src/arena_venue.rs` | **new** — `VenueClass` (10 classes, each with a gloss and an `editable()` rule), `VenueFieldMap` (loads the defs file; a missing file is an error, not an empty map), and `build` which joins the map to per-field values measured on the open database. |
| `crates/studio-domain/src/view.rs` | `arena_venue_view` — reads this row's value and the whole column for every field of the title's arena table. |
| `crates/studio-domain/src/session.rs` | `Session::arena_venue_view`. |
| `crates/studio-domain/src/lib.rs` | module registration. |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `ArenaVenueDto` / `VenueFieldDto`, the conversion, the registry method, the `arena_venue_view` command, the export roots. |
| `app/src/bindings/{ArenaVenueDto,VenueFieldDto}.ts` | **generated** (`export-bindings --write`; the check reports "in sync"). |
| `app/src/components/ArenaVenueCard.tsx` (+ `.test.tsx`), `app/src/pages/ArenaDetail.tsx`, `app/src/lib/client.ts` | the card, mounted above the existing Misc drawer (which stays as the raw editable fallback). |
| `crates/studio-api/tests/arena_venue.rs` | **new** — the inventory across four era signatures on the committed anchors. |

## 3. Evidence

`cargo test -p studio-api --test arena_venue`
([r1-p2-arena-venue.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p2-arena-venue.log)):

| Anchor | Fields | Arena rows | Constant | Unclassified | Classes present |
|---|---|---|---|---|---|
| PS3 NHL 07 (gen6 thin table) | 3 | 93 | 1 | 1 | identity, unknown |
| PS3 NHL 08 (rink-dressing set) | 31 | 138 | 5 | 5 | identity, reference, capacity, audio, horn, presentation, venue_geometry, unknown |
| X360 NHL Legacy (gen7 core) | 16 | 187 | 1 | 5 | identity, reference, capacity, audio, horn, venue_geometry, unknown |
| PS4 NHL 21 (colour zones + horn builder) | **65** | 450 | 33 | 5 | + lighting, flag |

Independent agreements with R51 that the test did **not** encode in advance:

- NHL 21's 65 fields sits inside the handoff's "60–67 on NHL 18–21".
- **17 of 450** NHL 21 arena rows carry at least one colour zone — 3.8 %, matching R51 §2's
  "non-zero on 1–4 % of rows (the created / custom arenas)".

Asserted per field, on every era: a class and a gloss are always present; the class counts sum to
the field list; an `unknown` field is never editable and always carries the "not classified" reason;
a read-only field always has a reason; `constant` and `distinct == 1` agree, and the constant value
equals this row's value; a colour always renders as `#RRGGBB`; a pre-PS4 title has no colour zones;
the "not a traced runtime effect" note is always in the payload.

`ArenaVenueCard.test.tsx` (3 cases): the grouping and gloss, a colour rendered as a swatch and hex
(and *not* as the raw number), identity fields read-only with "identity card" on the row, constants
hidden until asked for then shown with their reason, and the domain's refusal surfaced instead of an
empty card.

### A research classification kept, and reported rather than overwritten

R51's classes are name-based. `seat1zone1` — a 24-bit **colour** by its own measured value space —
lands in `capacity` because of the word "seat". Studio transcribes the class **as the research
recorded it** (silently re-grouping another lane's measurement would hide the disagreement) and
groups and renders by the *measured* colour flag instead, with the reason stated in the card's own
notes and in the defs file. Recorded here as an observation for the research lane; nothing was
edited in `docs/research/**`.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-api --test arena_venue` | **2 passed** |
| `cargo test -p studio-domain --lib arena_venue` | 2 passed |
| `npx vitest run` (full) | **27 files, 185 passed** (was 26 / 182) |
| `npx tsc --noEmit` | clean |
| `export-bindings --check` | **in sync** (128 export roots, 247 files) |

## 5. Coverage effect (Phase 2 matrix 2.5)

| Row | Before | After |
|---|---|---|
| Venue parameters beyond identity | **not inventoried in Studio terms** | **inventoried: impl+tested (dev)** on the four era signatures — every field of the open title's arena table listed with a candidate class, its constancy measured over that file's own arena rows, colour zones rendered as colours, and unclassified fields refused for editing with the reason |
| What the game does with any of them | unknown | **unchanged** — no consumer is traced and none is claimed; no runtime card is opened, because the card only describes data the user already owns |

Editing a *classified, non-constant* venue field goes through the existing cell-edit path (the Misc
drawer's `FieldRow`), so pending review / undo / save behaviour is unchanged by this package.

## 6. Open / not done

- No field has a traced runtime consumer; the classes stay candidates.
- The horn-builder columns (NHL 19–21) are classified but not joined to the goal-horn catalogue —
  that is handoff R50's separate slice.
- The card does not yet offer a colour **picker** for the zone fields; they edit as numbers through
  the existing drawer. A picker is the natural follow-up and needs no new evidence.
- NHL 17's one-off 57-field signature set is classified by the same map but not separately tested
  (the anchor is covered by the map, not by a case of its own).

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `p2_arena_api.py`, `p2_arena_session2.py` | deleted at checkpoint |
| Evidence `r1-p2-arena-venue.log`, `r1-p2-frontend-full.log` | retained |
| Generated bindings | committed to the working tree by `--write`, verified by `--check` |
| Temp roots / background processes | none |

## 8. Next action

Priority 2 continues with the gen7 player portrait / head containment card (research handoff 11 →
the Player asset card on every gen7 title, correcting a 60 000-pool assumption for NHL 09–13).
