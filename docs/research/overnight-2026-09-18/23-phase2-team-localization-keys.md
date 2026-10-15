# Handoff P2.3-X — team text in the dictionaries: key families keyed by `kOtt.artabbr`

Research lane, overnight 2026-09-18. Claim `p2.3-team-localization-keys`. Consumer: developer Phase 2.1/2.3
(team rename must touch DB strings *and* dictionary keys), LOC-W1 successor (free-text edit / key insertion).
Probes: `loc_families.py` (key-family census on all ten Xbox dictionaries), `team_loc_join.py` (join on Legacy).

Evidence: **observed** on the Legacy Xbox 360 anchor DB + dictionary (and the census on all ten); which
screen reads which family is **unknown** (queued for tomorrow: rename one key, see where it shows).

## The join

Team row field **`artabbr`** (`kOtt.RPbr`, 12-char inline string; e.g. `TOR`, `LETH`, `EASHL1`) is the suffix
of every per-team dictionary key. On Legacy (222 team rows):

| Key family (hash-folded; case varies by title) | Legacy hits | Text (Toronto) | Notes |
|---|---|---|---|
| `NHLTeamName_{artabbr}` | 220/222 | `Toronto Maple Leafs®` | full name (with ® on NHL teams) |
| `NHLCityName_{artabbr}` | 220 | `Toronto` | city |
| `NHLTeamName_Abbr3_{artabbr}` | 218 | `TOR` | 3-letter abbreviation (misses: `VXJ`, `EHC`) |
| `TEAMLINE1_{artabbr}` / `TEAMLINE2_{artabbr}` | 220 / 220 | `Toronto` / `Maple Leafs®` | two-line name (older `Teamline1_` spelling resolves by hash) |
| `TXT_NICKNAME_{artabbr}` / `TXT_NICKNAME_ALT_{artabbr}` | 220 / 220 | `Maple Leafs` / `Leafs` | NHL 14+ only (commentary/tweet nicknames) |
| `X_XLA_TEAM_{artabbr}` | 192 | `TOR` | scoreboard/ticker abbreviation (NHL 09+); 30 teams miss (e.g. `CGY`, `NJ`: keyed differently, e.g. `X_XLA_TEAM_NJD`? not chased) |
| `NICKLINE2_{artabbr}` / `_{n}` | 69 | `Maple Leafs®` | mostly NHL + national teams (numeric variants `NICKLINE2_52` = national team ids) |
| `NHLTeamName_{n}` / `NHLCityName_{n}` / `NHLTeamName_Abbr3_{n}` (n = 0..54) | 55 each | — | numeric-keyed duplicates for national/older teams (NHL 07 era; kept through Legacy) |
| `CITYLINE1_{n}` (35) | — | `Western` | conference/division-style captions (unclear) |

Joining on `abbrname` instead resolves only 134/222 (fails wherever the display abbreviation differs from
the art abbreviation, e.g. `ARZ` vs `PHX` art key) — so **`artabbr`, not `abbrname`, is the key**. The two
misses are the EASHL placeholder rows.

Per-title family census (`loc_families.py`): the families exist on every title 07–Legacy; case changes
(NHL 10–12 all-caps keys and texts), `Teamline1_` (mixed case, 08/09/13/14) coexists with `TEAMLINE1_`,
`X_XLA_TEAM_` appears from NHL 09, `TXT_NICKNAME[_ALT]_` from NHL 14, and NHL 07 has only the
`NHLTeamName/NHLCityName/Abbr3/TEAMLINE/NICKLINE/CITYLINE` set.

## Consequence for the team-rename workflow

A complete rename of an existing team on gen7 = (a) DB strings `fullname`, `shortname`, `abbrname` (inline,
fixed width 50/40/12 bytes) and (b) the dictionary rows `NHLTeamName_`, `NHLCityName_`, `NHLTeamName_Abbr3_`,
`TEAMLINE1_`, `TEAMLINE2_`, (`TXT_NICKNAME_`, `TXT_NICKNAME_ALT_`, `X_XLA_TEAM_`, `NICKLINE2_`) for the
team's `artabbr` — **free-text** changes, which LOC-W1's re-point cannot express (it can only alias another
team's text). Phase 2.3 therefore needs the pool-rewrite / text-encode writer that the batch deferred
(Huffman re-encode with the existing tree, or a pool append), the same capability 1.3 needs for league
labels. Changing `artabbr` itself re-keys every family at once (and the FE art? — `artabbr`'s use for asset
lookup is not observed; assets key on `artid`).

Which of the DB strings vs dictionary keys each screen shows is the first thing to read off tomorrow's
runtime pass (rename `NHLTeamName_TOR` only; then `fullname` only).
