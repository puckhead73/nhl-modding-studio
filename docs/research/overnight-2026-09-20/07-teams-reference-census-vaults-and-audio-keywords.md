# Handoff R14 — Teams: the identifier reference census (what a team/arena identity edit drags along), the numbered stadium vaults associated per level, and the team/arena audio-id binding (`pbp_keywords.bin`)

Research lane, 20 September 2026 (Priority 2 of the overnight campaign: 2.1 / 2.2 / 2.5 / 2.7). Claim `research-r14-teams`. Consumer:
developer Phase 2 (identity edits with dependency disclosure; arena/vault association; team PBP audio reassignment). Builds on handoffs
[06](../overnight-2026-09-19/06-ps4-team-identity-and-presentation-map.md), [07](../overnight-2026-09-19/07-ps4-vault-collections-and-slug-binding.md)
(Q-R7-1), [01](01-ps4-jersey-triple-contract-and-r7-reconciliation.md), [02](02-snd10-audio-banks-and-league-cue-chain.md) and the
2026-09-18 handoffs [20](../overnight-2026-09-18/20-phase2-team-identity-and-asset-joins.md) / [23](../overnight-2026-09-18/23-phase2-team-localization-keys.md).
Nothing here is game-verified; nothing written; no launch; **no writer proposed**.

Machine-readable: [`data/r14-team-reference-census.json`](data/r14-team-reference-census.json) — (a) for Xbox 360 NHL 08 / 10 / Legacy,
PS3 Legacy, PS4 15 / 18 / 21 anchors + the real NHL 20 install DB: every field of every table whose name contains
team/arena/artid/rival/affil/club, with its value range and its coverage by each identifier key set (`exhibitionteams.artid` / `.index` /
`.arenaid`, `exhibitionarena.arenaartid`), flagged **ambiguous** when several key sets cover ≥ 95 % of the values; (b) for PS4 16–21:
every numbered sibling vault with size, collection count and the `ge_level` names its string pool contains. Probe:
`r14_team_reference_census.py` (≈ 3 min); [`data/r14-audio-keywords.json`](data/r14-audio-keywords.json) — §3's joins per PS4 title + PS3 NHL 15, probe `r14_audio_keywords.py` (≈ 1 min).

## Plain-language summary

Three Teams questions get evidence. First, which other records point at a team or an arena: the census lists, per title, every table
column that holds team art ids, team indexes or arena art ids — the columns a rename or re-id must update or at least disclose — and
it is honest about the limit of value-based detection: small integer columns (division numbers, strategy biases, 0/1 flags) are
"covered" by every key set and must not be read as references. Second, the per-stadium vault files whose names nobody could derive
(Q-R7-1): no hash or attribute produces the number (two more approaches failed tonight), but each vault's own string pool names its
level, so a static level → vault association exists by content for 75 of 81 NHL 21 vaults (36 / 39 on NHL 16) — an association, not a
parsed reference. Third, the team and arena "audio ids" in the database are resolved: they are keyword ids in the play-by-play keyword
table (`pbp_keywords.bin`, sample id → keyword pairs), 100 % on every PS4 title and on PS3 NHL 15 (whose table equals the PS4 one);
an AHL club's keyword tags exactly two half-second samples in NHL 21 — its name calls — while an NHL club's keyword tags hundreds of
recorded lines that mention it. So "which club name the commentary says" is a per-team number that can be pointed at another
club's keyword, and the samples behind it are located. Player audio ids do **not** follow that rule (only 12.5 % fall in the keyword
space; treating them as direct sample ids was not discriminating) — the player-name binding stays open.

## 1. Reference census (observed; `reference_census`)

| Title | Columns found | Covered by a key set | Unambiguous | Ambiguous (several key sets) | Uncovered |
|---|---:|---:|---:|---:|---:|
| Xbox NHL 08 / 10 | 21 / 18 | 17 / 14 | 10 / 8 | 7 / 6 | 4 / 4 |
| Xbox Legacy = PS3 Legacy | 49 | 45 | 13 | 32 | 4 |
| PS4 15 / 18 / 21 / 20 (install) | 51 / 63 / 78 / 76 | 47 / 53 / 68 / 67 | 12 / 4 / 5 / 5 | 35 / 49 / 63 / 62 | 4 / 10 / 10 / 9 |

Unambiguous references (they carry the team's **index** — the team id — or its **art id**), Legacy Xbox as the witness, PS4 21 where
different: `stockteamjerseys.teamid` → index (703 rows / 222 distinct; PS4 21: 1 792 / 433 — every team), `stockteamjerseys.teamartid`
→ artid (PS4 21: 95 % — the spare-kit art ids 255 / 1000–1004 are the rest, handoff 01), `exhibitionrostertable.team` → index (5 532
rows; PS4 21: 9 090), `exhibitionplayerbiotable.team` and `.juniorteam` → index, `stockgoalieequipment.team`, `stockteamequipmenttable.team`,
`stockretirednumbertable.team`, `stockawardstable.team` (+ `.teamartid` → artid on PS4), `customteamjerseys.teamid/teamartid`,
`customteamequipmenttable.team`, `exhibitionteams.artid` → artid (218 distinct on Legacy, 312 on PS4 21), `exhibitionarena.arenaartid` →
arena art id (52 / 62 distinct). `exhibitionteams.arenaid` is 187-distinct on Legacy (an arena index; 433-distinct = one arena per team on
PS4 21), `exhibitionteams.ahlaffiliate` / `.rivalteam` → team index (values inside the index range; also inside the arenaid range, so
flagged ambiguous by the census — the field names decide). **Ambiguous by construction:** `*strategies.team_bias*`, `*answersmeta.teammod*`,
`arenatype`, `teamtype`, `forceteamcolors`, `draftteam` / `proteam` / `origteamid` (0–31: NHL club numbers — a *third* id space, the NHL
club rank, not the global index), `stockcareer*stats.team` on PS4 21 (126–157: a block of 32 ids inside the index range — the historic
NHL club rows, inferred), `stockleagues.artid` (0–25 — league art ids, not team). **Uncovered (not team references):**
`exhibitionplayerbiotable.artid` / `stockawardstable.playerartid` (player art ids), `exhibitionarena.arenaaudioid` (§3), `customteams.*`
(400–501: the custom-team id block).

**Contract input (proposal):** a team identity edit that keeps `index` and `artid` is dependency-free at the database level (names,
abbreviations, colours, `locstring` keys per handoff 23 remain their own operations); changing `artid` drags `stockteamjerseys.teamartid`,
`stockawardstable.teamartid`, the vault packs of handoff 07 (`CenterLogo`, jersey `Team`) and the registry files (`teamlogos/…/t<artid>`);
changing `index` drags every "→ index" column above plus the schedule slots (rank by index, handoff 12) — refuse or cascade. The
ambiguous columns need per-field semantics before any cascade; the census gives the list to disclose, not to rewrite.

## 2. Numbered sibling vaults ↔ levels (observed; `sibling_vaults`)

| Title | Vaults | `ge_level` rows | Vaults naming exactly one level | naming several | naming none |
|---|---:|---:|---:|---:|---:|
| 15 | 0 | 33 | — | — | — |
| 16 / 17 / 18 / 19 / 20 / 21 | 39 / 52 / 63 / 67 / 79 / 81 | 41 / 79 / 65 / 70 / 74 / 76 | 36 / 49 / 58 / 62 / 75 / 75 | 2 / 2 / 2 / 3 / 3 / 4 | 1 / 1 / 3 / 2 / 1 / 2 |

E.g. NHL 21 vault `20823504` → `pittsburgh` / `custom_pittsburgh` (`lightrig_custom_pittsburgh_unique`, `custom_pittsburgh_entranceprop`, …),
`51689039` → `losangeles`, `540494844` → `vegas`. The vault number is **not**: any Int/enum/text attribute of the level record; `lookup2`
(seeds `0xABCDEF00`, 0), djb2, djb2-xor, FNV-1/1a, CRC-32 or ~CRC-32 of 12 name forms per level (0 hits); the level's collection key;
present as a string in `gccollections.bin`. Each numbered vault's `DepN` chunk carries its own 64-bit id (e.g. `ab1873ec8fd794f2`) and
file names; those ids do not occur in `gccollections.vlt/.bin`. **Obstacle recorded** (two more approaches failed; Q-R7-1 stays open);
the content association is usable for *reading* (which vault holds a stadium's light rigs / props / banners) and for disclosure, not
for deriving a new vault's name.

## 3. Team / arena audio ids = PBP keyword ids (observed)

`pbp_keywords.bin` (`0DWK`+`KETT` on PS4, `KWD0PS30` big-endian on PS3; NHL 15 PS4 and PS3 tables are the **same 26 109 pairs**) is a
flat list of `(sample id, keyword id)` pairs — a sample may carry several keywords (NHL 21: 28 309 samples tagged, 24 099 once, up to 6).

| Title | Pairs | Keywords | `exhibitionteams.audioid` values ⊆ keywords | AHL clubs: tagged samples per keyword |
|---|---:|---:|---|---|
| PS4 15 (= PS3 15) | 26 109 | 2 121 | 211 / 211 | 1–8 |
| PS4 16 / 17 / 18 / 19 | 55 576 / 56 410 / 57 273 / 57 388 | 3 052 / 3 111 / 3 195 / 3 195 | 212 / 246 / 283 / 295 — all 100 % | 2–73 (most 14–37) |
| PS4 21 | 34 078 | 5 363 | 292 / 292 | **exactly 2 on 26 of 31 clubs, 7 on 5** — e.g. Bakersfield (163) → samples 4699 / 6858 (0.49 / 0.52 s), Belleville (401) → 5125 / 7142 (0.53 / 0.56 s), in the name-call groups (DSETs 3687 / 3689 / 3699 / 3702 of `pbp_samples_inxp.sbr`) |
| PS4 20 | 91 (2 keywords) | — | 2 / 292 | **title difference, verified against the archives:** `inxp_40.big:…/speech/eng/pbp_keywords.bin` is 744 B stored = plain (91 pairs) on the base install, while the sentence data (521 KB), the sample bank (878 KB / 547 MB) and the sentence → sample chain are complete (the league sentences resolve as on 21); how NHL 20 tags club mentions is **unknown** (the base install may rely on a patch, none on this machine, or on another table) |

NHL clubs' keywords tag hundreds to thousands of samples (Boston = keyword 2: 2 150 samples of 3–11 s — recorded lines that mention the
club, co-tagged with generic keywords 4 / 13 / 5 / 14), i.e. the keyword is "this sample says this club", used by the sentence engine to
pick club-specific lines (inferred from the counts and durations; the selection code is compiled). `exhibitionarena.arenaaudioid` and
`.cityaudioid` are the same keyword space (NHL 21: 156 / 157 values, 100 % ⊆; equal to the home club's `audioid` on the NHL arenas
inspected), and so is `chantaudioid` (60–63 values per title, 100 % ⊆; on the arena row up to NHL 17, on the team row from 18). `audioid2 == audioid + 1` on 425 / 433 NHL 21 teams (408 / 416 on NHL 20; 0 on 15–19) — meaning unknown (a second keyword
or an artefact). **Player** `audioid` (7 223 distinct, up to 34 797) is *not* in the keyword space (906 / 7 223) and, read as a sample
id, selects samples whose duration distribution equals the bank's (median 1.02 vs 1.37 s; 56 % vs 51 % under 1.5 s) — not
discriminating; `surnameid` likewise. Player-name audio binding: **open** (two hypotheses failed; next: the `pbp_speecheventsubscr` /
`0TVE` layer or a name-call DSET census keyed by the `rink_announcer` banks).

**Operation this enables (proposal):** "make the commentary call team A by club B's name" = set A's `audioid` (and, per arena,
`arenaaudioid` / `cityaudioid`) to B's keyword id — a plain DB field edit within the existing editor; disclose the tagged sample count
(0 = no recorded name). Runtime card **R14-AUD-1 (NHL 21 PS4):** an AHL exhibition after pointing one club's `audioid` at another AHL
club's keyword — the intro/goal call names the other club = the keyword is the consumer; unchanged = no cause identified (cache,
another copy of the DB, the sentence not firing) — next control: the same edit on the *arena* ids. Adding a *new* name recording is the
encoder/bank question of handoff 02 §6.

## 4. Unknowns (explicit)

1. Numbered-vault naming key (Q-R7-1) and how the level selects its vault at runtime. 2. Semantics of the ambiguous census columns
(division numbers vs ids) — per-field, not derivable from values. 3. `audioid2`; the generic keywords 1–~30; player-name audio binding.
4. NHL 20's club-mention tagging (its base `pbp_keywords.bin` is a 91-pair stub; the patch archives are not on this machine). 5. Runtime consumption of any of
this (cards).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r14_team_reference_census.py docs/research/overnight-2026-09-20/data/r14-team-reference-census.json   (≈ 3 min)
```
§3: `python …/probes/r14_audio_keywords.py <out.json>` (≈ 1 min) — it parses `pbp_keywords.bin` as `u32 count @12` (PS4, LE, pairs from 16) / `u32 count @16` (PS3, BE, pairs
from 20) of `(sample id, keyword id)`; join `exhibitionteams.audioid` / `exhibitionarena.{arenaaudioid,cityaudioid}` of the anchor (NHL 20:
install DB) into the keyword id set; count tagged samples per AHL keyword; look the sample ids up in `pbp_samples_inxp.sbr` with
`snd10_sbr.Sbr` for durations and DSETs. **Not executed:** launches, writes, Studio binaries.
