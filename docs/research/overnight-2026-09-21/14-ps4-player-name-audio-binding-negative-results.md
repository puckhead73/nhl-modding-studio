# Handoff R40 — PS4 player-name audio binding (Q-R14-2): five hypotheses tested and excluded on NHL 15–21; what the negative results rule out and the next discriminators

Research lane, 21 September 2026, Phase 3.5 PS4 (players' name audio; Q-R14-2 of the 2026-09-20 campaign). Claim
`research-r40-ps4-player-name-audio`. Read-only over the PS4 anchors and the extracted `audio/speech/eng` + `audio/rink_announcer/eng`
trees; nothing game-verified; no writer. **This handoff records negative results**: the binding stays open, but the hypotheses below need
not be retried, and one earlier near-claim is withdrawn before anyone consumes it.

Machine-readable: [`data/r40-ps4-player-name-keywords.json`](data/r40-ps4-player-name-keywords.json) (hypothesis 1 per title). Probe
`r40_ps4_player_name_keywords.py` (≈ 3 min); hypotheses 2–5 were scratch tests whose logic is stated below.

> **Correction R2 (coordinator review of 2026-09-21, applied 2026-09-21 17:00 MDT; campaign
> [overnight-2026-09-21-pm](../overnight-2026-09-21-pm/00-ATLAS.md)).** What §1 establishes is narrower than "the player rows' `audioid`
> is not consumed": each row of §1 is one **direct-join hypothesis** — a stated key on one side, a stated id space on the other, an
> equality test over the stated population — and each was rejected by a density check. That excludes those five direct joins on NHL 15–21.
> It does **not** exclude a partial (subset), namespaced (`bank` + id), conditional (only some rows), transformed (hashed / offset by a
> non-constant / table-relocated) or code-resident use of `audioid`; none of those was tested, so "no established binding" is the whole
> claim and "not consumed" is not claimed. The developer refusal to implement a player-name binding on PS4 stands on exactly that
> ground (an unestablished binding is not implemented). Per hypothesis the exact statement is: **H1** `exhibitionplayerbiotable.audioid`
> ∈ `pbp_keywords.bin` ids (population: all 7 247 / 5 555–7 236 players per title; evidence: dense key space makes membership
> non-discriminating, 64–85 ids collide with team keywords, 6 300 ids lie above the keyword maximum); **H2** `audioid` = `.SID` of
> `pbp_samples*.sbr` (population: all players; evidence: hit fraction per DSET equals the bank's id density); **H3** `audioid` = `.SID` of
> `rink_ann_samples*.sbr` (population: all players, NHL 15–21; evidence: 0–64 hits on 15–20; the NHL 21 hits are confined to a contiguous
> id range shared with unrelated columns); **H4** `surnameid → playernames.audioid` as H1–H3 (population: 2 089 rows carrying one);
> **H5** `audioid + K` / `surnameid + K` into the announcer pools (evidence: no sharp difference peak). The §4 discriminators and the
> user card R40-AUD-1 are restated in the pm campaign's USER-CARDS as observation-only cards with positive controls. Text preserved below.

## Plain-language summary

On the old consoles a player's name audio is found through the graffiti table (handoff 07). On PS4 that table does not exist and the
commentary picks name samples through a keyword file whose ids the *team* rows carry directly. The player rows carry an `audioid` too,
but tonight's tests show it is **not** a keyword id, **not** a commentary sample id and **not** a rink-announcer sample id: every apparent
match is explained by dense id spaces (every number up to 5 363 is a keyword; the announcer's 2019+ sample ids happen to be contiguous
over the same range as the 2019+ player ids). The `playernames` table's own `audioid` and the `surnameid` link behave the same way. So
the developer must keep the PS4 player-name listing at "no established binding" (teams and arenas are bound; players are not), and the
next step is a table search in the executable or a listening card — not another id-space comparison.

## 1. Hypotheses and results (NHL 21 unless stated; the JSON has 15–21 for H1)

| # | Hypothesis | Test | Result |
|---|---|---|---|
| H1 | `exhibitionplayerbiotable.audioid` = keyword id of `pbp_keywords.bin` (as for teams) | membership + what the keyword tags | **excluded as evidence**: the PS4 keyword table is dense (ids 1 … `keyword_max` all present: 5 363 / 5 363 on 21), so the 798–1 043 players whose audioid ≤ max match trivially; 64–85 of them share the id with a team (Afanasenkov 5 = Carolina 5, Allen 12 = Florida 12 — alphabetical 2008-era player ids); the other 6 300 players (audioid 5 364 … 34 797) have no keyword at all; NHL-club coverage 222 / 899 |
| H2 | `audioid` = commentary sample id (`.SID` of `pbp_samples*.sbr`) | per-DSET hit fraction vs. expectation | excluded: hits spread over 1 547 DSETs at the bank's id density (88 %); no DSET is > 95 % player ids (the 2026-09-20 R14 duration test agreed) |
| H3 | `audioid` = rink-announcer sample id (`rink_ann_samples*.sbr`) | per-title hits by DSET | excluded: **0–64 hits on NHL 15–20** (of 5 555–7 236 players); on NHL 21 1 454 hits in DSETs 69 / 70 — but only for `audioid` ≥ 18 505, exactly the range where those DSETs' ids run contiguously (their ids 19 900–90 442 also equal `game_id` / `artid` / `playerid` at the same 0.77 rate: a range coincidence, not a binding) |
| H4 | `exhibitionplayerbiotable.surnameid` → `playernames` row (7 247 / 7 247 resolve) → `playernames.audioid` (2 089 rows carry one; 30 000–34 796; `bank` 0–8) as keyword / commentary sample / announcer sample | membership + DSET spread | excluded: 0 keywords; announcer hits spread over 15 DSETs at density; commentary hits at density |
| H5 | constant offset `audioid + K` or `surnameid + K` into the announcer name pools | difference histograms (5 111 × 9 210 pairs) | no sharp offset (plateaus of contiguous runs only: 1 242 vs. 532 random at best) |

Also measured: no DB column enumerates the ≈ 3 900 non-team name-call keywords of NHL 21 better than trivially dense ones
(`playernames.index`, `surnameid`); the NHL 21 `pbp_metadata.xml` carries only `speaker` (0 / 1 / 2 / −1) and 89 sideline-text entries —
no name keys; `pbp_pickups.xml` maps sample → pickup id (not names).

## 2. What is established on PS4 (unchanged from the 2026-09-20 handoff 07 §3)

Teams (`exhibitionteams.audioid`, `audioid2`), arenas (`arenaaudioid`, `cityaudioid`) and chants (`chantaudioid`) are keyword ids that
tag their name-call and mention samples; the commentary sentence → DSET chain is complete (2026-09-20 handoff 02 §4.1); the PS4 phrase
records carry 12-byte parameter items (`u8 index │ u8 index + 1 │ u8 index │ 0 │ 1 │ ff │ 0 │ u32 tag`), i.e. the gen7 event-reference
shape with a 32-bit tag — the tag values do not match FNV-1 / FNV-1a / CRC32 of the gen7 parameter names (case-insensitive tried), so the
PS4 tag namespace is its own (open).

## 3. Withdrawn before consumption

During the work an intermediate reading ("the 906 players with audioid ≤ 5 363 are the recorded names; Stastny 1292 → 2 samples of
0.49 s") was written into the R40 probe docstring; it was **withdrawn** on the density check (H1) and the docstring corrected. No
handoff, queue row or claim carried it.

## 4. Next discriminators (not done)

1. The PS4 executable: the 2026-09-18 `p1bs_eboot_*` probes already read `eboot.bin`; a table pairing the 7 247 player `audioid` values
   (or `game_id`) with keyword ids ≤ 5 363. **Tried at the end of the session (bounded):** a scan of the NHL 21 `eboot.bin` (104.8 MB) for
   runs of ≥ 40 consecutive u16 / u32 values that are player audioids, at strides 1–4 — the only long runs (1 496 / 389 entries at stride
   3) are code with repeated constants (757 / 1 129 as the first word), not a table. A pointer-relocated or hashed table would not show
   up this way; a disassembly-level search is the next step, not a value scan.
2. The commentary context / event layer: `pbp_speecheventsubscr.bin` (`0TVE`) parameters of the goal / shot events — if a `player_name`
   parameter is typed as a *keyword* the resolution is in the game code (id → keyword through the roster), not in data.
3. A user listening card (after the developer ships PS4 team-name reassignment): rename one AHL club to another club's keyword and one
   player's `audioid` to another player's, in a copied DB; observable = whether the commentary's *player* name changes — bounded verdict
   "audioid is consumed by the commentary" / "is not" (separate from *how* it resolves). Listed in USER-CARDS as R40-AUD-1.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r40_ps4_player_name_keywords.py docs/research/overnight-2026-09-21/data/r40-ps4-player-name-keywords.json   (≈ 3 min)
```
H2–H5 used the 2026-09-20 `snd10_sbr.py` / `pbp_sentences.py` / `r14_audio_keywords.py` readers in scratch (removed); each test is a
few lines against the sets named above.
