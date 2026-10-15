# Save Editor — first implementation (12 October 2026)

**Summary.** The launch screen's Save Editor now opens a single save file or extracted game database, works out which game and platform it belongs to, and edits it with the same database screens Mod Studio uses — leagues, teams, players, arenas, transactions, pending changes and validation. Graphical assets, audio, localization and game settings (tunables) are deliberately absent. Saves usually carry only part of the game database; the tables a save lacks are shown read-only from Studio's bundled copy of that game's default database (the anchors), so every screen is complete while only the save's own data can change and be written back. The first save of a session copies the original file into the app data folder.

Evidence level: **implemented and automated-tested** against the committed anchors. Nothing here is game-verified; see the checklist at the end.

## 1. What a user can open

| File | Result |
|---|---|
| PS3 `SYS-DATA`, Xbox 360 save file (container v1–v4) | Identified, opened, edited, saved. v1 (NHL 10/11) writes stay experimental (un-reversed `tag_1c`), shown as a note. |
| PS4 `DATA` (LZ4 / chunkzip) | Identified, opened, edited, saved. PS4 write-back is structurally checked only (note shown). |
| Extracted `nhlng.db` / `NHL07NG.db` / `nhl_game.db` | Identified, opened, edited, saved; never supplemented. |
| Roster saves | Allowed. Tables the save lacks (e.g. arenas, league tables) shown read-only. |
| Franchise / Dynasty, Season, Be a Pro, 3-on-3 saves | Allowed after an explicit acknowledgement. They hold their mode's tables, not the rosters (verified, §3); teams and players come from the default database read-only. Mode progress is not kept consistent with edits. |
| NHL 07 / NHL 08 roster saves | Roster **change lists** against the game database. Opened applied to the bundled database and edited; saved as a regenerated change list (§3a, added 13 Oct). |
| Settings / profile / Ultimate Team saves | Refused (settings are out of Save Editor scope). |
| PS3 split saves (`DATA0000…`) | Refused with an explanation (no multi-part reader). |
| NHL 20 PS4 | No bundled definitions (no anchor). Refused by default; an advanced route accepts a user-supplied metadata file, which must describe every table and field of the file. |

## 2. Identification

`studio_domain::save_identify` (`SchemaIndex`, built once per baseline inventory):

1. Container probe — raw TDB, PS3/Xbox container (version, save type, checksums, writability), PS4 save.
2. Byte order separates PS4 from PS3 / Xbox 360 / Recompiled.
3. Schema compatibility — every table/field of the file declared at the same bit width by the target's bundled metadata; incompatible targets are never candidates.
4. Exact layout match against the target's anchor main database or anchor roster save.
5. Title ids in the path (RPCS3 `BLUS…`/`BLES…` folder, Xenia title-id folder, PS4 `CUSA…`).
6. Save kind refined from a PS4 save's `NAME` file or, for an unrecognised container magic, the save's file/folder name.
7. The editor's team/player model must build over the file as it will be opened (supplemented, §3).

Result over the anchors: every available target's main database and roster save identifies as its own **game** (`game_confident`). Legacy and Legacy Recompiled count as one game. PS3 saves also settle the platform from their folder's product code. A bare database or an Xbox save outside a Xenia tree leaves PS3 / Xbox 360 (/ Recompiled) open; the UI asks the user, preselecting the first.

## 3. Supplementing partial saves (user direction, 12 Oct)

Probe results (`examples/save_table_overlap.rs`, `save_tdb_scan.rs`, `save_records_scan.rs`):

- Gen7 franchise/season/Be a Pro containers hold **one** TDB of mode tables (33–43 tables, which the main DB carries as empty capacity-0 placeholders) plus a mostly zero trailer. PS4 NHL 15/16 mode saves hold one compressed TDB record (39–59 tables, descriptors identical to the main DB). No hidden second roster database was found.
- Roster saves lack arena, league and other tables the game reads from its own database.

`studio_domain::table_overlay` builds one composite TDB: the save's header, directory and table region verbatim, then directory entries and blocks for every anchor table the save lacks. Directory offsets are relative to the directory's end, so the save's offsets stay valid. Saving strips the appended tables and recomputes the CRCs; an unedited save round-trips byte for byte. Appended tables are frozen:

- `EditSession::run` refuses any operation that changes a frozen table (header or body), naming the table;
- fields of frozen tables read as non-editable (scoped in `Registry::with_workspace`);
- `Project::package` writes only the save's tables; post-save and reload verification recompose before comparing.

PS4 NHL 17/19 roster payloads end with an 18–20-byte record that embeds the TDB size; the NHL 18 3-on-3 payload ends with JSON; the NHL 15 franchise payload with padding. Trailers are re-appended verbatim. Because the embedded size is not rewritten, a save whose edits change the database length (row growth) is refused while a trailer is present.

## 3a. NHL 07 / NHL 08 change lists (addendum, 13 October 2026)

`studio_domain::change_list_overlay`. The composite is the bundled game database with the save's insert / delete / update companions applied ([`roster_difflist::effective_roster`]); parent rows stay in database order, inserted keys follow in key order, so an empty change list shows the database byte for byte.

- **Editable:** the parent tables of the `rosters` save group (bios, roster rows, player slots, free agents, team strategies, AI ratings, …). Every other table is frozen. A parent without a delete + insert pair also freezes the fields its update companion does not carry (`table.field` entries in the frozen scope; `view::read_field` honours them).
- **Edit time:** `EditSession::run` builds the change list for every committed operation and refuses one the save cannot record (an unrecordable field, a full companion), naming the table/field.
- **Save:** only keys whose row changed since the file was read are regenerated; every other companion row is kept as the file holds it. A changed key is written as a delete + insert pair where the parent has both companions (NHL 07 game-written roster rows), otherwise as an update row (game-written skater ratings); a key back at the database value records nothing. Touched companions are key-sorted, `max == current`, allocation type 6 (empty 2). The result is re-projected and must equal the composite's rows field for field. The container keeps its fixed length (zero tail), `checksum_a` recomputed; a non-zero tail or an overflow is refused.
- **Evidence:** the general writer reproduces the game-returned NHL 07 saves byte for byte — ROSTER2 + roster-row jersey 38 = ROSTER3, ROSTER3 + skater raw speed 49 = ROSTER4 (both loaded and resaved natively on 2026-10-05). Bio edits, roster moves and free-agent changes written by Studio, and every NHL 08 write, are automated-tested only.

## 4. Boundaries

- No project folder, no game installation, no deployment. Backups: `<app data>/save-editor-backups/`, one per session, taken before the first overwrite (even an unedited repack is not guaranteed byte-identical).
- Refused for Save Editor sessions on the server: dictionary attach/rename/retext/growth, settings apply, archive-member apply. The bundled dictionary is attached read-only for name resolution.
- UI hides: logos, arena asset/art cards, league imagery and commentary, dictionary rename panels, Game Settings, Historical, Legends, Hockey Shop, Assets. Uniforms keeps its database jersey-registry editing (textures need an installation).

## 5. Automated evidence (12 Oct, Windows, debug profile)

| Check | Result |
|---|---|
| `studio-domain` `save_identify` unit (path hints) and `tests/save_identify.rs` (anchor matrix, open policy) | 2/2 pass |
| `studio-domain` `table_overlay` unit (byte-exact round trip; save-table edit survives strip; frozen edit detected; trailer kept) | 2/2 pass |
| `studio-api` `tests/save_editor.rs`: round trips (identify → open → bio jersey + last-name edit → save twice → one backup equal to the original → reopen, checksums valid, values present) on PS3 v4 NHL 14, PS3 v3 NHL 09, Xbox v2 NHL 12, PS4 NHL 17, PS4 NHL 18 3-on-3 (acknowledged), bare NHL Legacy DB; franchise/season saves (NHL 15 PS4, NHL 10 Xbox, NHL 11 PS3) open read-only, player edit refused, save leaves their database unchanged; refusals (change list, wrong game, settings save, non-database, dictionary write); manual metadata mismatch refused | 9/9 pass |
| Regression, `studio-domain`: lib, `in_place_save`, `pending`, `session`, `txn_coordination`, `view`, `workspace` | 291 (6 pre-existing ignores), 7, 4, 11, 3, 16, 13 — all pass |
| Regression, `studio-api`: `bindings` unit (incl. live drift check), `command_execution_context` | 25/25, 1/1 |
| Bindings | `export-bindings --write`: 4 new files, 356 unchanged |
| Frontend: `tsc --noEmit`, `npm run build`, `SaveEditor.test.tsx` (3), `Start.test.tsx` (3), `LeagueDetail.test.tsx` (mock updated) | pass |

Pre-existing failures observed, not caused by this work (files untouched, or behaviour this change does not reach) and not repaired here:

- `studio-domain/tests/save_matrix.rs`: committed `docs/audits/data/save-matrix.csv` still records the NHL 15 PS4 roster save as unsupported; the working tree's chunkzip support now round-trips it. Generated evidence was not regenerated (CLAUDE.md evidence rule).
- `studio-api/tests/registry.rs::free_agent_pool_through_registry`: free-agent pool 340 vs 542 team-less players in the master list.
- Frontend: `IsoOpenPanel`, `Ps3DiscPanel`, `Ps4PackagePanel` (one extraction case each), `TeamLogoCard` (text "research unknown" not found), `About` (Vite denies the vendored extract-xiso licence import).
- `studio-api/tests/onb003_corrections.rs` does not compile (`PackageEdits` initializer lacks `career_removals`).
- `bindings::tests::export_root_list_is_unique_and_non_empty` was already 7 roots behind; updated to 204 (7 earlier + 3 Save Editor) with the note in its message.

Probes kept for re-running the format findings: `crates/studio-domain/examples/save_table_overlap.rs`, `save_tdb_scan.rs`, `save_records_scan.rs`, `ps4_trailer.rs` (read-only over the anchors).

## 6. Open items

- **PS4 trailer CRC (existing path, not changed here):** Mod Studio's PS4 roster-save path keeps the 18–20-byte trailer inside the working bytes, and `tdb-core` writes the EOF CRC at `buffer length − 4`. On NHL 17–19 that lands in the trailer. Needs confirmation and a fix in the shared engine. The Save Editor's supplemented sessions keep the trailer outside the TDB.
- Mode-save tables (standings, schedules played, finances) have no curated screens yet; they are editable only where an existing screen reaches them.
- Franchise rosters: the anchors show no roster inside mode saves. Where the game takes a franchise's roster from (main DB or an active roster save) is unverified, so the read-only teams/players in a franchise session are the vanilla defaults, not necessarily what the franchise uses.
- Split PS3 saves, NHL 20 bundled definitions and PS4 trailer size rewriting.
- ~~NHL 08 release-to-free-agent fails ("no player with wBIz 0")~~ fixed 13 Oct, see §8.

## 7. User checklist (native verification still required)

1. Launch screen → **Save Editor** → Choose file… → pick a PS3 `SYS-DATA` roster save. Expect the game, *PlayStation 3*, *Roster save*, and a note that other tables are read-only.
2. Open → Teams: the header reads *Save Editor · NHL xx · PlayStation 3 · Roster save*; only Save Data and Validation sections appear.
3. Change a player's jersey number, Save. Expect a backup notice naming a file under `save-editor-backups`. Load the save in the game/emulator and confirm the number.
4. Open an Xbox 360 roster save outside a Xenia folder: choose the platform when asked.
5. Open a Franchise/Season save: acknowledge, then confirm player fields are read-only and editing one is refused; Save and confirm the game still loads the save.
6. Open a settings save: confirm the refusal text.
7. Open an NHL 07 or NHL 08 roster save: Teams/Players show the game's rosters with the save's changes. Change a jersey number and move a player, Save, and load the roster in the game. Bio fields the save cannot record show read-only.

## 8. Tester build/install fixes (addendum, 13 October 2026)

Reported by a tester (package `NHL All Stars 08 (2)_ps3_nhl08.nhlmod`, built from an **imported** NHL 08 PS3 database, not the vanilla anchor).

1. **NHL 08 install: "more than one row of exhibitionrostertable … has key = 701".** The tester's own database already carries key 701 on two roster rows (624 and 625; the package's before values show it). The installer resolved rows by key only. Now, when a key matches several rows, the row whose bytes equal the author's original row (the fingerprint every cell already carries) is chosen; still ambiguous → refused as before. Installer-side: the existing package installs without a rebuild. Test: `package_row_identity::duplicate_roster_keys_resolve_by_row_fingerprint`.
2. **NHL 07 build: "stockcareer…stats: 6 row(s) removed … cannot be expressed".** Career-row removal packaging required every surviving row to be byte-unchanged; an edit to a surviving row blocked the build. The builder now aligns saved rows onto the original rows (closest match), records the removals plus complete new bytes for edited survivors (`CareerRemovalOp.rewritten`, edits schema **6**; schema 5 packages still install), and refuses only when a survivor's key would change. The installer writes the rewrites field by field, requires the row to equal the author's bytes, then removes. Test: `package_row_identity::nhl07_career_removal_with_an_edited_survivor_packages_and_installs` (installed database byte-identical to the author's, restore byte-identical to the original).
3. **NHL 08 release / sign / move.** NHL 08 has an explicit free-agent list, keyed roster blocks with no empty-slot convention, and index key 0 is a real player (game id 0, a shipped free agent). Releasing wrote pointer 0 into the slot (which on NHL 08 names that player) and rewriting the free-agent list rejected key 0. Now, where index key 0 is some real player's only key (measured over every anchor: NHL 08 alone; NHL 09–14 have no key-0 row, NHL 15+ key-0 rows belong to players with another key), a membership leaves by removing its roster row (ordered) and arrives by inserting a row at the team block's end with a minted key, the delete / insert NHL 08's own roster saves declare for this table; the free-agent list accepts key 0 when it is the player's only key. NHL 07 and NHL 09+ paths unchanged. Automated only; not game-verified. `tdb-edit` `field_ops` `release_then_sign_round_trips_a_player` and `fa_ops_reject_bad_args` (NHL Legacy) fail in the pre-existing, uncommitted free-agent-list guard at the top of `release_to_fa`, before this change's code; not changed here.
