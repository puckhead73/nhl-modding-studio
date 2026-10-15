# Receipt 03 — research handoffs consumed; packed archives as patch containers; NHL 12 PS3 image

Overnight developer lane, 18 September 2026. Status: **implementation complete / automated
verified / Codex review pending.** Nothing launched, nothing live written, no original modified.

## Plain-language summary

The research lane traced the league code → dictionary-key table out of every Xbox 360 executable
(NHL 09–15 and Legacy have a real pointer table; NHL 07/08 only an ordered literal block) and
confirmed all ten Xbox titles are installed with anchor-identical files. Studio now carries that
evidence exactly as measured: the vocabularies name the Xbox executables as their witnesses, so the
Leagues page shows `traced` on those Xbox targets and `by-reference` on their PS3 twins (never
`traced` on a target whose executable was not read); NHL 09's code 4 now re-points the key the
game actually uses (`DEL`, not `TXT_LEAGUE_DEL` — a rename of the wrong key would have changed
nothing on screen); three raw codes have names (NHL 09/10 "Historic Teams", NHL 10 "Prospect
Teams"); and the codes the executables know but no stock team uses (CHL, United States, Russia,
Rest Of World, HUT, Legend Teams on Legacy) are catalogued as `enum` entries. A test now fails if
any vocabulary key ever drifts from the traced tables. Separately, the disc-image route from
receipt 02 was widened to plain archive files: an extracted tree's `cacheboot.big` / `cache.big`
/ `fe.big` can be opened, staged, edited and patched in place by the same range-patch engine, so
the packed tree route needs no 300 MB candidate copies. Finally, the NHL 12 PS3 blocker is
resolved as a finding rather than a route: the image RPCS3 registers is an encrypted redump ISO
(data regions unreadable) and its disc key is not on the machine, so no in-image edit is possible
and the only path is registering the extracted folder — a user action.

## Handoffs consumed (identity)

| Handoff | File | Consumed into |
|---|---|---|
| `p1.1-xbox360-league-code-key-tables` (ready 05:05) | `docs/research/overnight-2026-09-18/10-phase1-league-identity-xbox360.md`, `data/xbox360-league-code-key-tables.json` | `defs/leagues/gen7-{nhl09,nhl10,nhl11,nhl12-15,nhllegacy,nhl08}.toml`; `LabelConfidence::Enum`; tests |
| `r0-xbox360-source-inventory` (ready 05:25) | `01-xbox360-source-inventory.md` + `data/xbox360-*.{json,csv}` | disposition rows below; Recompiled `_compiled` disposition; no code change needed (Studio's `title_ids` already matched) |

## Changes

- **Vocabularies.** `[key_witness]` added to NHL 09, 10, 11, 12–15 (`traced_on` = each title's
  Xbox executable, VA and image hash in `source`); every entry whose key equals the traced key
  (case-insensitively — the game's lookup is by case-folded hash; `elitserien` vs `ELITSERIEN`)
  flipped `hypothesis → traced`; NHL 09 code 4 `TXT_LEAGUE_DEL → DEL`; new `confidence = "enum"`
  entries: NHL 09 code 8, NHL 10 codes 8–9, NHL 11 code 13, NHL 12–15 codes 15–19, Legacy 16–21.
  NHL 08 keeps `hypothesis` with the literal-block note; NHL 07 unchanged (label source still
  unresolved; not proven absent).
- **`LabelConfidence::Enum`** ("enum"): present in the executable's league enum with no team row
  on the anchors; label = the traced key's dictionary text.
- **Tests.** `leagues::vocabulary_keys_match_the_traced_xbox360_tables` (every pointer-table
  title: witness present iff traced, every traced code catalogued, keys equal, `traced` claimed;
  literal-block titles never `traced`; 100+ bindings checked); `key_evidence_is_scoped…` now
  asserts NHL 14 traced on Xbox / by-reference on PS3 and NHL 08 never traced;
  `layout_specific_codes_resolve_per_title` asserts the new NHL 09/10 labels and the `DEL` key;
  `dictionary_rename.rs` NHL 12 PS3 → `by-reference` with the witness named.
- **Archive-file containers.** `xiso_source::Container { Image | ArchiveFile }` — a file that is
  not an XDVDFS image but starts with an `EB`/`BIGF` magic is a container of one member (itself,
  `[0, len)`); `locate_databases`, `read_located`, `plan_write_back`, `iso_patch::plan` and the
  staging/apply/restore flow all run on it unchanged. `tdb_xiso::MemberReader::over_file`. Test:
  `xiso_source::container_tests`. UI wording widened (open panel accepts `.iso` or `.big`;
  Pending-edits panel titled "Disc image / packed archive").
- **Real-file check (read-only):** `NHL 14\NHL 14\cacheboot.big` (X: tree) stages, re-points
  and plans `fe/loc/nhl_eng_us.db [1898912, 3602116) e5aff828… → 3af0b8a5…` — the same
  replacement identity as the ISO plan, at the archive's own offset (`tree-archive-plan-nhl14.txt`);
  file size and mtime unchanged.

## NHL 12 PS3 (observed tonight, read-only)

`H:\Emulators\RPCS3\config\games.yml` maps `BLUS30791` to
`X:\…\Sony Playstation 3\NHL 12 (USA) (En,Fr).iso` (7 826 046 976 B). The image has a PS3 disc
header (`PlayStation3` at 0x800, 4 regions) and a plain ISO9660 tree (`PS3_DISC.SFB`,
`PS3_GAME/USRDIR/{cache.big, boot.big, data0.big, …}` — the database is packed, as on Xbox NHL 12),
but every archive member's bytes are ciphertext (magics are noise), i.e. a redump-style
**encrypted** image; the only disc key present is `dkey/NHL 08 (USA).dkey`. Without the NHL 12
key the image cannot be read or patched by Studio, and RPCS3 cannot boot it either. Disposition:
**no image route**; the extracted folder `X:\…\NHL 12\[BLUS 30791] NHL® 12\` (its DB and
dictionary equal the anchors per the batch) must be registered in RPCS3 by the user; the batch's
loose dictionary candidate (`a4e710dc…`) then applies as on the other PS3 rows. Studio was not
changed for this (an ISO9660 reader would only serve a decrypted image, which does not exist here).

## Disposition of the completion record's Xbox rows (consuming R0)

Rows 12/14/16/18/20 (NHL 15/14/13/12/11 Xbox 360), 22/24/26 (NHL 10/09/08) and 27 (NHL 07):
**installed** (tree + ISO for 10–Legacy); DB/dictionary = anchors everywhere; binding **traced**
on 09–15 and Legacy; routes available: ISO range patch (receipt 02), tree archive range patch
(this receipt), loose file (tree extraction; existing in-place save). Which of the three the
running emulator reads is the runtime question for tomorrow (R0 §4: Xenia `cache:` shadowing for
NHL 09; loose-vs-archive precedence untested; ISO-vs-folder launch not recorded). NHL 07 stays
**blocked** for the rename (label source unresolved). Recompiled: `_compiled` absent; archive
route on `H:\…\NHL Legacy Decomp Test\game\cacheboot.big` (= Vanilla tree's `b558b4f6…`) via the
container route.

## Checks

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test leagues` | 9 passed |
| `cargo test -p studio-domain --lib iso` (xiso_source + iso_patch + iso_staging + container) | 9 passed, 1 ignored |
| `cargo test -p studio-api --test dictionary_rename --test iso_route` | 4 + 1 passed, 1 ignored |
| Tree-archive plan-only on the real NHL 14 `cacheboot.big` | ok, file untouched |
| `tsc --noEmit`; `vitest run` | clean; 9 files, 68 tests (incl. `DiscImagePanel.test.tsx`, 5) |
| Checkpoint 2 (before this receipt's edits): `cargo test -p studio-domain -p studio-api -p tdb-xiso` | 384 passed, 0 failed, 11 ignored |

## Changed paths

`defs/leagues/gen7-{nhl08,nhl09,nhl10,nhl11,nhl12-15,nhllegacy}.toml`,
`crates/studio-domain/src/{leagues.rs,xiso_source.rs,iso_patch.rs}`, `crates/tdb-xiso/src/lib.rs`,
`crates/studio-domain/tests/leagues.rs`, `crates/studio-api/tests/dictionary_rename.rs`,
`app/src/components/{IsoOpenPanel.tsx,DiscImagePanel.tsx,DiscImagePanel.test.tsx}`,
`local/runs/overnight-development-2026-09-18/tree-archive-plan-nhl14.txt`.
