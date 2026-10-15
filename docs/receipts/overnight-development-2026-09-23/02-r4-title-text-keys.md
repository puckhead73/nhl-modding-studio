# Receipt 02 (R4) — Song and arena title keys resolved through the open title's own dictionary

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Committed PS4 anchor dictionaries read
**only**; the seam test's install root is a disposable temp directory. Nothing staged, committed,
pushed, launched or written live. Dispatch: [the coordinator review of receipts 10–17](../../validation/overnight-development-20260922-receipts10-17-review.md),
fresh-session item 2. Claim: `dev-r4-title-text-keys`.

## 1. Plain-language summary

The PS4 song catalogue and the arena goal-horn catalogue name things by dictionary keys
(`$TXT_SONG_ALIEN_DANCE`, `$NHLTeamName_ANA`). Until now the cards showed those keys, and the song
card even prettified a genre by chopping the key's spelling. Both cards now resolve the keys through
the **same text service the team-text card uses**, and only through the **dictionary attached to
the open project** — never another title's. A key the dictionary lacks, or a project with no
dictionary attached, keeps the raw key on screen with an explicit state ("not in this project's
attached dictionary", "no dictionary is attached"), and every reply says which dictionary it
resolved through so a reply for a different attachment is recognisable. Nothing is edited: no new
field meanings are inferred from the labels, and the candidate meanings / constants of the venue
card are disclosed exactly as before.

Measured on the way: the PS4 dictionaries carry the song / genre / team keys **without** the `$`
the CSVs write (`TXT_SONG_ALIEN_DANCE` resolves exactly on the NHL 17 / 18 / 20 anchors,
`$TXT_SONG_…` never; NHL 15 / 16 carry no song keys at all), so one leading `$` is stripped before
the lookup and that is stated in the card's note.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/text_keys.rs` | **new** — `KeyLookup` (`exact` / `by-hash` / `absent` / `no-dictionary` / `not-a-key`), `KeyText`, `key_of`, `resolve_key_text` (over `leagues::resolve_text`), `DictionaryStamp` / `stamp`, `resolution_note`; unit case on the NHL 17 and NHL 15 anchor dictionaries |
| `crates/studio-domain/src/song_catalogue.rs` | `SongRow { title, title_lookup, genre_label, genre_lookup }`, `SongGenreCount`, `SongListing { dictionary }`; `song_listing(…, dictionary)` resolves title and genre keys (the query matches the resolved text too) and adds the resolution note |
| `crates/studio-domain/src/arena_venue.rs`, `view.rs`, `session.rs` | `build(…, dictionary)`: the goal-horn row's name key resolves through the session's dictionary; `VenueFieldView { resolved_key, resolved_key_lookup }`; the note names the key's state; unit case with the NHL 17 anchor dictionary (exact) and an absent key |
| `crates/studio-api/src/{dto,registry,convert,bindings}.rs` | `SongGenreCountDto`, `DictionaryStampDto`, the new `SongRowDto` / `SongCatalogueDto` / `VenueFieldDto` fields; `Registry::song_catalogue` passes `ws.display().dictionary()` (export roots 143 → 145) |
| `crates/studio-api/tests/song_catalogue.rs` | **new** `song_and_horn_keys_resolve_through_each_projects_own_dictionary_across_attach_detach_reload_and_switch` |
| `app/src/bindings/{SongCatalogueDto,SongRowDto,SongGenreCountDto,DictionaryStampDto,VenueFieldDto}.ts` | **generated** |
| `app/src/components/SongCatalogueCard.tsx` (+ `.test.tsx`) | title / genre from the resolved text or the raw key (no key parsing in the frontend), lookup state in the tooltip and `data-lookup`, the dictionary line; one new Vitest case |
| `app/src/components/ArenaVenueCard.tsx` (+ `.test.tsx`) | the resolved horn text keeps `data-key-lookup`; unresolved keys render amber |

## 3. What the seam test proves (`song_catalogue.rs`, committed anchors, disposable root)

Two PS4 projects open side by side (NHL 17 and NHL 18), a catalogue with one shipped key and one
made-up key, a genre file, and a goal-horn catalogue:

1. **Nothing attached** — both projects: `title: null`, `title_lookup: "no-dictionary"`, the raw
   key on display, no stamp, the note says no dictionary is attached and none is borrowed; the horn
   key is `no-dictionary`.
2. **Attach NHL 17's dictionary to A** — A resolves `Alien Dance` / `Rock` (`exact`), the made-up
   key is `absent` and stays raw, the stamp is the attached file's locale / sha / rows, the note
   counts 1 resolved / 1 not in the dictionary, a query on the resolved text matches; **B is
   unchanged** (`no-dictionary`, no stamp).
3. **Attach NHL 18's to B** — B resolves under its own stamp, which differs from A's.
4. **Re-attach A with NHL 15's** (the reload path) — the song key becomes `absent` (NHL 15 carries
   no song keys), the stamp changes; a reply carrying the old stamp is recognisably stale.
5. **Detach A** — raw keys, no stamp, horn `no-dictionary`; B still resolves.
6. **Close A** — B still resolves under its own stamp.

Late replies: every reply is stamped with the dictionary it resolved through, and the app shell
already invalidates every query on attach / detach; the card shows the stamp's locale beside the
labels. (The frontend has no dictionary state of its own to compare against; the stamp is the
disclosure.)

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib -- song_catalogue arena_venue text_keys` | **7 passed** |
| `cargo test -p studio-api --test song_catalogue --test arena_venue` | **3 passed**, 2 ignored (real PS4 trees) |
| `export-bindings --write` / `--check` | 5 written, in sync |
| `npx vitest run src/components/SongCatalogueCard.test.tsx src/components/ArenaVenueCard.test.tsx` | **7 passed** |
| `npx tsc --noEmit` | clean |

## 5. Coverage effect (Phase 2 matrix 2.5 / 2.7, PS4)

| Row | Before | After |
|---|---|---|
| 2.7 song catalogue titles / genres | keys shown as keys; genre prettified by string parsing in the card | **resolved through the project's own dictionary: impl+tested, dev** (anchor dictionaries NHL 15–18 / 20); unresolved keys explicit |
| 2.5 goal-horn name on the venue card | `$NHLTeamName_ANA (AnaheimDucks)` | text through the project's own dictionary with the key and its state disclosed |

## 6. Not done / limitations

- Resolution follows the **attached** dictionary; which dictionary file a title reads at runtime
  and whether the console shows these strings on these screens is not claimed.
- The real PS4 install runs (`--ignored`) were not rerun for this package; the seam test uses
  synthetic catalogues and the committed anchor dictionaries (NHL 21 has no anchor dictionary).
- NHL 20's anchor dictionary is the NHL 19 base (known); no claim about NHL 20's own file.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `locprobe/` (the dictionary probe) | deleted at closeout |
| Temp roots `nhlms-songloc-seam-*` | removed on drop; verified absent |
| Background processes | none |

## 8. Next action

Item 3 — PS3 NHL 12 / 13 archive-backed kits ([receipt 03](03-r4-ps3-archive-kits.md)).
