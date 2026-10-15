# Handoff A-X — audio banks and cue ids on the Xbox 360 family (1.5 / 2.7 / 3.5 / 5.4 pointers)

Research lane, overnight 2026-09-18. Claim `p2.7-audio-banks-xbox360`. Consumer: developer 2.7 (team/arena
cues), 5.4 (menu music), 3.5 (player names), 1.5 (league cues — no league-named bank member found; see the
narrowed statement below). Machine-readable:
`data/xbox360-assets-audio.csv` (every `audio/**` member in every archive of the ten titles). Probe: `audio_joins.py`.

Evidence: **observed** member names/sizes/archives and DB id joins; bank internals **not decoded** (ABK
`ABKC` header seen; sample codec unknown here — O6 reports EALayer3 inside the Legacy music SBR).

## Layout per era

| Era | Where | Families |
|---|---|---|
| NHL 08 | `audioboot.big` (event defs `audio/sfx/events/aems*.csi`, `audio/context/events.csi`, `speech/pbp_eng.csi`), `data0.big` (`audio/sfx/ram/*.abk` 212 banks, `audio/sfx/streams/*.ast`, `audio/streams/streams.{dat,viv}` 226 MB) | horns/chants per id, crowd, PBP (`pbp_eng.{cti,csi,evt,idx,nam}`) |
| NHL 10 | `audioboot.big` (defs, `music_*`), `audio.big` (`audio/sfx/*.abk` 188, `audio/speech/*` 21), `audioboot2.big`, `stadium.big` (8 sfx) | multilingual announcers `ann_{eng,fin,swe}_*` (**NHL 10 only** among the sampled titles), music defs |
| NHL 12–Legacy | `audioboot.big` (defs, music defs, `music_samples.sbr`), `audioboot2.big` (`horns.abk`, `sfx.abk`, `fe.abk`, `hut_fe.abk`, `skates_v2.abk`, coach feedback), `nocache.big` (`audio/sfx/{id}_{goal,period,chant}.abk`, `music_samples.sbs` ~280–300 MB, `pbp_eng_samples.sbs` ~790–920 MB, reverb `.irf`), Legacy also `cacheaudio.big` (7 sfx) | matches O6's Legacy paths (`audioboot.big` → `music_samples.sbr` + `music_musicboxdata.bin`; `nocache.big` → `music_samples.sbs`) — **confirmed on the X: Legacy tree and on NHL 12/13/14/15 too** |

## Cue-id joins (Legacy; NHL 10 identical in shape)

| DB field (`exhibitionarena`) | Bank member | Join |
|---|---|---|
| `goalhorn` (9 bits) | `audio/sfx/{id:03}_goal.abk` in `nocache.big` | banks for ids **000–029, 100, 194** (Legacy; NHL 10: 000–029, 100); DB uses 42 distinct ids — the 30 NHL arenas match, ids 30–61 (other leagues/custom) have **no bank** → engine fallback (candidate: 100 = generic; **unknown**) |
| `periodhorn` (9) | `…/{id:03}_period.abk` | same coverage |
| `chantaudioid` (9) | `…/{id:03}_chant.abk` (117 banks, ids up to 4582) | 38/62 DB ids matched; the chant id space is wider than the horn space and partly unreferenced (79 banks with no arena) |
| `arenaaudioid` (10), `cityaudioid` (11); team `audioid` (11) | speech banks `pbp_eng_samples.sbr/.sbs` (+ `pbp_eng_*` defs, `keywords.bin`, `sentencedata.bin`) | ids index the PBP speech sample tables — needs the SBR index decode (O6 did the music SBR: 239 slots) |
| shared | `audio/sfx/horns.abk` + `aemshorn.csi` (event defs, `MOIR` container) | the *generic* horn set + event bindings — every title 08–Legacy |

`{id:03}_goal.abk` sizes: 26 792 B (id 100) … 199 324 B (id 027) — one bank = one team's horn sample set.

## What this means for the roadmap steps

- **2.7 team cues:** replacing a team's goal/period horn = replacing (or re-pointing the arena's id at) one
  `nocache.big` member; the member is a self-contained ABK bank, so a same-length constraint does not apply
  to the *content* but the EB archive member length does (S2-A same-length rule; a re-plan or padding is
  needed for a different-size bank). Assigning an id that has no bank is exactly the "fallback names"
  question in the roadmap — unknown until a runtime test.
- **5.4 menu music:** the O6 locations are confirmed family-wide (12–Legacy). Editing needs the SBR/SBS
  index + `music_musicboxdata.bin` (tracklist) understood — O6 covers the Legacy specimen; the same files
  exist on 12–15 (sizes differ).
- **3.5 player names:** `pbp_eng_*` (samples + `keywords.bin`/`sentencedata.bin`/`graffitidata.bin`) — the
  team `audioid`/arena `cityaudioid` ranges (≤ 1 999) are consistent with sample-table indices; not decoded.
- **1.5 league cues (scope narrowed, R1 item 3):** among the inspected member *names* (`audio/**` in every
  archive of the ten Xbox 360 titles, `data/xbox360-assets-audio.csv`) there is **no separately league-named
  bank** (`grep league` finds nothing). That is a statement about file names only: it does **not** establish
  the absence of league-related sounds *inside* the generic banks (`fe.abk`, `sfx.abk`, `hut_fe.abk`, the
  event definitions `aems*.csi` / `events.csi`, or the PBP sample tables), whose internals were not decoded.
  PS4 audio was not inspected at all (**unknown**). Record 1.5 as "no per-league bank member found by name on
  Xbox 360; generic-bank contents and PS4 unmeasured", not as a general absence.

## Not established

ABK/SBR internals and codec on Xbox 360; fallback for missing horn ids; whether `cacheaudio.big` members
override `nocache.big` ones (7 Legacy members: list in CSV).
