# Handoff R5 — targeted Phase 1 gaps: NHL 07 league-label source (scoped negative), NHL 07 texlib variant (closed), matrix lag notes

Research lane, 19 September 2026 (queue entry R5 of the [overnight research brief](../../plans/overnight-research-2026-09-19.md)).
Claim `research-r5-targeted-gaps`. Consumer: developer (coverage-matrix cells "X360 07 — blocked (labels not in
dictionary)" in 1.1/1.3, the NHL 07 preview cells in 1.4, and the 1.3 "locales … not inventoried" wording).
Bounded by the brief: one hypothesis per item, one concrete source set, no repeated blind searches for ids 77/78.

Machine-readable: [`data/r5-nhl07-label-grep.json`](data/r5-nhl07-label-grep.json) (every hit of the key/label
patterns across the NHL 07 Xbox 360 sources with member path, offset and a short context),
[`data/r5-nhl07-texlib-variant.json`](data/r5-nhl07-texlib-variant.json) (13 decoded NHL 07 texlibs with variant,
format, dims, slot). Probes: `nhl07_label_grep.py`, `rw4xb2_nhl07.py`. Preview:
[`previews/contact-nhl07-x360-league-families.png`](previews/contact-nhl07-x360-league-families.png).

## Plain-language summary

Two small Phase 1 questions got a bounded answer within their source set. (1) NHL 07's three league names
(`CHTM_LEAGUE_SWEDISH/FINNISH/GERMAN`) have **no text in the inspected sources** (rev. 2 wording, review F4 — rev. 1 said
"anywhere in the shipped data"): every archive member up to 8 MB and the loose trees were decompressed and searched
(18 968 members incl. two levels of nested packages; 32 top-level members above 8 MB were skipped — `gamedata/anim/anim.viv`
and 31 `rendering/stadium/stadium_*_container_0.rx2`; deeper nesting, other encodings than ASCII/UTF-16 and any
compression other than RefPack were not searched), the executable was decrypted and searched, and the English
dictionary was read row by row — only the keys themselves exist (in the executable's
literal block), and the only "Elitserien"/"Liiga" strings are table names in the database metadata. So Studio cannot
rename those leagues through a dictionary on NHL 07; what the game shows for them is a runtime question (raw key,
blank, or logo-only). (2) The NHL 07 texture container variant that the preview code could not read is now pinned:
some NHL 07 texlibs store the pixel slot with a zero tag and an absolute page offset; parsing that variant decodes the
13 league-family specimens measured (contact sheet attached) — rev. 2: not every texture on the title; coverage beyond
those 13 members is unmeasured. A third item is bookkeeping: the locale inventory
the matrix calls "not inventoried" already exists in research 13.

## 1. NHL 07 league-label source (scoped negative; hypothesis: a data member carries the text)

Source set (observed, all read-only): `X:\…\NHL 07\NHL 07 (USA)\` — `data0.big … data5.big`, `boot.big`
(every member ≤ 8 MB: 18 968 members, RefPack-decoded when wrapped, nested BIGF members walked two levels),
loose `fe/`, `db/`, `gamedata/`, `attribdb/`, `boot/`, `debug/` trees; `default.xex` decrypted with the
2026-09-18 `xex_image.py` (image SHA-256 `403a69a7…`, deleted after the search); `fe/loc/nhl_eng_us.db`
(3 952 rows, read with the research TDB reader, keys and texts).

Patterns: `CHTM_LEAGUE`, `CHTM`, `LEAGUE_SWEDISH/FINNISH/GERMAN`, `Elitserien`, `ELITSERIEN`, `SM-liiga`,
`Liiga`, `Bundesliga`, `Swedish`, `Finnish`, `German` — ASCII and UTF-16.

Result: **no label text in the inspected set** (bounds: members ≤ 8 MB, nesting two levels, RefPack only, ASCII/UTF-16
patterns; the 32 members > 8 MB listed in the summary were not searched). Hits: the three keys in the executable's `FE::CLeagueIonAdapter` literal block
(`0x820b3678/8c/a0`, as handoff 10); `Elitserien`/`Liiga` only as the schedule-table names in
`db/NHL07NG-meta.xml` (`intlelitserienschedule`, `intlsmliigaschedule`); `German(y)` only as country text (player
origin lists, city strings in the DB); the three `CHTM` hits in art members (`jersey_0_51_1`, `p151`,
`teamlogoswide/t18`) are random byte coincidences inside textures. The dictionary holds `NHL`, `National`,
`International`, `Custom Teams` as literal keys/texts and nothing for the three European leagues. No ION screen
package references the `CHTM_LEAGUE_*` keys either (the keys appear nowhere outside the executable).

Reading (inferred): the NHL 07 front end receives the league `NAME` attribute from the adapter, looks it up in the
dictionary, misses, and either shows the raw key, shows nothing, or uses the league logo art alone
(`leaguelogos/l0–l6`, decoded in §2). Which of the three happens is **unknown** → card R5-07-1. For Studio: the
"blocked — labels not in dictionary" cells stay blocked with this evidence; a rename on NHL 07 would require
either a dictionary **key insertion** (class-B growth, refused per handoff 20) or an executable literal edit
(out of scope for this run per the brief).

Not inspected: other NHL 07 locale dictionaries (none ship — `localization-locales.json` lists `eng_us` only),
the NHL 07 PS3 research copy.

## 2. NHL 07 `RW4xb2` texlib variant ("tag-0 absolute slot") — specimen gap closed (observed)

Handoff 16 noted that NHL 07's `small`/`wide` league-logo texlibs "use a layout `rx2_preview.parse` does not read".
Measured on 13 NHL 07 texlibs (`leaguelogos l0/l1`, `leaguelogossmall l0`, `leaguelogoswide l0/l1`,
`divisionlogos d0–d2`, `trophies t0–t3`, `flagssmall f0`): two variants coexist inside one title —

| Variant | File-table data entry | `hdr_size` @0x44 | Pixel slot | Seen on |
|---|---|---|---|---|
| standard (= NHL 08–11) | tag `00 01 00 31`, offset relative to `hdr_size` | 0x1000 | `hdr_size + 0` | `leaguelogos`, `leaguelogoswide l1`, `divisionlogos d2`, `trophies` |
| **NHL 07 tag-0** | tag `00 00 00 00`, words `[0x1000][0][size][0x1000][0]`, offset **absolute** | = file-table end (e.g. 0x9030) | 0x1000 | `leaguelogossmall`, `leaguelogoswide l0`, `divisionlogos d0/d1`, `flagssmall` |

The descriptor (tag `00 02 00 03`, 0x38 bytes at 0x160) is unchanged (format byte at +35, height/width at +37..+39),
and the Xbox untile/swap rules produce correct images for both variants (`rw4xb2_nhl07.py`; contact sheet: NHL
crest, globe, NHL wordmarks, West/East conference shields, four trophies, Canadian flag). Studio's `tdb-rx2::texlib`
reader should accept a tag-0 slot whose `ints[3] == 0x1000` and treat `ints[0]` as absolute; the writer for
same-dimension replacement writes the pixel bytes back at that absolute offset (the slot size and the descriptor are
unchanged). Formats/dims for the NHL 07 league families: `leaguelogos` 100×104 ARGB8; `small` 128×64; `wide` 512×248;
`divisionlogos` 560×408 / 524×408 / 496×448; `trophies` 408×200 … 468×344; `flagssmall` 128×96.

## 3. Matrix lag (bookkeeping, no new research)

- 1.3 "Locales other than `eng_us` … research unknown (not inventoried)": the inventory exists —
  `docs/research/overnight-2026-09-18/data/localization-locales.json` (per title/platform: which `nhl_*_*.db` exist,
  loose vs `cache.big`), `localization-capacity-xbox360-locales.json`, `localization-capacity-ps3.json`, handoff 13
  §2. Xbox: 07/08 `eng_us` only; 09 seven locales; 10/11 `eng_us`+`fre_fr`; 12–Legacy eight (`lng_lg` included).
  PS3: 08 `eng_us`; 09–11 + `fre_fr`; 12–Legacy eight, loose + `cache.big` copies. The cell can move to
  "not impl — inventory in research 13; same route as `eng_us` by identity (per-locale alphabets differ, 13 §2)".
- 1.4 NHL 07 preview cells: with §2 the RW4 preview *can* cover NHL 07 once the reader accepts the tag-0 slot (a
  developer change); the `data0.big` packages listed in `r5-nhl07-texlib-variant.json` are candidate fixtures. Rev. 2:
  a located fixture does not make the cell "tested" — it stays `impl, test missing` (or `not impl` for the tag-0 reader)
  until a named developer test executes against it.
- P1-C "NHL 15's patch DB untested": the private fixture `patch_inxp00.big:nhl_game.db` (`abe36f58…`) is already in
  the developer's retained artifacts (receipt STATUS) — a developer test, not research. Rev. 2: research availability
  of the fixture is recorded here; the matrix cell changes only when the developer's named test runs.

## 4. Card (later)

- **R5-07-1 (NHL 07 Xbox 360, observe only):** open the team-select / league filter screens and note how the
  Swedish, Finnish and German leagues are labelled — raw `CHTM_LEAGUE_*` text, blank, or logo only. Determines
  whether a future rename would need a key insertion (text shown) or nothing (logo only).

## 5. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-19/probes/nhl07_label_grep.py <out.json>      # ~25 min (Python RefPack)
python local/runs/overnight-research-2026-09-19/probes/rw4xb2_nhl07.py <package.big> <out.png>
```
Executed: the grep over 18 968 members + loose trees + raw `default.xex`; the XEX decrypt + grep (image deleted);
the dictionary row scan; 13 texlib decodes and the contact sheet. **Not executed:** launches, writes. Scratch
`local/runs/…/r5/` deleted after copying the two JSON files to `data/`.
