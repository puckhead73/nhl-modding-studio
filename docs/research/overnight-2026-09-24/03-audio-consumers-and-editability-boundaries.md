# Handoff R85 — audio consumers and editability boundaries (Leagues 1.5 / Teams 2.7): the gen7 loader's own asset table (`na_en_loadfile.xml`, identical loose / archived on all 16 trees) names every numbered horn and chant bank by id — the residual `goalhorn` / `periodhorn` / `chantaudioid` values are **unlisted in the loader table**, not only absent from the folders; the four-digit chant banks are **goalie chants keyed by the goalie's announcer audio id** (44 / 45 on NHL 10, every match a goalie); the PS4 goal-horn consumer is located (`sfx_ptc.big` → `goalhornpatch.ptc`, Harmony `PTCH` patch, format undecoded); and a per-class read / preview versus replacement table with every unknown named

Research lane, 24 September 2026, 05:34 → 05:40 MDT (clock corrected against file times) (brief item R3). Claim `research-r85-audio-consumers`. Read-only: the 16 gen7
trees, the PS4 NHL 21 extracted tree, the committed anchors, this campaign's JSONs. Nothing listened to; nothing game-verified; no
writer proposed.

Machine-readable: [`data/r85-gen7-loadfile-horn-assets.json`](data/r85-gen7-loadfile-horn-assets.json). Probe
`r85_gen7_loadfile_horn_assets.py` (≈ 1 min). The goalie-chant join and the PS4 patch look were inline probes (commands in §6).

## Plain-language summary

The older consoles keep a plain text list of every sound bank the game may load, with the horn or chant number written next to each
file. That list names exactly the banks that exist (0–29 and 100, plus 194 from NHL 14) — so the European and minor-league arena horn
numbers are missing from the game's own load list, which is the strongest thing static data can say short of running the game. The
same list shows that the crowd's goalie chants (the four-digit files) are keyed by the goalie's announcer id: on NHL 10 all but one
of the 45 chant banks match a goalie in the roster, and every match is a goalie. On PlayStation 4 the goal horn is built by a
"patch" inside an archive that was named last time and is now confirmed to hold the horn patch, the team names and the horn-element
ids; its binary format is not decoded, so it is a located consumer, not a mapping. The closing table says, class by class, what
Studio can read or preview today and exactly what is still missing before anything could be written back.

## 1. Gen7 loader asset table (measured, 16 / 16 trees)

`audio/na_en_loadfile.xml` — loose on X360 09–Legacy and PS3 11 / 14 / 15 / Legacy and as an `audioboot.big` member on every tree
(PS3 09 / 10 / 12 / 13 archive-only); loose and archived copies byte-identical on all 11 trees with both (SHA-256 in the JSON; the
X360 09 archived copy is RefPack-wrapped, 3 317 → 27 393 B).

| Asset type | loadtype | ids listed (every tree unless noted) | descr == id | matches the members present |
|---|---|---|---|---|
| `AemsSfxGoalHornBank` → `audio/sfx/<id:03>_goal.abk` | temp | 0–29, 100 (31; NHL 09–13); + 194 (32; NHL 14–Legacy) | 100 % | exact both ways on 16 / 16 (no listed bank missing, no shipped bank unlisted) |
| `AemsSfxPeriodHornBank` → `<id:03>_period.abk` | temp | same sets | 100 % | exact |
| `AemsCrowdTeamChantBank` → `<id:03>_chant.abk` | temp | 73 ids in 0–79 (gaps 30, 32, 33, 35, 47, 50) | 100 % | exact |
| `AemsCrowdGoalieChantBank` → `<id:04>_chant.abk` | temp | 45 four-digit ids (4, 31, 119, 127, 170 … 4 582) | 100 % | exact |
| `AemsHornsBank` → `audio/sfx/horns.abk` | temp | — (the empty bank, R81) | — | present |
| `AemsHornsEventDefinition` → `audio/sfx/aemsHorn.csi` | **perm** | — (the segment-built horn module, R81) | — | present |

Residual `goalhorn` / `periodhorn` values (R78 for the loose trees, R82 for PS3 09 / 10 / 12 / 13 and X360 09): **0 of them are listed**
on any tree (NHL 09: 6 / 6 unlisted; 10 / 11: 14 / 14; 12 / 13: 13 / 12; 14–Legacy: 12 / 11). Residual `chantaudioid` values (30–136, 23–25
per title) are listed in neither chant table. Reading: the loader binds ids to banks through this table; an unlisted id has no bank to
load. What the engine then does (silence, bank 0, the segment-built `h_goalHorn`) remains runtime (card H-HORN-1); no per-arena sustain /
gap parameter table exists in any loose definition file on X360 NHL 15 (`Sustain` occurs only in the two module definitions
`aemshorn.csi` / `aemscrwd.csi`), so the segment-built candidate's parameters, if used, come from code or compiled context scripts.

## 2. Goalie chant banks = goalie announcer audio ids (measured join; new)

The 45 `AemsCrowdGoalieChantBank` ids intersected with `exhibitionplayerbiotable.audioid` of the title's own anchor: NHL 10 X360
**44 / 45**, PS3 NHL 12 39 / 45, X360 NHL 15 26 / 45 — every matched row has `position` 4 (goalie) on all three; 0 / 45 match
`playernames.audioid`. The load file is byte-identical across 10 → 15 (except the two horn additions), so the set is a NHL 10-era goalie
list that later rosters no longer fully contain. Contract: `<audioid:04>_chant.abk` = the crowd chant for the goalie whose announcer id is
`audioid`; listing only (`exhibitionplayerbiotable.audioid` → chant bank present / absent). Not a `chantaudioid` consumer.

## 3. PS4 goal horn consumer located (observed; format undecoded)

`nhl/data/kettle/audio/sfx/sfx_ptc.big` (NHL 21: EB archive, 497 168 B, 89 `.ptc` members, stored) holds `goalhornpatch.ptc` (53 368 B),
`goalhornoverlay.ptc` (52 328 B) and `periodlhornpatch.ptc` (3 984 B). Each begins `PTCH` + u32 `0x0107` (version) + 16-byte id + …;
the goal-horn patch's string table names the patch (`GoalHornPatch`), its parameters (`GoalHornID`, `HornsPlayState`, `NHLGlobalGameStates`,
`Fader.FadeType`, `Pause.PauseControl`), game-state enums and **team names** (`Anneheim` … `Winnepeg`, spelled as shipped); its aligned
u16 / u32 words contain every horn-element id 500–539+ three to five times and 334 / 289 words in the club / generic id ranges. This is
the consumer R50 pointed at; the GoalHornID → element / sample resolution sits inside it. **Not decoded** — a Harmony patch grammar is a
new format investigation, not a bounded join; the period-horn patch (`PeriodHornID` enum `Generic` / `Siren` / `Buzzer`) shows the
`periodhorn` 0–2 values on NHL 21 are an enum, not a bank id.

## 4. Names, city / arena ids, PBP, chants, music — status after this campaign (no repeat of R35 / R78)

| Join | Status | Where |
|---|---|---|
| `exhibitionteams.audioid` → pbp `team_ID` sample group | measured (R35, 2026-09-21 handoff 07) | unchanged |
| `exhibitionarena.cityaudioid` / `arenaaudioid` → tag | no established binding (R35 Q-R35-1; R79 restatement); not loader-table ids (this handoff §1: the loader lists no city / arena assets) | open, runtime or a new key source |
| `chantaudioid` → `<id:03>_chant.abk` | partial candidate (R78): 30–136 residuals unlisted (§1) | listing with the residual text |
| goalie chants | **measured** (§2) | new |
| PBP sentences → samples | name-joined (2026-09-20 handoff 13 / 14); "no sentence carries this name" preserved as the listing state for any name without a sentence | unchanged |
| arena music / songs | PS4 catalogue (R54) implemented by the developer; gen7 music box (2026-09-20 handoff 10) | unchanged |
| EA-MP3 / EALayer3 warm-up and PCM start | no new static evidence this session; the junction test is not repeated; cards A-MP3-1 / A-EAL3-1 stand; A-SPX-1 added for EA-Speex | — |

## 5. Read / preview versus replacement, per class (contract boundaries; every unknown named)

| Class | Read / preview today (researcher-verified) | Replacement contract elements | Known | Unknown / blocked |
|---|---|---|---|---|
| gen7 `.abk` ram entries (EALayer3 v1, PS3) | table + record grammar (handoff 01 §1), diagnostic decode cc 0 / 1 | entry indexing (table offsets rel. to table start; absent slots), framing (SNS-less EA-frames; first block warm-up + PCM frame), sizes (segment `size` incl. 8; `samples`; `num_samples`; data size at 0x24 4-aligned; file size at 0x14), sharing (bank 0 unreferenced; chant 000 byte-identical across titles), placement (loose file or archive member: RefPack / chunkzip / stored), timing (loop_start; segments) | same-size same-sample-count byte swap is structurally expressible; the bank tree at 0x78 references samples by index (unverified) | **encoder** (no EALayer3 v1 encoder qualified; the public `ealayer3` tool exists but is not evaluated here), the 0x78 tree's absolute pointers, changed-size table rewrite, loader acceptance |
| gen7 `.abk` ram entries (EA-XMA, X360) | grammar + diagnostic XMA2 rewrap (handoff 02 B) | as above + packet padding rule, `w0` flag bits | as above | **encoder** (XMA encoders are Xbox SDK tools, not redistributable), same unknowns |
| gen7 speech / music `.sbs` streams (EALayer3 v1 / EA-XMA / EA-Speex) | SBKR index (R9), SNS walk, diagnostic decode per class | id table entry → `.sbs` offset (R28: aliases, shared offsets), stored size and slack to the next offset (R28 "headroom"), SNS block framing per codec, `num_samples` in the SNR words | same-size replacement bytes could be placed in the slack (R28) | encoder per codec (Speex: libspeex encodes UWB — a candidate path, untested; EALayer3 v1: unqualified; XMA: no); the `.sbr` record fields that mirror duration (`.DUR`), loader / streaming assumptions, archive member re-wrap for archived `.sbs` |
| PS4 SBle banks (EA-MP3 / Opus / Speex) | index (R9), diagnostic preview (EA-MP3 rev. 2; Speex now) | `.OFF` / `.RAM` records, DSET fields, `.sbs` layout | listing | encoders (EA-MP3 cross-block reservoir; Opus-shaped packets), `.DUR` mirrors, patch data (`sfx_ptc.big`) that may bind ids |
| DB audio ids (`goalhorn`, `periodhorn`, `chantaudioid`, `audioid*`) | resolved listings (developer; R50 / R78 / this handoff) | a DB cell edit within the id space | reassignment to another **listed** id is a plain DB edit (developer's existing field route) | runtime effect; unlisted ids (this handoff §1); PS4 element / flair semantics (patch undecoded) |

## 6. Consequence (developer; not Codex-accepted) and reproduce

- Arena page, gen7: residual horn ids → "no loader asset for this id (`na_en_loadfile.xml` lists 0–29, 100[, 194]); no bank in
  `audio/sfx` or the archives; `horns.abk` empty" — the same wording family as R81, now with the loader table as the source; the two
  candidates stay behind disclosure. Chant residuals → "not in the loader's team-chant table". Goalie chants → list on the player /
  goalie page by `exhibitionplayerbiotable.audioid` (present / absent), labelled as measured on NHL 10 and ageing on later titles.
- PS4 arena page: the horn patch archive can be named as the consumer ("Harmony patch `goalhornpatch.ptc` in `sfx_ptc.big`; not decoded");
  `periodhorn` on NHL 21 is an enum (`Generic` / `Siren` / `Buzzer` per the period patch strings) — present as such, not as a bank id.
- No replacement route is authorised by this handoff; §5 is the checklist a future contract must fill.

```
python local/runs/overnight-research-2026-09-24/probes/r85_gen7_loadfile_horn_assets.py docs/research/overnight-2026-09-24/data/r85-gen7-loadfile-horn-assets.json
```
Executed twice (a guard for the R78 family lookup). The goalie-chant intersection and the `sfx_ptc.big` listing / string dump were inline
(three anchors; the NHL 21 archive), reproducible from the r85 JSON `ids.AemsCrowdGoalieChantBank` and `eb_list.py` on `sfx_ptc.big`.

> **Correction note 13:25 MDT (R88, [handoff 06](06-correction-checkpoint-campaign-review.md) §F4):** §5's "same-size same-sample-count byte
> swap is structurally expressible" describes class **(a) same-length encoded-payload substitution**, whose prerequisites (tree pointers,
> a same-codec payload of identical length and sample count, duration mirrors, archive re-wrap, loader acceptance) are listed apart from
> class **(b) arbitrary audio import** (adds a qualified encoder, changed-size table rewrite, pointer update) in handoff 06 §F4. Decoding
> the `ABKC` 0x78 tree (R89) serves indexing / stream discovery; it does not by itself qualify either class.

> **Note 13:47 MDT (R89, [handoff 07](07-abk-bank-tree-read-selection-stream-address.md)):** §5's "the bank tree at 0x78 references samples by
> index (unverified)" is now measured: sample-list entries reference every record by index exactly once, and stream entries carry the
> `.ast` offset (48 751 / 48 751 records, 29 962 / 29 962 streams). The replacement columns are unchanged (handoff 06 §F4).

> **Note 13:54 MDT (R90, [handoff 08](08-gen7-team-chant-binding.md)):** §4's `chantaudioid` row is extended: banks 055–066 are club chants
> (Swedish clubs, named inside the bank and in the crowd script `aemscrwd.csi`), referenced by no arena `chantaudioid` and by no team /
> arena audio field on any of the 11 trees — selected by the compiled crowd module, not by the database. The arena-keyed join is
> re-measured per title (24–25 ids without a bank; tenants DEL / national / Czech / create-a-team).
