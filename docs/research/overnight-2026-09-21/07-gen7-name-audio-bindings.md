# Handoff R35 — Gen7 name-call bindings: which database audio id keys which speech-module tag table (PS3 anchors 09–Legacy; PS3 = Xbox 360 for the speech data)

Research lane, 21 September 2026 — Phase 2.7 (team PBP / PA calls) and 3.5 (player name calls), consumed early because handoff 04's model
made the join a one-probe question. Claim `research-r35-gen7-audio-id-bindings`. Nothing game-verified; read-only; no writer.

Machine-readable: [`data/r35-gen7-audio-id-bindings.json`](data/r35-gen7-audio-id-bindings.json). Probe
`local/runs/overnight-research-2026-09-21/probes/r35_gen7_audio_id_bindings.py` (≈ 10 s; the repo `anchors/ps3/<title>/databases/nhlng.db`
+ metadata, the gen7 speech trees on X:).

## Plain-language summary

The database carries several "audio id" columns, and the commentary system carries tag tables keyed by the values the game passes it as
event parameters (handoff 04). Joining them shows two distinct systems: the **play-by-play** commentators' player names are keyed by
`playernames.audioid` (the `player_ID` tag: 99.9 % of the ids have samples on 09 / 14 / 15 / Legacy, 85 % on 10–13), and the **rink
announcer**'s player names by `exhibitionplayerbiotable.audioid` (the announcer module's `playerid` tag: 81–92 % of the roster has a sample —
the rest are players with no recorded name). Team and arena audio ids (`exhibitionteams.audioid`, `exhibitionarena.chant/city/arenaaudioid`)
are values inside the sample-group id space on every title and — the discriminating join — inside the `team_ID` tag's key set on the
titles where that table exists (207 / 211 on NHL 15). So "which lines say this club's / this player's name" is a static listing on every
gen7 title, and a reassignment (point a roster player at another recorded name) is a database edit validated against the tag key set — the
same shape as the PS4 `pbp_keywords.bin` finding of 2026-09-20 handoff 07 §3, now with the gen7 mechanism understood.

## 1. Measured joins (PS3 anchors; the speech tables are identical on Xbox 360 per handoff 04)

| Column | Values (distinct, range) | Best-matching tag table | Containment | Second reading |
|---|---|---|---|---|
| `playernames.audioid` | NHL 09 1 902 (10 000–12 399); 10 / 11 2 242 (10 000–12 989); 12 3 737; 13 4 046 (–14 678); 14 3 875 (30 000–35 041); 15 1 172 (30 004–34 059); Legacy 1 683 (30 004–34 370) | **pbp `player_ID`** (tag 2 on 13+, 2164 on 09–12) | 09 **1 900 / 1 902**; 10 1 898 / 2 242; 11 1 898 / 2 242; 12 3 165 / 3 737; 13 3 464 / 4 046; 14 **3 862 / 3 875**; 15 **1 169 / 1 172**; Legacy **1 681 / 1 683** | not in KWD0 (0 %) — the keyword index is not the player index |
| `exhibitionplayerbiotable.audioid` | 3 750–5 413 distinct (1–14 591) | **ann `playerid`** (tag 2163 of the announcer module; named by the NHL 09 `ann_eng_speecheventsubscr.xml`: `playerid` 2163, `teamid` 2164, `numberid` 2165) | 09 70 %, 10 76 %, 11 83 %, 12 83 %, 13 **92 %**, 14 92 %, 15 84 %, Legacy 81 % | 0 / 44 in `playernames.audioid` — a different id space; 17 % in KWD0 on NHL 15 (coincidental range overlap) |
| `exhibitionteams.audioid` | 151–212 (1–509; Legacy 1–1 378) | pbp secret-name (sample group) ids: **100 %** on every title (trivial: every small integer is a group id); pbp `team_ID` tag: **207 / 211** (NHL 15), 206 / 212 (Legacy) — handoff 02 §4 | as stated | KWD0 100 % on 15 / Legacy (2026-09-20 handoff 07 §3) |
| `exhibitionarena.chantaudioid` / `cityaudioid` / `arenaaudioid` | 52–63 / 80–156 / 81–158 (1–136 / 1–511 / 1–701; Legacy city 1–1 999) | secret-name ids 100 %; `team_ID` tag 130 / 155 and 130 / 157 (NHL 15, handoff 02 §4) — the arena tags are not `team_ID`; the announcer / pbp tag that keys them is **not identified** (no `arena_ID`-like parameter contains them better) | — | KWD0 100 % on 15 / Legacy |
| `exhibitionteams.audioid2` (Legacy) | 1 value (4 095) | sentinel | — | — |

Residuals kept in the JSON (`values_outside_best`, first 40 per column): e.g. the 3 `playernames.audioid` values without a `player_ID` table on
NHL 15, the 344 on NHL 10 / 11 (names present in the DB but not recorded), the roster players without an announcer sample.

## 2. What this gives the developer (read-only listing; edit = DB value validated against a key set)

1. **Player (PBP):** `playernames` rows (`name`, `isfirstname`, `bank`, `index`, `audioid`) are the recorded-name catalogue; the samples of a
   name = graffiti `(player_ID, audioid, NumTags = n)` tables → sample ids → `SBKR` → stream (handoff 05 for codec / alias facts). A
   `playernames.audioid` without a table = "name in the catalogue but no recording" (list it, do not offer it).
2. **Player (announcer):** `exhibitionplayerbiotable.audioid` → ann graffiti `(playerid 2163, value, n)` → `ann_eng_samples`; players outside
   the key set = "no announcer recording" (8–30 % per title).
3. **Team / arena:** `exhibitionteams.audioid` → pbp `(team_ID, value, n)` (name-call lines: the 2026-09-20 handoff 07 §3 numbers on NHL 21
   have their gen7 counterpart here); the arena ids' tag is open (Q-R35-1) — list them as sample-group ids only.
4. **Reassignment contract (proposal):** changing a roster player's `exhibitionplayerbiotable.audioid` or a team's `audioid` to another value
   present in the corresponding key set is a plain DB cell edit with no file dependency; refuse values outside the key set; the runtime
   effect (the announcer calling the new name) is card R35-AUD-1. No sample file is touched.

## 3. Open (Q-R35-1)

The tag that keys `exhibitionarena.*audioid` on gen7 (the announcer's `teamid` 2164 or a pbp arena parameter — neither contains the values
better than the trivial group-id set); the meaning of `playernames.bank`; PS4 has no graffiti file (its speech uses the DSET banks and
`pbp_keywords.bin`) — the PS4 name binding stays the keyword-id finding of 2026-09-20 handoff 07 §3 (teams) and open for players (Q-R14-2).

## 4. Card R35-AUD-1 (deferred; PS3 NHL 15, after a DB edit path exists)

Baseline: a copied roster DB. Treatment: set one roster player's `exhibitionplayerbiotable.audioid` to another player's value that is in the
announcer `playerid` key set. Observable: the arena announcer names the other player on a goal / three-stars. Control: an unedited copy.
Verdict bounds: changed = the announcer keys names by this column; unchanged = no cause identified (a second copy of the DB, a cache, or a
different consumer) — next discriminator: the same edit on `playernames.audioid` for the play-by-play line. Restore: the original DB by hash.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r35_gen7_audio_id_bindings.py docs/research/overnight-2026-09-21/data/r35-gen7-audio-id-bindings.json   (≈ 10 s)
```
Executed once (all 8 titles) plus the announcer-parameter check against the NHL 09 XML (scratch). Not executed: writes, launches.
