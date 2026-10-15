# Handoff R9 — the SND10 sound-bank index decoded (PS4 `SBle`, gen7 `SBKR`, gen7 `ABKC`), and the league cue → sample chain on NHL 21

Research lane, 20 September 2026 (Priority 1A of the overnight campaign). Claim `research-r9-snd10-audio`. Consumer: Phase 1.5
(league audio, P1-H), Phase 2.7 (horns / PBP / arena music), 3.5 (name audio), 5.4 (menu music); replaces the "SND10 bank index
undecoded" blocker of handoffs [04](../overnight-2026-09-19/04-league-audio-consumers.md) §3 and
[19](../overnight-2026-09-18/19-goalhorn-ids-and-hornbank-static.md) §2. Nothing here is game-verified; no audio was decoded to PCM or
played; no install, archive or bank was written; no game launched. **No writer is proposed; the preservation contract in §6 is a
proposal for the developer, not an authorisation.**

Machine-readable (under [`data/`](data/)):

- [`r9-snd10-bank-census.json`](data/r9-snd10-bank-census.json) — every `.sbr`/`.abk` of the seven extracted PS4 trees (418 `SBle`
  banks, 44 470 DSETs, 987 037 records) and the loose PS3 `USRDIR/audio` trees of NHL 11 / 14 / 15 / Legacy (12 `SBKR`, 862 `ABKC`):
  identities, DSET/field/storage census, offset validation (sampled 36 780 + 3 344 records → 36 780 / 3 343 land on an SNS header
  block), codec / rate / channel census, duration agreement (13 420 / 13 420).
- [`r9-nhl21-ps4-pbp-sentences.json`](data/r9-nhl21-ps4-pbp-sentences.json) — NHL 21 PS4 `pbp_sentencedata.bin` parsed to
  sentence → phrase → sample-group → sample ids (2 137 sentence slots, 4 953 phrases, **4 953 / 4 953 resolved** to a bank DSET);
  `filtered` = the 93 league/cup/competition-keyed sentences with their sample ids and durations.
- [`r9-nhl15-ps3-pbp-sentences.json`](data/r9-nhl15-ps3-pbp-sentences.json) — the gen7 counterpart (PS3 NHL 15), phrase records
  decoded structurally; the sample-id link is **partial** (§4.2).

Probes (`local/runs/overnight-research-2026-09-20/probes/`, read-only): `snd10_sbr.py` (the reader: `Sbr` for `SBle`, `Sbkr` for
`SBKR`, SNS block walker, SNR header split; module docstring = format record), `snd10_census.py` (the census, incl. `ABKC`),
`pbp_sentences.py` (sentence data, PS4 and `--gen7`).

**Evidence vocabulary:** **observed** = read from the named files and validated by the counts given; **inferred** = a reading whose
producing mechanism is not traced; **unknown** = not established. Codec *labels* are inferred from observed framing (§3) and stated as such.

## Plain-language summary

Every sound bank in the PS4 titles and the later Xbox 360 / PS3 titles can now be read: which sounds it holds, which sample each sound
id points at, how long each sample is and which codec family it uses. A bank is a table of "sound groups", each with a list of
records carrying a sound id, a byte offset into the embedded data or the sibling `.sbs` stream file, and a duration; the four ways EA
packs those columns (a single value, an arithmetic progression, an array with a base and a shift, a small value table with bit-packed
indices) are all decoded, and every sampled record's offset lands exactly on a sample header — 36 780 of 36 780 on PS4, 3 343 of 3 344
on the older banks. On NHL 21 the play-by-play sentence file is also decoded far enough to go from a named commentary line
(e.g. `INTRO_MATCHUP_LEAGUE_AHL`, `GAME_OVER_CUP_WON_CHAMPIONS_LEAGUE`) to the exact sample ids and byte ranges it can play, for all
4 953 phrases; so the "league audio" cell moves from *format unknown* to *sample located, codec identified, no encoder*. What is still
blocked is producing a replacement sample in the game's codec (EA-Opus / EA-MP3 on PS4, EALayer3 v1 on PS3, XMA on Xbox 360) — a
codec/encoder question, not an index question — and, for the older titles, the last link from a named speech *event* to the
sentence that plays (their sentence file names only 701 of the events).

## 0. Sources (observed)

| Family | Files | Identity | Notes |
|---|---|---|---|
| PS4 15–21 | `H:\Emulators\ShadPS4\Games\CUSA{00561,01909,04326,07569,11126,15439,18182}\databigs\Extracted\nhl\data\kettle\audio\**\*.sbr` (+ sibling `.sbs`) | per-bank SHA-256 of the first MiB + length in the census JSON | 29 / 26 / 21 / 50 / 73 / 107 / 112 banks; the extracted trees are partial extracts of `inxp_40.big`, `game_*.big` etc. (whatever was extracted earlier — not re-extracted here) |
| PS3 11 / 14 / 15 / Legacy | `X:\Games\Emulation\Roms\Sony Playstation 3\<title>\PS3_GAME\USRDIR\audio\{speech,music,sfx}\*` loose | census JSON | NHL 12 has no `USRDIR/audio` dir on this dump, NHL 13 has none loose (in-archive), NHL 08–10 keep only `.sbs` loose (their `.sbr` are in `audioboot.big`, not opened here) |
| Xbox 360 Legacy | `X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\audioboot.big:audio/speech/pbp_eng_samples.sbr` (chunklzx, 289 818 → 916 492 B), `…/music/music_samples.sbr`, `…/sfx/fe.abk`; `nocache.big:audio/speech/pbp_eng_samples.sbs` (raw, 789 000 896 B) | member offsets/sizes in the EB tables | one-title spot check (§2.3) |

## 1. `SBle` (PS4 15–21) — the container (observed on 418 / 418 banks)

```
0x00 'SBle' | u32 file size | u16 version (4) | u16 DSET count | u32 bank id
0x10 'MRAH' | u32 | u32 pointer-block offset (0x50) | u32 0x20
0x20 u32 data-chunk offset | u32 0x50 | zeros
0x50 DSET count × (u32 DSET offset, u32 pointer-slot)          ← the DSET list; DSET headers and their arrays are not adjacent
data chunk: 'data' [+ 0x104-byte "Stream File Path Buffer" (00 c1 '[[' … ']]' 00) when the bank streams from a .sbs] [+ embedded SNS payloads]
DSET ('TESD'): u32 size | u64 dataset id | u32 0 | u32 | u32 data-chunk offset | u32 | 24 × 0
   +0x38 u32 record count | u16 field count | u16 | u16 0x48 | u16 | u16 | u16 0
   +0x48 field descriptors, 24 B each: name (4, reversed: 'DIS.' = .SID) | u8 type | u8 storage | u8 shift/step | u8 element size |
         u32 x | u32 0 | u32 array offset (absolute) | u32
```

Fields seen: `.SID` (sound id), `.RAM` (offset into the bank's data chunk, embedded), `.OFF` (offset into the `.sbs`), `.SBS`
(stream-bank number, constant 6), `.DUR` (float seconds); three unnamed keys (`19613ec4`-like reversed names) on 36 DSETs, opaque.

**Storage modes (observed, all four validated by the offset test):**

| storage | meaning | value of record *i* | witnesses |
|---|---:|---|---|
| 0 | inline (one record) | `x` | 66 698 descriptors |
| 1 | arithmetic progression | `x + i · step`, step = int16 from the (shift, element-size) bytes (1, 2, 3, −1, −2, 15 040 seen) | 32 533 |
| 2 | array | `x + (elem[i] << shift)`, elem of 1 / 2 / 4 bytes at the array offset | 73 283 |
| 3 | value table + bit-packed indices | table of `x` values (elem size each) then record-count × `shift`-bit little-endian indices; value = `table[index]` | 290 |

`x` is the field's base (first/min value): `.OFF` base 32 = the `.sbs` 32-byte `data` header; `.RAM` base 4 or 0x108 = the `data`
tag (+ path buffer); `.DUR` base is the float's bit pattern and the array holds bit-pattern deltas (e.g. `0x3ff43fbe + 0x4f2486` =
3.053 s; 13 420 / 13 420 durations agree with `num_samples / rate` of the referenced sample within 0.02 s); `.SID` base is the lowest
id of the group (validated on the 39 048 PBP sample ids of `pbp_metadata.xml`: base + delta gives 32 843 distinct ids, **0 outside**
the metadata id set; the raw deltas would give 17 898 with 6 790 duplicates).

Mixed banks exist (e.g. NHL 21 `crowdbedssamplebank.sbr` 70.9 MB with a 21.5 MB `.sbs`): some DSETs use `.RAM`, others `.OFF`; both
resolve with the rule above (0 misses after the rule was applied).

**Sample payload (observed; both embedded and `.sbs`):** EA SNS block stream — `u8 type | u24 BE size (incl. the 4-byte header)`:
`0x48` header block (holds the 8-byte SNR header), `0x44` data blocks (then `u32 BE samples-in-block`, e.g. 0x2808 = 10 248), `0x45` end
block (size 4). SNR header words (big-endian): word 1 = `version(4) | codec(4) | channel config(6) | sample rate(18)`, word 2 =
`type(2) | loop(1) | num samples(29)` — the split is the community's; the rate and sample-count positions are **confirmed** here by the
48 000 / 44 100 / 32 000 values and by the `.DUR` agreement above. The `.sbs` file itself is `'data'` + 28 zero bytes then the streams
back to back (first sample at 32 on every PS4 speech bank inspected).

## 2. Gen7 banks (Xbox 360 / PS3 NHL 11–Legacy)

### 2.1 `SBKR` (speech / music banks; observed on 12 PS3 banks + 2 Xbox Legacy members)

```
0x00 'SBKR' | u32 0x00000110 | 12 × 0 | u32 0 | u32 bank id | u32 0 | u32 sample count N
0x20 u32 0x30 | u32 id-table offset (0xb6) | u32 'SNS1' tag offset (0xb0) | u32 0 | u32 0x007e0000 | zeros
id table: N × 10 B (big-endian): u32 sample id | u16 count (1 on every record) | u32 ptr → 6-byte record: u16 0 | u32 ptr → u32 .sbs offset
.sbs: 'SBKS' + 12 zero bytes, then SNS block streams (same 0x48 / 0x44 / 0x45 framing as PS4)
```

Validated: 3 343 / 3 344 sampled ids land on a `0x48` header (the one miss is a NHL 14 `ann_eng_samples` entry whose pointer is out of
range). Xbox 360 Legacy `pbp_eng_samples.sbr` (59 940 ids, 300 / 300 sampled) is the same layout with the `.sbs` inside `nocache.big`;
its bank id `19925777` equals the PS3 speech banks' (NHL 11 / 14 / 15 / Legacy all carry `19925777` for both `pbp_eng` and `ann_eng`,
and `1072fa61` for music). The Legacy music bank holds **239** slots (Xbox and PS3 alike; NHL 15 263, NHL 14 231, NHL 11 195).

### 2.2 `ABKC` (`.abk` sfx / horn / chant banks; 862 PS3 files + 1 Xbox member, observed)

```
0x14 u32 file size (862 / 862 equal the length) | 0x18 u32 sample-table offset | 0x1c 0x78 (bank tree start) | 0x24 u32 data size
sample table: 'S10A' | u32 0 | u32 count | u32 header length = 16 + 4·(count − 1) [850 / 862 files; the 12 others are the large
   `sfx.abk`-class banks, not decoded] | (count − 1) × u32 offset of samples 2..n from the table start |
per sample: SNR word 1 | SNR word 2 | u32 data size | u32 (= word 2) | raw codec stream (no SNS framing)
```

The bank tree at 0x78 (sound → sample index, the `{id:03}_goal.abk` single-sample horns need none) is **not decoded**. Horn banks:
135 / 217 files hold one sample; chants 3–5.

### 2.3 Cross-platform (observed on the Legacy spot check)

PS3 and Xbox 360 Legacy share the bank *index* format and the speech codec (EALayer3 v1 @ 44 100 mono, §3); the Xbox `.abk` sfx are
XMA (`fe.abk` sample 1: codec 3, 44 100, 12 792 samples) where the PS3 `.abk` are EALayer3 v1. Xbox members are `chunklzx`-wrapped
inside `audioboot.big` (the 2026-09-18 `chunklzx.py` decoder was used), the `.sbs` is a raw member of `nocache.big`.

## 3. Codec census (observed nibbles; labels inferred from framing)

| Codec nibble | Where | Framing observed → label |
|---|---|---|
| `0x0b` | PS4 15–17 speech / music / sfx / crowd / ui; 18–21 sfx / crowd / ui (2 593 · 2 945 · 3 267 · 2 214 · 1 904 · 1 988 · 2 813 sampled) | data blocks = 6-byte prefix + MPEG audio frames (sync `FF FB` found in 30 / 30 sfx and 40 / 40 player-call blocks, 0 in any `0x0c` block) → **EA-MP3** |
| `0x0c` | PS4 18–21 speech, 19–21 music, threes/gauntlet commentary, part of sfx/ui (926 · 3 074 · 4 874 · 5 481) | 2-byte length-prefixed packets, silence packet `fc ff fe` repeated, no MPEG sync → **Opus-shaped ("EA-Opus")** |
| `0x06` | PS4 15/16 remnants (73 / 9 samples), NHL 20/21 `music_inxp` 2 | header only → EALayer3 v2 (public table label) |
| `0x09` @ 32 000 Hz mono | PS4 15–21 `rink_announcer` banks (603–624 per title), PS3 NHL 14 `pbp_eng_samples` (302 / 302) | **not identified** (the public table's label for 0x09 is implausible for these platforms; nothing decoded) |
| `0x05` | PS3 11 / 14 / 15 / Legacy speech (24 000 mono ann / 44 100 pbp), music (44 100), `.abk` sfx (22 050 / 24 000 / 44 100 / 48 000) | header only → EALayer3 v1 (public table; consistent with the 2026-09-18 memory note) |
| `0x03` | Xbox 360 Legacy `fe.abk` | header only → EA-XMA |

So the PS4 speech codec changed between NHL 17 (`0x0b`) and NHL 18 (`0x0c`); a replacement sample must be encoded per title. **No
encoder for any of these exists in Studio or in this lane; none was attempted.**

## 4. The league cue chain

### 4.1 NHL 21 PS4 — complete at the static level (observed)

`pbp_sentencedata.bin` (`0CPS` + `KETT`, 64-bit file-offset pointers): 2 137 sentence slots → 2 137 sentence variants (2 100 distinct
names) → 4 953 phrases. A phrase record is `u32 type | u32 groupKey | u32 historyKey | u64 0 | …`; **`groupKey` equals the second
u32 of a DSET id in one of the sixteen `pbp_samples*.sbr` banks of the same directory (4 953 / 4 953 resolve; 2 164 of the 3 731
DSETs are referenced)** and `historyKey` is an entry of `pbp_samples.shst` (27 `HSle` records; the repetition-history set). The
`silence` bank supplies the pauses (`ca730eb7` → silence ids 1, 10; `5d47a43f` → 3, 7). Every referenced DSET yields its sample ids,
`.OFF` into `pbp_samples_inxp.sbs` (1 011 709 856 B), `.DUR` and the SNS/SNR header (EA-Opus, 48 000, mono).

League/cup examples (from `filtered`, all sample ids in `pbp_samples_inxp.sbr` unless marked):

| Sentence (slot) | Phrases → sample ids |
|---|---|
| `INTRO_MATCHUP_LEAGUE_AHL` (1874) | `dc68e1e8` → 9754–9757, 20064–20068, 34294–34303 (19 variants) |
| `INTRO_MATCHUP_LEAGUE_INTERNATIONAL` (1028) | `51f8af9e` → 13919–13921 |
| `INTRO_MATCHUP_LEAGUE_CHAMPIONS_GEN` / `_FINAL` (511 / 1820) | 18497–18499 / 18500–18502 |
| `INTRO_MATCHUP_LEAGUE_ALLSTAR_GAME` (1444), `…_NHL_ALUMNI_TEAMS` (2119), `…_TOP_VS_BOTTOM` (312) | 13922–13924, 13925–13928, 34268–34272 |
| `GAME_OVER_CUP_WON_CHAMPIONS_LEAGUE` (35) | `0fa142ce` → 27362–27365, silence 1/10, `b7ef50fb` → 24295–24296, silence 3/7 |
| `GAME_OVER_CUP_WON_CALDER` / `_KELLY` / `_MEMORIAL` / `GAME_OVER_SPENGLER_CUP_WON` | shared opener 27362–27365 + cup-specific 24287/24289 · 24291–24292 · 24293–24294 · 24288/24290 |
| `CUP_ENDINGS_SEQ2_TOURNAMENT_WON` / `_NHL`, `SEQ5_TROPHY_HOIST_STANLEY_CUP_NHL`, `SEQ5_…_SPENGLER_CUP` | 16658–16662 / 16663–16668, 39351–39355 + 16694–16698, 29148–29152 |

The 93 league-shaped sentences (leagues, cups, Spengler, Memorial, Calder, Kelly, Champions, All-Star, international, alumni) are
listed with their ids in the JSON. The *runtime* selection (which sentence a context fires, which variant is picked) stays in the
compiled context scripts (handoff 04 §2) and is unknown; this handoff gives the static reachable-sample set per named sentence.

### 4.2 Gen7 (PS3 NHL 15 witness) — partial (observed)

> **Corrected 2026-09-20 by handoff [13](13-gen7-speech-events-evt0-and-sentence-join.md) rev. 3:** the rev. 2 parser read one body per
> sentence and mis-read the name of multi-body sentences. The file holds **1 489 named sentences with 3 337 bodies** (not "701 distinct
> names"), placed at `FNV-1(name) mod 1489` (hash table, verified on every record), and 15 705 / 19 658 phrase words are bank ids (not
> 2 106 / 3 812). `EVT0` is the event → parameter table; `MOIR` is an id → name table, not a script. `r9-nhl15-ps3-pbp-sentences.json`
> was regenerated with `pbp_sentences.py` rev. 3. The text below is kept as written.

`pbp_eng_sentencedata.bin` (`SPC0PS30`, 32-bit big-endian pointers): 1 489 slots, 1 489 variants, **701 distinct names**, phrases =
`4 type bytes | u8 2 | ff ff ff | u32 287 | u32 X`. Every phrase carries the constant 287 first and one further word `X`; `X` is a
sample id of `pbp_eng_samples.sbr` in **2 106 of 3 812** phrases (the others are in the id range but absent from the bank — keyword /
parameter references are the inferred reading, not established). The AHL / CHL cup *events* named in handoff 04 (`CUP_END_CUP_AHL`,
`CUP_END_WON_CHL`, …) exist in `pbp_eng_speecheventdef.csi` / `pbp_eng_speecheventsubscr.bin` (3 497 / 3 976 names) but **not** as
sentence names (the sentence file names `CUP_END_CUP_NHL`, `CUP_END_HANDSHAKE`, … — 1 435 name strings, 701 as variants); the
event → sentence subscription (`EVT0`, header `u32 2 | u32 count | u32 slots | u32 0x1d | u32 0x28 | …` on NHL 21 with 64-bit
pointer lists) is the **remaining decode** for gen7. The NHL 21 sentence names coincide with the event names for the league lines,
which is why the PS4 chain closes without it.

## 5. The horn bank, revisited with the decoded index (observed; binding still open)

NHL 21 `hornbank.sbr`: DSET 0 = 413 records, `.SID` = 56 + delta → ids {56, 57, 58, 62, 71, 74, 77, 80, 82, 158, 232–291, 300–320, …, 751}
(EA-Opus 48 000 stereo, 1.9–40.1 s); DSET 1 = 5 records, ids 2/3/5/6/8 (EA-MP3). NHL 18: 118 + 3; NHL 17: 91 + 3; NHL 16: 56 (ids 33–75)
+ 1; NHL 15: 334 (ids 2, 2, 3, 3, … in pairs — two samples per id) + 1. The DB `exhibitionarena.goalhorn` id space of handoff 19
(NHL 18–21: 1–31, 41, 501, 690) is **not** the bank's `.SID` space as decoded here (56 + delta): only ten DB ids coincide with raw
deltas and two with base + delta. The mapping DB id → bank sound id therefore runs through something else (the UI's
`GoalHornList` / a compiled table) — **Q-R9-1**, and handoff 19's runtime test 1 remains the discriminating card.

## 6. What this enables — the smallest operations and their preservation contract (proposal; policy, not evidence)

1. **Preview / extract (implementable now, no runtime question):** list banks → DSETs → records with sound id, offset, length (walk the
   SNS blocks to the `0x45` end block), duration, codec nibble; extract one sample as its SNS stream (or the raw `.abk` stream). Decoding
   to PCM needs a codec: EALayer3 v1 (gen7), EA-MP3 / EA-Opus (PS4), XMA (Xbox sfx) — none in Studio; extraction is codec-agnostic.
2. **Reassign (PS4, same bank):** point a sentence phrase at another existing group — a 4-byte `groupKey` write in
   `pbp_sentencedata.bin` (the file is pointer-relocated; changing a u32 in place moves nothing). Preserve every other byte; the
   `historyKey` must stay a valid `.shst` entry. Runtime effect unknown (card R9-AUD-1).
3. **Replace a sample in place (any family):** same codec, same channel/rate, new SNS stream **≤ the original's byte length** (the
   `.OFF`/`.RAM` of the *next* sample bounds it; zero-fill the remainder); patch the SNR `num samples` word and, on `SBle`, the `.DUR`
   float (storage 0: inline; storage 2: the array delta; storage 1 / 3: **refuse** — the progression / table cannot express one
   changed value without re-encoding the field). `SBKR` has no duration column. Everything else byte-preserved.
4. **Replace with a longer sample:** append the stream at the end of the `.sbs` / data chunk and re-point `.OFF`/`.RAM` — possible only
   when the field's storage can express the new offset (storage 2 with `(new − x) >> shift` < 2^(8·elem)); otherwise the field must be
   re-encoded (storage 2 / element size 4), which moves every later array and every DSET-header array pointer → **refuse in the first
   implementation**. For the PS4 `.sbs` inside `inxp_40.big` etc. the archive placement rules of handoff 02 rev. 2 apply on top.
5. **Refusals:** codec nibble differs from the slot's; `.DUR` field storage 1 / 3; unnamed fields on the DSET; `ABKC` banks whose
   header-length rule fails (12 files); any record whose offset does not land on a `0x48` block (1 known).

## 7. Unknowns (explicit)

1. Codec identity of `0x09`; PCM decoding of any codec; encoders. 2. The bank tree at `ABKC` 0x78. 3. Gen7 event → sentence
subscription (`EVT0`) and the non-bank phrase word `X`. 4. PS4 `pbp_speecheventsubscr.bin` / context → sentence runtime selection.
5. DB `goalhorn` → hornbank sound id (Q-R9-1). 6. The three unnamed DSET fields; `SBle` header words at 0x14 / DSET +0x14 / +0x1c;
`SBKR` word 0x007e0000; `.SBS` = 6. 7. NHL 08–10 (`.sbr` inside `audioboot.big`, not opened) and NHL 12 / 13 PS3 (in-archive) —
same layout is expected from the Legacy result but is **untested**. 8. Whether the loader tolerates a shortened sample / a changed
`.DUR` — runtime.

## 8. User cards (deferred; no review requested now)

- **R9-AUD-1 (NHL 21 PS4, listen-only, no edit):** none needed for the static chain. If the developer implements operation 2 (reassign
  `INTRO_MATCHUP_LEAGUE_AHL`'s phrase to the `INTRO_MATCHUP_LEAGUE_INTERNATIONAL` group) the card is: AHL exhibition on the edited
  copy → the intro line changes to the international line = the sentence file is the consumer and reassign works; unchanged →
  no cause identified (cache, other copy of the file, context not firing) — next control: the same edit on the `_INTERNATIONAL`
  sentence in an international game.
- **R4-AUD-2 / handoff 19 test 1** unchanged (horn id → sound).

## 9. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-20/probes/snd10_census.py docs/research/overnight-2026-09-20/data/r9-snd10-bank-census.json [--sample 300] [--only ps4|ps3]      (≈ 6 min)
python local/runs/overnight-research-2026-09-20/probes/pbp_sentences.py "<…CUSA18182…/audio/speech/eng>" docs/research/overnight-2026-09-20/data/r9-nhl21-ps4-pbp-sentences.json --filter LEAGUE,CUP_END,…
python local/runs/overnight-research-2026-09-20/probes/pbp_sentences.py "<…NHL 15/PS3_GAME/USRDIR/audio/speech/pbp_eng_sentencedata.bin>" <out.json> --gen7 "<…/pbp_eng_samples.sbr>" --filter CUP_END
python local/runs/overnight-research-2026-09-20/probes/snd10_sbr.py <bank.sbr> [--dump-fields] [--json out.json]
```

Executed (read-only): the census twice (before/after the mixed-bank and codec-label corrections; final numbers above); the NHL 21
sentence parse; the PS3 NHL 15 gen7 parse; ad-hoc checks in the session scratchpad — hornbank field dumps, the storage-mode
hypotheses (base + shift, progression step width, table + bit indices — each confirmed by `0x48` hits), the PBP id-space test against
`pbp_metadata.xml`, the DSET-key-in-sentence-data search (6 355 hits, 0 for full 64-bit ids), the MPEG-sync test per codec nibble,
the Xbox Legacy member extraction (two `.sbr` copies written to the scratchpad and deleted at closeout). **Not executed:** launches,
writes, any Studio binary, any audio decode.

> **Note 2026-09-24:** §2.2's per-sample record line describes non-loop `ram` records only — loop-flagged and stream-type records and absent slots are specified in the [2026-09-24 handoff 01](../overnight-2026-09-24/01-gen7-archive-audio-coverage-and-abk-records.md) §1; §3's `0x09` "not identified" row is superseded: EA-Speex ([2026-09-24 handoff 02](../overnight-2026-09-24/02-codec09-easpeex-and-ea-xma-preview.md)).
