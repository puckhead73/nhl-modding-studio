# Handoff P1.1-X — league code → dictionary-key bindings traced on every Xbox 360 executable

Research lane, overnight 2026-09-18. Claim `p1.1-xbox360-league-code-key-tables`. Consumer: developer
(Phase 1.1 identity, LOC-W1 rename route, `defs/leagues/*.toml` `key_evidence`, F5 platform-specific evidence).
Machine-readable: [`data/xbox360-league-code-key-tables.json`](data/xbox360-league-code-key-tables.json).
Probes: `local/runs/overnight-research-2026-09-18/probes/{xex_image.py,ptr_table.py,league_tables.py,tdb.py}`.

## Plain-language summary

All ten Xbox 360 executables under `X:\Games\Emulation\Roms\XBOX360` were decrypted and decompressed
tonight (retail XEX2 key; LZX decoder validated byte-for-byte against the user's own decompressed NHL
Legacy image, SHA-256 `966acda4…`). In **NHL 09, 10, 11, 12, 13, 14, 15 and Legacy** the league
code → dictionary-key list is a static pointer table in the executable, exactly like the Legacy trace
the batch already had. Every key was then resolved in the title's own `nhl_eng_us.db` (each X: copy is
byte-identical to the repository anchor). Result: the binding class for those eight Xbox targets can move
from *hypothesis* to **traced**, three raw codes are now identified (NHL 09 code 8, NHL 10 codes 8 and 9),
and one concrete exception was found (NHL 09 uses key `DEL`, not `TXT_LEAGUE_DEL`). NHL 07 and NHL 08
have no pointer table; their keys exist as an ordered literal block behind the `FE::CLeagueIonAdapter`
class name, and NHL 07's three `CHTM_LEAGUE_*` keys are not in its dictionary at all.

Evidence level: **observed** on the named executables and dictionaries; **inferred** where stated.
Nothing here is game-verified; which screens consume the table remains untraced on every title.

## 1. Tables (observed)

`code` is the `kOtt` league code (`xMjj` on the gen7 titles); `text` is the title's own English dictionary
text; *resolution* `exact` = row keyed exactly, `hash` = only a case-folded `hashid` match exists (the
game's lookup is by hash; Studio's P4 fallback already handles this).

| Title | Table | Codes → keys (text) |
|---|---|---|
| **NHL 09** `45410872` | pointer `0x83054ce0` (10) | 0 NHL · 1 AHL(`ahl`) · 2 ELITSERIEN(`elitserien` "Elitserien") · 3 SMLIIGA "SM-Liiga" · **4 `DEL`** "DEL" · 5 TXT_LEAGUE_O2_EXTRALIGA "O2 Extraliga" · 6 TXT_LEAGUE_SUPER_LEAGUE "Superliga" · 7 NATIONAL(`National`) · **8 HISTORIC_TEAMS "Historic Team"** · 9 CUSTOM_TEAMS(`Custom_Teams`) |
| **NHL 10** `454108BA` | pointer `0x83333c3c` (11) | 0 NHL "NHL®" · 1 AHL · 2 elitserien(`ELITSERIEN`) · 3 SMLIIGA "SM-LIIGA" · 4 TXT_LEAGUE_DEL · 5 TXT_LEAGUE_O2_EXTRALIGA · 6 TXT_LEAGUE_SWISS_LEAGUE "NATIONAL LEAGUE" · 7 NATIONAL · **8 HISTORIC_TEAMS "HISTORIC TEAM"** · **9 PROSPECT_TEAMS "PROSPECT TEAMS"** · 10 CUSTOM_TEAMS |
| **NHL 11** `454108FD` | pointer `0x834a28a0` (14) | 0–7 as NHL 10 (NATIONAL at 7) · 8 OHL · 9 QMJHL · 10 WHL · 11 PROSPECT_TEAMS · 12 CUSTOM_TEAMS · **13 CHL** |
| **NHL 12** `45410964` | pointer `0x83705f18` (20) | 0–11 as NHL 11 · 12 TXT_LEGEND_TEAMS "LEGEND TEAM" · 13 CUSTOM_TEAMS · 14 NHL (Winter Classic rows) · 15 CHL · 16 UnitedStates · 17 Russia · 18 TXT_ROW_LEAGUE "REST OF WORLD" · 19 HUT |
| **NHL 13** `4541099B` | pointer `0x837eea88` (20) | identical key list to NHL 12 (texts mixed-case: "Legend Teams", "Rest Of World") |
| **NHL 14** `454109BE` | pointer `0x8398fc20` (20) | identical to NHL 12/13 (`elitserien` → "SHL") |
| **NHL 15** `454109DC` | pointer `0x839bbc30` (20) | identical to NHL 12/13/14 (`SMLIIGA` → "Liiga") |
| **NHL Legacy** `454109EC` | pointer `0x839b1230` (22) | 0–6 · **7 TXT_LEAGUE_NORWAY** · 8 NATIONAL · 9 OHL · 10 QMJHL · 11 WHL · 12 PROSPECT_TEAMS · 13 CUSTOM_TEAMS · 14 NHL · 15 EA_SPORTS_HOCKEY_LEAGUE · 16 CHL · 17 UnitedStates · 18 Russia · 19 TXT_ROW_LEAGUE · 20 HUT · 21 TXT_LEGEND_TEAMS — **reproduces the batch's trace exactly** (the batch quoted file offset `0x19aa64c`; that is the raw-PE offset, VA `0x839b1230` in loaded layout) |
| **NHL 08** `45410820` | literal block `0x82072dec` (6) | NHL · AHL(`ahl`) · ELITSERIEN(`elitserien`) · SMLIIGA "SMLiiga" · NATIONAL(`National`) · `CUSTOM TEAMS`(`Custom Teams`) — **order only** (matches Studio codes 0–5; reference mechanism not traced) |
| **NHL 07** `454107F3` | literal block `0x820b3650` (6) | NHL · NATIONAL(`National`) · CHTM_LEAGUE_SWEDISH · CHTM_LEAGUE_FINNISH · CHTM_LEAGUE_GERMAN · `CUSTOM TEAMS`(`Custom Teams`) — order matches Studio gen6 codes 0–4 (+ custom); **CHTM_LEAGUE_\* are not in `nhl_eng_us.db` (3 952 rows) by key or hash** |

VA = virtual address in the loaded image (base `0x82000000`). The table sits in `.data` and points into
`.rdata`; in the raw PE file layout (what `xextool`-style dumps produce) `.data` is shifted (Legacy:
raw `0x1989400` ↔ VA `0x1990000`), which is why the batch's file offset and tonight's VA differ.

The same executables also carry a *second*, shorter league list (NHL, AHL, elitserien, SMLIIGA, DEL, O2,
SWISS, [OHL, QMJHL, WHL,] CUSTOM …) followed by `YES/MINIMAL/NO` and difficulty names — a different enum
(evidently a mode/settings league selector), **not** the `kOtt` code table. Do not merge them.

## 2. Family fingerprint and exceptions

- **NHL 12 / 13 / 14 / 15 Xbox 360 share one identical 20-key table** (same order, same keys); NHL 14 and 15
  additionally share the trophy/division enum that follows it. This corroborates the `gen7-nhl12-15`
  vocabulary and extends it: codes **15 CHL, 16 UnitedStates, 17 Russia, 18 TXT_ROW_LEAGUE, 19 HUT** exist in
  the executable enum even though no anchor team row uses them (Studio's toml stops at 14). Whether a team
  assigned one of these codes is accepted by the game is a runtime question (queue for tomorrow's cards).
- **NHL 11** adds code **13 CHL** beyond Studio's 0–12.
- **NHL 09 exception:** the executable key for code 4 is **`DEL`**, not `TXT_LEAGUE_DEL`. Both keys exist in
  the NHL 09 dictionaries (Xbox anchor, PS3 anchor-labelled file and the PS3 disc private fixture), both with
  text "DEL" today. A LOC-W1 re-point of `TXT_LEAGUE_DEL` would therefore **not** change the on-screen label on
  NHL 09; the candidate must re-point `DEL`. `defs/leagues/gen7-nhl09.toml` entry 4 should carry
  `dictionary_key = "DEL"`. Also NHL 09 `TXT_LEAGUE_SWISS_LEAGUE` does not exist (no Swiss league that year —
  consistent with the batch).
- **NHL 09 code 8 = HISTORIC_TEAMS** ("Historic Team"); **NHL 10 code 8 = HISTORIC_TEAMS**, **code 9 =
  PROSPECT_TEAMS** — the three raw codes flagged in the batch (`CEN`, `NAJ/WPR`) now have keys. The
  batch's lead "9 = Top Prospects pair" is confirmed for NHL 10.
- **Legacy**: identical to the batch trace; nothing new except the VA correction above.
- **NHL 07/08**: no pointer table. The keys are the literal block after `FE::CLeagueIonAdapter` in `.rdata`
  (07: `0x820b3650`; 08: `0x82072dec`). No `lis/addi`, `lis/ori`, `r30`-relative, data-pointer or
  precomputed-hash reference to those literals was found in either image (all four searched), so the
  consuming code path is untraced — treat the order as **inferred**. For NHL 08 the order equals the
  DB code order, and every key resolves in its dictionary (by hash for `AHL/ELITSERIEN/NATIONAL/CUSTOM TEAMS`).
  For NHL 07 the three `CHTM_LEAGUE_*` keys are absent from `nhl_eng_us.db`; a plain-text grep of the whole
  `fe/ion` ION tree and `db/` found no `CHTM` / `Elitserien` text either (ION `.big` members may be
  RefPack-compressed — not decoded tonight). **NHL 07 label source stays unresolved**; the candidates are
  now narrower: the key itself displayed raw, or a resource inside the ION screen packages. Scope of this
  negative finding (R1 item 3): it covers exactly the inspected data — the loose `fe/loc/nhl_eng_us.db`
  (= anchor, `5d3c0b01…`, 3 952 rows, by key and by `loc_hash`) and a plain-text grep of the loose `fe/ion`
  and `db/` trees of the X: NHL 07 (USA) copy. Not inspected: RefPack-compressed members inside `data0/data1.big`
  and the ION `.big` packages, other localization languages, and the executable's own resource section.

## 3. What development can do now (safe candidates)

1. **Promote evidence class per target.** `key_evidence = "traced"` is now justified for Xbox 360 NHL 09, 10,
   11, 12, 13, 14, 15 (and Legacy, already). PS3 rows stay *hypothesis* (PS3 executables are encrypted
   `EBOOT.BIN`; the PS3 databases and dictionaries are byte-identical to the Xbox ones for 10–Legacy, which
   raises the prior but is not a trace). Recompiled inherits the Legacy Xbox trace by identity of
   `default.xex` (`d818eb15…` in all three H: trees and the X: tree — see the source inventory).
2. **Vocabulary corrections** (`defs/leagues`): NHL 09 code 4 key `DEL`; NHL 09 code 8 `HISTORIC_TEAMS`
   "Historic Team"; NHL 10 code 8 `HISTORIC_TEAMS` "Historic Team", code 9 `PROSPECT_TEAMS` "Prospect Teams";
   NHL 11 code 13 `CHL`; NHL 12–15 codes 15–19 as above (mark "enum present, no stock team").
3. **LOC-W1 candidates** for the Xbox rows can now be minted from the X: trees (loose `fe/loc/nhl_eng_us.db`
   is present in every X: title root — but note it is an *extraction*; the disc/ISO ships it only inside
   `.big` archives for NHL 11+; see the source inventory for what Xenia actually loads).
4. **Test hook:** `data/xbox360-league-code-key-tables.json` can back an automated assertion that each
   `defs/leagues/*.toml` entry's `dictionary_key` matches the traced key for its Xbox target.

## 4. Not established / next discriminating checks

- Which screens read the table (team select tile vs. roster header) — runtime, tomorrow's cards.
- PS3 executables: untraced (would need a decrypted `EBOOT.BIN`; none on this machine — RPCS3's cache holds
  only compiled objects, per the batch).
- NHL 07 `CHTM_LEAGUE_*` label source: decode the ION screen `.big` members (BIGF, RefPack) and grep; or
  check whether the game falls back to printing the key when the dictionary misses.
- NHL 07/08 consuming code path for the literal block.

## 5. Reproduction

```
python local/runs/overnight-research-2026-09-18/probes/xex_image.py "<title root>\default.xex" scratch/xex/<title>.bin
python local/runs/overnight-research-2026-09-18/probes/ptr_table.py scratch/xex/<title>.bin 82000000 SMLIIGA --window 4 30
python local/runs/overnight-research-2026-09-18/probes/league_tables.py    # regenerates the JSON
```
Image hashes (virtual layout): in the JSON (`image_sha256`). Scratch images are disposable (~200 MB) and are
deleted at the lane's next cleanup boundary; they regenerate in ~30 s each.
