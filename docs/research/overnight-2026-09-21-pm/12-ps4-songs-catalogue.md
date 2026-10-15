# Handoff R54 — PS4 team-song catalogue (Misc 5.4 / Teams 2.7): `songs.csv` names every selectable goal / power-play / win song on NHL 17–21 (title key, composer, genre); the DB `*songid` cells and the music banks do not join it by `audioid` or `index`

> **Counting note — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-10).** Distinct composers are 134 / 132 / 128 / 96 (NHL 21 / 19 / 18 / 17) when the `None` row's empty composer is excluded and names are trimmed and case-folded; the figures below counted the empty value. The developer's rule is adopted.

Research lane, 21 September 2026 pm, roadmap step 5.4 / 2.7 PS4 (the 2026-09-20 handoff 10 unknown 2 "track titles / artists on any
title"). Claim `research-r54-ps4-songs-catalogue`. Read-only; nothing game-verified.

Machine-readable: [`data/r54-ps4-songs-catalogue-join.json`](data/r54-ps4-songs-catalogue-join.json). Probe `r54_ps4_songs_catalogue_join.py`
(≈ 2 min).

## Plain-language summary

The NHL 17–21 installs ship a song catalogue (`creation_zone_csv/songs.csv`: 191 / 245 / 637 / 356 / 422 rows) with an identifier, a
`$TXT_SONG_…` title key, the composer, a genre id (12 genres named in `songgenres.csv`: rock, hard rock, pop, punk, electronic, dubstep,
hip-hop, country, blues / jazz, classical, misc, none), an art id and an `audioid`, plus goal / power-play / win (and, from 19, intro)
eligibility flags — the track titles the 2026-09-20 handoff could not locate, for the selectable team-song set. Two things do *not*
follow: the database's `goalsongid` / `winsongid` / `powerplaysongid` cells (6 non-zero rows per title, values 1 / 113 007 / 154 810 /
164 509 / 235 706) are not catalogue `audioid`s nor `index`es, and no sound bank under `audio/` carries the catalogue `audioid`s as sample
ids (the music banks' ids are 44–1 582 and 13 004–13 803; `musicbank.sbr` DSET 0 has 411 records against 422 catalogue rows — a
row-order or hashed relation is possible, untested). So the catalogue is a *listing* input (what the user can pick and what it is called),
not yet the binding from a team row to a bank sample.

## 1. Catalogue

| Title | rows | columns | `audioid` range | composers | flags (all rows eligible for goal / PP / win on every title; `isintrosong` 0 everywhere) |
|---|---|---|---|---|---|
| NHL 17 | 191 | `index identifier artid audioid composer genre isgoalsong ispowerplaysong iswinsong name` | 67–251 105 | 97 | 191 / 191 / 191 |
| NHL 18 | 245 | same | 67–276 003 | 129 | 245 / 245 / 245 |
| NHL 19 | 637 (the file repeats its header rows; 637 data rows) | + `isintrosong` | 60 001–2 784 101 | 133 | 637 / 0 / 637 / 637 |
| NHL 20 | 356 | same | 101 406–2 784 101 | 133 | 356 / 0 / 356 / 356 |
| NHL 21 | 422 (`index` −1…510) | same | 101 406–2 958 139 | 135 | 422 / 0 / 422 / 422 |

`artid` 53–61 (a genre / cover art id; 60 on 83 rows), `genre` 0–11, `name` = `$TXT_SONG_<IDENTIFIER>` dictionary keys. NHL 15 / 16: no
catalogue (their music tables are the 2026-09-20 handoff 10 gen7 / bank readings).

## 2. Joins (negative, recorded)

| Join | Result |
|---|---|
| `exhibitionarena.*songid` (17) / `exhibitionteams.*songid` (18–21) → catalogue `audioid` | 0 of the 2 / 6 non-zero cells on every title |
| → catalogue `index` | 0–2 (the value `1` only — coincidence) |
| catalogue `audioid` → `.SID` of any bank under `audio/` | none beyond coincidental low ids (`playercallibank`, `fesfxbank`, `pbp_samples`); the `music/*.sbr` banks have no hit |

The six non-zero DB cells (three teams × three columns on 18–21; two arenas on 17) carry the same four large values across titles —
a fixed test / created-team set rather than a live binding. Candidate for the catalogue → sample step: `musicbank.sbr` DSET 0 (411
records, `.SID` 799–1 528) by row order or by a hash of `identifier` — not tested; the `music_ptc.big` patch data (as for horns) is the
other candidate.

## 3. Consequences (developer)

- Misc 5.4 / Teams 2.7 PS4 17–21: a **song catalogue listing** (identifier, localized title via the `$TXT_SONG_*` key, composer, genre
  name, eligibility flags) is implementable from the CSV; label it "selectable songs (catalogue)", not "the team's song".
- The team / arena `*songid` cells stay raw ids with "no catalogue join" — do not resolve them through `audioid` or `index`.
- Replacement / preview: none (the sample binding is open; the music banks are EA-Opus / EA-MP3 per the 2026-09-20 reading — preview
  would follow handoff 07's decode contract once a binding exists).

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r54_ps4_songs_catalogue_join.py docs/research/overnight-2026-09-21-pm/data/r54-ps4-songs-catalogue-join.json   (≈ 2 min)
```
Executed three times (encoding tolerance for a non-UTF-8 byte in the NHL 21 file; the NHL 19 repeated-header rows).
