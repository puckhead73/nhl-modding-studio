# Receipt 16 — R3 / Misc 5.4 (Teams 2.7 PS4): the song catalogue as a read-only listing

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real PS4 extracted trees on `H:` read
**only**; the test's temp root removes itself. Nothing staged, committed, pushed, launched or
written live. Claim: `dev-r3-song-catalogue`. Dispatch: item 5 of the coordinator review of
2026-09-22 (Miscellaneous presentation). Research consumed (unedited): 2026-09-21 pm handoff
[12](../../research/overnight-2026-09-21-pm/12-ps4-songs-catalogue.md) (R54 §1–3).

## 1. Plain-language summary

The NHL 17–21 installs ship a catalogue of the songs a team's goal, power-play and win music can be
chosen from — a title key, the composer, a genre, and eligibility flags — that no Studio surface
showed. The Team page now lists it: every row, the genre names from the sibling genre file, a
filter by title / identifier / composer / genre, and the counts. It is deliberately labelled
"selectable songs (catalogue)" and says in its own text what the research measured as *not*
established: the team's song cells are not catalogue ids (none of the shipped non-zero values
joins), and no sound bank carries the catalogue's audio ids as samples — so the card never claims
which song a team has, offers no preview and writes nothing. NHL 15 / 16 and the Xbox 360 / PS3
titles say why nothing is listed instead of showing an empty table.

On the real installs the listing reproduces the research: 422 / 637 / 245 / 191 rows on
NHL 21 / 19 / 18 / 17, NHL 19's repeated header rows skipped and named, the genre ids named from
`songgenres.csv`, and NHL 15 refusing by contract with its real root attached.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/song_catalogue.rs` | **new** — `SongCatalogue::{path_for, genres_path_for, load, parse}` (columns by name; NHL 17 / 18 without `isintrosong`; repeated headers skipped and counted; non-UTF-8 bytes replaced and noted; genres from `songgenres.csv`), `song_listing` (contract by title / platform, filter, cap, genre counts, the two R54 negatives as notes). 3 unit cases. |
| `crates/studio-domain/src/lib.rs` | module registration. |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `SongCatalogueDto` / `SongRowDto`; `Registry::song_catalogue(project, install_root, query, limit)`; the command; export roots 140 → **142**. |
| `crates/studio-api/tests/song_catalogue.rs` | **new** — the seam: a gen7 project and a PS4 root without the file say why; `--ignored`: the real NHL 21 / 19 / 18 / 17 catalogues and NHL 15's refusal. |
| `app/src/bindings/{SongCatalogueDto,SongRowDto}.ts` | **generated** (`--write`; `--check` in sync). |
| `app/src/components/SongCatalogueCard.tsx` (+ `.test.tsx`), `app/src/pages/TeamDetail.tsx`, `app/src/lib/client.ts` | the card under the name-calls card; the command typing. |

## 3. Evidence

`cargo test -p studio-api --test song_catalogue -- --ignored` —
[r3-song-catalogue-real.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-song-catalogue-real.log), **1 passed**:

| Install | Rows (R54) | Distinct composers (R54) | Genres named | Repeated header | Flags |
|---|---|---|---|---|---|
| NHL 21 (CUSA18182) | **422** (422) | 134 (135) | 12 (`$TXT_ROCK` 96 … `$TXT_NONE` 1) | — | goal / win on every row; intro never |
| NHL 19 (CUSA11126) | **637** (637) | 132 (133) | 12 | 1 skipped and noted | as above |
| NHL 18 (CUSA07569) | **245** (245) | 128 (129) | 12 | — | as above |
| NHL 17 (CUSA04326) | **191** (191) | 96 (97) | 12 | — | as above |
| NHL 15 (CUSA00561) | — | — | — | — | **refused by contract** ("NHL 15 / 16 ship no song catalogue") |

Composers are one fewer than R54 on every title: Studio counts distinct non-empty names after
trimming and case-folding; the handoff's figure evidently includes the `None` row's empty
composer or a case variant. Recorded as observation O-10, not adjusted.

Unit (3): the NHL 21 shape with intro flags, genres and a repeated header; the NHL 17 shape without
intro and two wrong-header refusals; the listing's unavailable reasons (non-PS4, NHL 15, an unmeasured
title, no title, a PS4 root without the file), the filter / cap / genre counts, and a non-UTF-8 byte
replaced and noted. Seam (1, default run): the gen7 refusal, the missing-file reason, NHL 15. Card
(2): rows with genre and eligibility, the genre pills, the filter re-query, the unavailable reason,
the error.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib song_catalogue` | **3 passed** |
| `cargo test -p studio-api --test song_catalogue` (default) | **1 passed**, 1 ignored |
| `cargo test -p studio-api --test song_catalogue -- --ignored` (real installs) | **1 passed** |
| `export-bindings --write` / `--check`; bindings drift test | 2 written, in sync, **142 roots**; passed |
| `npx tsc --noEmit` | clean |
| `npx vitest run src/components/SongCatalogueCard.test.tsx` | **2 passed** |

## 5. Coverage effect (Phase 2 matrix 2.7 / Misc 5.4, PS4 17–21)

| Row | Before | After |
|---|---|---|
| Team song selection (PS4 17–21) | `*songid` cells raw; no listing of what can be chosen | **read-only catalogue listing: impl+tested, dev** on the real NHL 17 / 18 / 19 / 21 installs; the cells stay raw with "no catalogue join" stated (R54 §2) |
| NHL 15 / 16, gen7 | — | refused with the reason (no catalogue; gen7 music tables / banks are the 2026-09-20 handoff 10 reading) |
| Preview / replacement | — | unchanged: none (catalogue → bank binding open, R54 §2) |

## 6. Not done / limitations

- The `$TXT_SONG_*` title keys are shown as keys; resolving them through the PS4 dictionary is a
  follow-up (the same route the team text card uses).
- No join from a team's song cells to the catalogue, by measurement; no preview; no writer.
- NHL 20's install was not run (its anchor is the NHL 19 base; the catalogue is the install's own).
- Desktop review card **V-35**.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `wire_songs.py` | deleted at this checkpoint (the scratchpad is empty) |
| Temp roots `nhlms-songs-*`, `nhlms-songsapi-*` | removed by the tests; verified absent |
| Background processes | none |
| Evidence log `r3-song-catalogue-real.log` | retained |

## 8. Next action

The campaign closeout (receipt 17).
