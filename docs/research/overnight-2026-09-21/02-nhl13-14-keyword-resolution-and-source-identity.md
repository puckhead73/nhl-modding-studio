# Handoff R25 — NHL 13 / 14 have no keyword table; the phrase-word resolution states per title; gen7 speech source identities (loose vs. archive) and one truncated Xbox NHL 14 stream

> **Supersession note (same day):** §3's "resolution states 1 / 2 / 3" and its `GRF0` entry reading (`n │ ptr → n words`, "leading word") are
> superseded by [handoff 04](04-gen7-speech-model-from-ea-source-xml.md): the phrase words are (tag, value) pairs, the graffiti entry is
> `(TagId, TagValue, NumTags) → numRefs + sample ids`, and every phrase on every title resolves through its secret-name table. §1 (no keyword
> table on NHL 13 / 14), §2 (source identities, the truncated NHL 14 stream) and §4's DB-audio-id lead stand.

Research lane, 21 September 2026. Applies review F2 of
[research-r22-r23-review-2026-09-21.md](../../validation/research-r22-r23-review-2026-09-21.md) ("the Xbox NHL 13 mismatch is not resolved
until that title's own in-archive KWD0 and matching bank have been checked") and the review's step 2 (extract / stream the NHL 13 / 14
keyword members, record identities, rerun the resolution, keep residuals). Claim `research-r25-gen7-speech-sources`. Nothing game-verified;
no extract written to disk except two per-title temporary copies of in-memory members for the existing readers (deleted by the probe
itself); no launch; no writer.

Machine-readable: [`data/r25-gen7-speech-sources.json`](data/r25-gen7-speech-sources.json). Probe
`local/runs/overnight-research-2026-09-21/probes/r25_gen7_speech_sources.py` (≈ 12 s, read-only; uses the 2026-09-18 `eb_list.py` /
`chunklzx.py`, the 2026-09-19 `chunkzip_walk`, the 2026-09-20 `snd10_sbr.py` / `pbp_sentences.py` / `r14_audio_keywords.py`).

## Plain-language summary

The review asked whether NHL 13's and NHL 14's own keyword tables, hidden inside their archives, explain why NHL 13's sentence words do not
match its sound bank. They cannot: **neither title ships a keyword table at all** — not loose, and not in any of the 11–13 archives of either
platform (every archive's member list was read). The keyword file first appears in NHL 15. What every gen7 title does ship is a
"graffiti" index (`pbp_eng_graffitidata.bin`), and it turns out to be keyed by the speech events' *parameter ids* (player, team, home / away,
penalty type …) plus one extra type that has exactly one entry per sentence word — on every title, including NHL 13, **every** word used by
any sentence is a key of that type. The constant word that sits in every sentence body (287 on NHL 15) is that type's number, which retires
its "keyword 287 / control value" reading. So the read-only chain now has three distinct resolution states: NHL 15 / Legacy words → keyword
table → samples (as before, with 915 / 1 468 words per title outside the keyword table but inside the graffiti index); NHL 14 words are
the bank's own sample ids for 97 % of the distinct words (the bank starts at id 1, so equality is not discriminating on its own); NHL 13
words are neither (its bank starts at 77 269) and resolve only through the graffiti index, whose leading data word is still undecoded. On the
side, the source identities were pinned: the loose `audio/speech` files of every Xbox 360 and PS3 tree are byte-identical to the
`audioboot.big` members once decoded (PS3 NHL 13 has no loose copies — archive only), and the Xbox NHL 14 loose `pbp_eng_samples.sbs` is a
truncated copy (620 MiB of an 820 MB stream) that must not be used as that title's bank stream.

## 1. Keyword-table census (measured, both platforms)

| Tree | EB archives read (members) | `pbp_eng_keywords.bin` |
|---|---|---|
| Xbox 360 NHL 13 | audioboot 25, audioboot2 10, boot 592, cache 14 235, cacheaudio 7, cachemovie 6, cacheoverlay 63, cacherender 10 969, data0 2, nocache 322, nocacherender 10 173 | **none**; not loose |
| PS3 NHL 13 | same archive set (boot 465, cache 14 205, nocache 321, …) | **none**; no loose `audio/speech` at all |
| Xbox 360 NHL 14 | + cacheboot 61, renderboot 280 (13 archives, 40 838 members) | **none**; not loose |
| PS3 NHL 14 | 13 archives | **none**; not loose |
| Xbox 360 / PS3 NHL 15 | 13 archives | `nocache.big` entry 237 (X360: offset 1 393 907 280, 46 472 B chunklzx; loose copy 208 892 B, KWD0, 26 109 pairs, 2 121 keywords) |
| Xbox 360 / PS3 Legacy | 13 archives | `nocache.big` (X360 46 762 B; loose 26 069 pairs, 2 131 keywords) |

Consequence for handoff 13 rev. 4 §2 / handoff 14 (2026-09-20): "keywords file not loose (in-archive) on NHL 13 / 14" is **superseded** by
"no keyword table exists on NHL 13 / 14". The rev. 4 explanation of the Xbox NHL 13 mismatch ("phrase words are keyword ids") therefore
does not apply to NHL 13 / 14; see §3.

## 2. Source identities: loose trees vs. `audioboot.big` members (measured)

`audioboot.big` is an EB `0x00000400` archive with 25 members on every gen7 title; its 8 `pbp_eng` speech / context members are `chunklzx`
on Xbox 360 and `chunkzip` on PS3 (`speecheventdef.csi` / `contexteventdef.csi` stored). Decoded in memory and hashed:

| Tree | members decoded | equal to the loose file | notes |
|---|---:|---|---|
| X360 NHL 13 / 14 / 15 / Legacy | 8 / 8 each | **8 / 8** each | the loose Xbox trees are the shipped build |
| PS3 NHL 14 / 15 / Legacy | 8 / 8 each | **8 / 8** each | chunkzip members decode to the loose bytes |
| PS3 NHL 13 | 8 / 8 | no loose files (absent) | decoded members used for the measurements below; SHA-256 in the JSON |

Bank streams (`pbp_eng_samples.sbs`, in `nocache.big` entry 2): X360 NHL 13 loose 751 296 056 B = member size, max bank offset
751 294 704 ✓; X360 NHL 15 731 829 012 = member ✓; X360 Legacy 789 000 896 = member ✓; **X360 NHL 14 loose 650 117 120 B (= 620 MiB exactly)
vs. member 820 131 756 B and max bank offset 820 125 120 → the loose file is truncated** (the archive member is the only complete stream on
that tree). PS3 loose streams (NHL 13 2 094 403 520; 14 1 128 772 804; 15 1 153 780 608; Legacy 1 095 933 360 B) all cover their banks' max
offsets; PS3 `nocache.big` member sizes were not compared (different codec, larger streams).

`SBKR` banks (loose = member): NHL 13 83 453 ids **77 269–208 407**; NHL 14 89 734 ids 1–91 905; NHL 15 45 442 ids 58–55 834; Legacy 59 940 ids
58–75 273 (identical on both platforms per title; offsets differ per platform stream).

## 3. Phrase-word resolution states per title (measured on the sentence files above; `FNV-1` slot rule holds on every set)

| Title (both platforms) | distinct words / occurrences | = a bank id | in KWD0 | in GRF0 word-type keys | per-body constant | GRF0 word type = max EVT0 param id + 2 |
|---|---|---|---|---|---|---|
| NHL 13 | 4 182 / 29 392 | **0 / 0** | no table | **4 182 / 29 392** | 257 | 257 = 255 + 2 ✓ |
| NHL 14 | 4 422 / 31 490 | 4 290 / 31 325 | no table | **4 422 / 31 490** | 257 | 257 = 255 + 2 ✓ |
| NHL 15 | 2 976 / 19 658 | 1 897 / 15 705 | 2 061 / 18 407 (915 distinct words outside) | **2 976 / 19 658** | 287 | 287 = 285 + 2 ✓ |
| Legacy | 3 546 / 22 930 | 2 262 / 18 155 | 2 078 / 20 922 (1 468 outside) | **3 546 / 22 930** | 298 | 298 = 296 + 2 ✓ |

**`pbp_eng_graffitidata.bin` (`GRF0`)** — bounded layout (parsed as far as stated, nothing beyond): `GRF0<plat> │ u16 3, u16 3 │ 0 │ 0 │ u32
slots @0x14 │ 0 │ @0x1c slots × (u32 count, u32 ptr | ffffffff)`; pointers are relative to 0x1c; each points at `count` 16-byte entries
`u32 type │ u32 key │ u32 n │ u32 ptr → n × u32`. 8 368–18 525 slots per title, 0 malformed entries. **The entry types are the speech `EVT0`
parameter ids** (5 of the 6 most frequent on every title: `player_ID` (2), `team_ID` (1), `team_home` (3), `team_away` (4),
`second_team_ID` / `first_team_ID` (273 / 272), `playoff_focus_team` (67 / 68 on NHL 13) …; all 29–81 types per title are parameter ids except the
one or two above the parameter table: 256 + **257**, 286 + **287**, **298**). The word type's entries number exactly `max word` (4 661 / 4 912 /
3 512 / 4 218) with keys 1..max, and every sentence word is a key.

Word-type entry data: a leading word (1–8, top values 2 / 3 / 5) followed by 0–5 further words; the further words are bank sample ids
(NHL 14 1 372 / 1 372; NHL 13 1 330 / 1 368; NHL 15 765 / 802; Legacy 853 / 888 in the bank) — but 3 140–4 191 of the entries carry no sample
at all, so **this is not the word → samples table**. On NHL 15 / Legacy every keyword id is a word-type key, the sample tails are subsets of the
keyword's sample list in 1 903 / 1 909 cases and never equal, and the leading word is not the keyword's sample count (167 / 2 121 agree).
Candidate reading (not verified): GRF0 is the sample *tag* index (samples tagged by parameter value — player, team, home/away …) used to
narrow a keyword's candidate takes, and the word type is a per-word tag; the slot hash rule and the leading word are undecoded.

**Resolution states for the contract (replace handoff 14's single "keyword → samples" line):**

1. NHL 15 / Legacy: word → KWD0 keyword id → sample ids → bank entries (22 080 / 26 109 and 21 999 / 26 069 keyword sample ids are bank
   entries; the rest are missing ids to be listed, not hidden). Words outside KWD0 (915 / 1 468 distinct; includes the constant) stay
   "identifier (graffiti key, unresolved)".
2. NHL 14: no keyword table. 4 290 / 4 422 distinct words equal bank ids — a *candidate* direct-id reading, undiscriminated because the bank
   starts at 1 (any small integer "matches"); 132 words are not bank ids. Display as "identifier (unresolved; equals a bank id)" until a
   discriminating test exists.
3. NHL 13: no keyword table; words never equal bank ids (bank 77 269+). Every word is a GRF0 word-type key; 716 / 4 661 keys carry 1–5 sample
   ids, the rest none. Display as "identifier (unresolved)".
4. The constant word (257 / 287 / 298) is the GRF0 word-type number (= max parameter id + 2), present once per body — a *type tag*, not a
   keyword; that it is also a KWD0 id on NHL 15 / Legacy (287 ∈ 1..2 121) is coincidental range overlap and stays recorded.

## 4. What this closes and what remains

- **Closed (data level):** existence of NHL 13 / 14 keyword tables (none); loose-vs-archive identity of every gen7 speech input (NHL 13 PS3 =
  archive only); the Xbox NHL 14 stream truncation; the GRF0 type space = EVT0 parameter ids; the per-body constant's identity.
- **Open:** the actual word → sample mapping on NHL 13 (and the discriminating test for NHL 14's direct-id candidate); GRF0's slot hash and
  leading word; what the KWD0 table adds on NHL 15 beyond GRF0. Next discriminating step (cheap, static): for one NHL 15 sentence body, take a
  known keyword (e.g. `140` from handoff 13 §4's worked example) and compare the KWD0 sample list with the union of GRF0 entries of types
  `player_ID` / `team_ID` for the values the sentence's event parameters carry — if the intersection reproduces the takes, GRF0 is the tag
  filter and the NHL 13 path is word → (all tagged samples of the parameter values). Then the NHL 13 test: the type-2 (`player_ID`) keys vs
  the NHL 13 DB `playernames.audioid` (the Players-phase probe R26 does this on NHL 15 / Legacy first).
- **Lead for Phase 3.5 / 2.7 (candidate, measured once, to be probed in the Players / Teams packages):** on the PS3 NHL 15 anchor,
  `playernames.audioid` (1 172 distinct, 30 004–34 059) are GRF0 `player_ID` keys 1 169 / 1 172 (Legacy 1 681 / 1 683); `exhibitionteams.audioid`
  (211 distinct) are `team_ID` keys 207 / 211 (Legacy 206 / 212); `exhibitionarena.cityaudioid` / `arenaaudioid` 130 / 155 and 130 / 157.
  `exhibitionplayerbiotable.audioid` (1–14 567) is not in the key space (1 419 / 5 311) — a different join (probably the `playernames` row).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r25_gen7_speech_sources.py docs/research/overnight-2026-09-21/data/r25-gen7-speech-sources.json   (≈ 12 s)
```
Inputs: the eight tree roots named in the probe (read-only); every input's SHA-256 / size is in the JSON (`audioboot_members`, `bank`,
`sentences`, `word_resolution.keywords` / `.grf0`). Temporary files: `r25_<tree>_{sbr,sent,evt}.bin` in `%TEMP%` (this session: the
scratchpad), written and removed by the probe. Executed: 3 runs of r25 plus the exploratory scratch scripts (removed at closeout); the
`playernames.audioid` test was a scratch run against `anchors/ps3/{nhl15,nhllegacy}/databases/nhlng.db` (to be made durable in R26).
