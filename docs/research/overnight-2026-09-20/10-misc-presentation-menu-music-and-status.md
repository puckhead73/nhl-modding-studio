# Handoff R17 — Miscellaneous presentation: the gen7 music box (menu / arena music slot table, parsed and bound to the bank), the PS4 music banks, and the UI / boards / scorebug status

Research lane, 20 September 2026 (Priority 5 of the overnight campaign: 5.4 first, 5.1–5.3 status). Claim `research-r17-misc-presentation`.
Consumer: developer Phase 5.4 (menu music), Phase 2.7 (arena music: goal / period / intro tunes), 5.1–5.3 planning. Builds on handoffs
[02](02-snd10-audio-banks-and-league-cue-chain.md) (bank index), [15](../overnight-2026-09-18/15-audio-banks-xbox360.md) §5.4 (locations)
and [03](../overnight-2026-09-19/03-non-logo-league-imagery.md) (registries). Nothing here is game-verified; nothing written; no launch;
**no writer proposed**.

Machine-readable: [`data/r17-musicbox.json`](data/r17-musicbox.json) — PS3 NHL 11 / 14 / 15 / Legacy `music_musicboxdata.bin` parsed:
every record (sample id, category, kind, label, context) and the censuses; probe `r17_musicbox.py` (≈ 5 s).

## Plain-language summary

The older titles' music is fully described by one small table: the "music box" lists every track in the music bank once (195 / 231 /
263 / 239 tracks on NHL 11 / 14 / 15 / Legacy, 100 % of the bank's sound ids) with the situation it plays in — `MenuTunes` (the menu
music, 20–42 tracks), `HomeGoal` / `AwayGoal`, `EndOfPeriod`, `IntroNationalAnthem`, `TeamIntroduction`, `OffPlayGeneric` / `OffPlayOrgan`
(stoppage music and organ), `PenaltyHome` / `PenaltyAway`, `WinGame` / `LoseGame`, playoff variants and, on Legacy, `WinterClassic` /
`ShootoutIntro` — plus a category number that groups those contexts. Combined with yesterday's bank decode this gives the whole chain
for gen7 music: table row → sound id → bank record → `.sbs` byte range → EALayer3 v1 stream. "Reassign a track to the menu" is a
36-byte record's context pointer; "replace a track" is the bank/codec question of handoff 02 §6. On PS4 the equivalent selection lives
in a compiled AUGASM script (`music.bin`), so only the banks' contents are listed (423 tracks in `musicbank.sbr`, the `inxp` banks for
the newer play modes, the Nordic-locale bank), not their roles. Boards, scorebug and general UI got no new measurements tonight; what
exists (registries, Apt/Scaleform containers) is summarised with the specific next step for each.

## 1. Gen7 music box (observed on PS3 NHL 11 / 14 / 15 / Legacy; Xbox 360 ships the same file inside `audioboot.big` — not re-extracted)

`music_musicboxdata.bin`: `'MBX0'` big-endian; header word 4 → `(u32 record count, u32 first record offset)`; records of 36 bytes:
`sample id | 1 | 1 | 1 | 1 | 1 | category | kind | ptr` → `ptr` addresses two string pointers (`VolumeType`, context name). Every record's
sample id is a `music_samples.sbr` (SBKR) id and every bank id has a record (195 / 195, 231 / 231, 263 / 263, 239 / 239).

| Context | NHL 11 | 14 | 15 | Legacy | Category (observed grouping) |
|---|---:|---:|---:|---:|---|
| `MenuTunes` | 22 | 32 | 42 | 26 | 1 (all) + 6 (6 on 14 / 15 / Legacy) |
| `IntroNationalAnthem` | 21 | 21 | 21 | 21 | 2 |
| `TeamIntroduction` | 14 | 15 | 15 | 11 | 8 (+3 in 2) |
| `HomeGoal` / `AwayGoal` | 11 / 8 | 11 / 10 | 10 / 8 | 33 / 14 | 3 (Legacy: +3 / +3 in 2) |
| `EndOfPeriod` | 16 | 16 | 16 | 16 | 4 |
| `OffPlayGeneric` / `OffPlayOrgan` | 52 / 23 | 67 / 23 | 90 / 23 | 60 / 28 | 7 (+ some in 2 / 6 / 8) |
| `PenaltyHome` / `PenaltyAway` | 2 / 4 | 2 / 4 | 2 / 4 | 4 / 6 | 6 |
| `WinGame` / `LoseGame` | 8 / 8 | 11 / 6 | 13 / 6 | 5 / 4 | 9 (+ some in 2) |
| `PlayoffOrTournamentWin` / `…Lose` | 3 / 3 | 7 / 6 | 7 / 6 | 4 / 4 | 2 |
| `WinterClassic`, `WinterClassic2012`, `ShootoutIntro` | — | — | — | 1 / 1 / 1 | 2 |

`kind` is 1 on 177–241 records per title with a few 2 / 7 / 8 / 9 / 10 (meaning unknown; the 9 / 10 rows on 14 / 15 are within
`OffPlayGeneric`/`MenuTunes` — not measured further). Category ↔ context is a grouping, not 1:1 (the JSON has the cross-table).
Titles/artist strings are not in this file; the dictionary carries `CREDITS_MUSIC_*` (credits) and `TXT_SONGS` / `TXT_ARTISTS`
labels — the jukebox's per-track names were not located tonight (unknown).

**Operations this enables (proposal):** (a) *list* the menu playlist and the arena-music slots per title with durations and codec
from the bank (handoff 02); (b) *reassign* a track's context (edit the record's `ptr` to another existing context pair — same-length
in-place, no relocation; runtime unobserved → card R17-MUS-1: point one `OffPlayGeneric` record at the `MenuTunes` pair and note whether
the menu plays it); (c) *replace* a track's audio: the in-place / append rules of handoff 02 §6 on `music_samples.sbs` (+ the `.sbr`
offset/duration words), codec EALayer3 v1 — encoder = blocker; (d) *add* a track: a new SBKR id + record — the SBKR id table is a flat
sorted array (insertion = rewrite of the `.sbr`), not proposed.

## 2. PS4 music (observed inventory; selection compiled)

| Bank (NHL 21) | DSETs / records / distinct sound ids | Role (inferred from names) |
|---|---|---|
| `music/musicbank.sbr` + `.sbs` (837 MB) | 4 / 423 / 423 (ids 799–13 771) | the main soundtrack (menu + arena) |
| `music/music_inxp.sbr` + `.sbs` (557 MB) | 5 / 6 210 / 285 (one DSET of 6 160 records = per-context variants of 285 ids) | the newer modes' music (`inxp`) |
| `music/music_inxp_embargoed.sbr` | 2 / 1 429 / 70 | embargoed (region/date-gated) tracks |
| `music/music_inxp_nordic.sbr` | 1 / 1 / 1 | the Nordic-locale alternative (`@IsNordicLocale`, handoff 04 of 2026-09-19) |

The context → track selection is in `augasm/music.bin` (compiled AUGASM; readable identifiers only: `MusicSampleHistory`,
`FestivalMusicSampleHistory`, `MusicState`, `MusicBEController`, …) — no plaintext playlist and no per-track context table were located
in the extracted tree; listing and replacing (codec EA-MP3 / Opus-shaped per title, handoff 02 §3) are possible, *reassigning* is not
(compiled). Track titles: not located (unknown).

## 3. UI, boards, scorebug — status (no new measurements tonight)

| Step | What exists | Specific next step |
|---|---|---|
| 5.1 UI (menu branding, icons, fonts) | gen7: ION/Apt packages in `fe.big`/`cache.big` (`generalhelpers` registry, handoff 03 of 2026-09-19; the 288-byte quad record and Apt object of handoff 05 today are the same container family); PS4: `definition.kettle.assetpaths` registry + GNF/PNG members | a census of the Apt `0` object variants (240 / 256 B) across UI packages to pin the position/anchor words (Q-R12-1) — bounded, not started |
| 5.2 Boards / rink-side ads | PS4: `ge_level` typed references to banner / centre-ad / dasher texture packs (handoff 07 of 2026-09-19 §2, ~100 slots per level, most null) and the numbered stadium vaults (today's 07 §2); gen7: `stadium_*` containers (`.rx2` on Xbox, `.rpsgl` on PS3) | decode the `ge_level` banner/dasher pack classes with the R7 reader (same method as R16) — bounded, not started |
| 5.3 Scorebug | Scaleform `.gfx` overlays (PS4 `overlays_2x.big`, gen7 `cacheoverlay.big`) — not decoded by any lane; text keys in the dictionary | inventory the `.gfx` members and their embedded image/text resources — new format work, not started |
| 5.4 Menu music | this handoff (gen7 complete at the table level; PS4 banks only) | card R17-MUS-1; PS4 selection stays compiled |

## 4. Unknowns (explicit)

1. Music-box `kind` values and the category semantics beyond the observed grouping. 2. Track titles/artists on any title. 3. PS4
context → track selection (compiled). 4. Runtime acceptance of a re-pointed context (card). 5. All of §3's formats.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r17_musicbox.py docs/research/overnight-2026-09-20/data/r17-musicbox.json   (≈ 5 s)
python local/runs/overnight-research-2026-09-20/probes/snd10_sbr.py "<…CUSA18182…/audio/music/musicbank.sbr>" --dump-fields
```
Executed (read-only): the music-box probe (twice — the record-table lookup was corrected from a fixed offset to header word 4), the
PS4 bank inventory and the `music.bin` string scan. **Not executed:** launches, writes, Studio binaries, audio decoding.
