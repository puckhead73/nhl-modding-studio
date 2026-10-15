# Handoff R53 — Gen7 Players: portrait (`artid`) and head (`headid`) file containment per title, NHL 09–Legacy (Xbox 360) and PS3 15 / Legacy; the `hasportrait` flag measured against the files; bespoke heads 100 % present on 10–Legacy

> **Reading note — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-8).** Bucket counts must exclude `playerheads/silhouettes/` (four pool stand-ins; NHL 09 becomes 2 924 / 218 with them excluded). A consumer needs three classes — bespoke / pool / *other* (head id 0 or below the era floor) — not two; on Legacy the two rows flagged without a file are Gergo Nagy and Maurizio Colella (developer join, receipt 14).

> **Correction 2026-09-27 05:07 ([2026-09-26 handoff 17](../overnight-2026-09-26/17-portrait-head-reassignment.md) §1):** the summary sentence "60 000+ on NHL 14–Legacy" is wrong for NHL 14 — its roster pool heads are 10 000–10 182 (no roster head id reaches 60 000), as this handoff's own table row ("< 60 000") shows; 60 000+ applies from NHL 15. Studio's floor rule is data-driven and already correct.

Research lane, 21 September 2026 pm, roadmap step 3.x gen7 (prompt step 5 "Players: identities / joins, portraits / heads / body assets").
Claim `research-r53-gen7-player-assets`. Read-only directory listings of the loose trees joined to the anchors; nothing game-verified.
Extends the Legacy-only reading of the 2026-08 memory (`ubPc.lOnr` = portrait id, `pPaD` = head id, bespoke iff equal) to every gen7 title.

Machine-readable: [`data/r53-gen7-player-asset-containment.json`](data/r53-gen7-player-asset-containment.json) (per title: buckets, file
counts, per-class presence, flag vs file cross-tab, residual lists). Probe `r53_gen7_player_asset_containment.py` (≈ 1 min).

## Plain-language summary

On every Xbox 360 title from NHL 10 to Legacy, every roster player's head model id resolves to a shipped head file — the 246–270 bespoke
heads (head id = portrait id) and the 3 663–5 385 pool heads alike — and every bespoke player's portrait archive exists (one NHL 10 miss:
Schenn; NHL 13: Grigorenko). NHL 09 keeps its heads under a different file name (`head_<id>`) and lacks 5 of 256 bespoke head files
(Stastny, Staal, Malkin …) and 5 portraits. The pool-head id space moves: 10 000–10 181 on NHL 09–13, 60 000+ on NHL 14–Legacy. The
database's `hasportrait` flag is not simply "a portrait file exists": on NHL 13 93 players and on Legacy 127 players are flagged 0 but
have the file (and 2 Legacy players are flagged 1 without one) — the flag is a display switch the developer must not derive from the
file, and vice versa. The portrait pool on disk is much larger than the roster (214 unused archives on 09 up to 4 724 on Legacy), so a
portrait reassignment has a large shipped target set; a head reassignment has 129–457 unused shipped heads. PS3 NHL 15 / Legacy carry
exactly the same sets as Xbox 360.

## 1. Per title (Xbox 360 unless stated; anchors' rosters)

| Title | players | portrait archives (buckets `p0_4000` / `p4001_8000` / `p8001_12000`) | head files (`rendering/player`) | bespoke (head = portrait) | bespoke: head file / portrait file | pool class + id range | pool: head file | `hasportrait` 1 → file / 0 → file | unused portraits / heads |
|---|---|---|---|---|---|---|---|---|---|
| NHL 09 | 3 968 | 2 920 (2 428 / 396 / 96; + `portrait.big`, `pportrait.big`, `h0/h1.big`, `1286.big` loose files) | 480 (`head_<id>`) | 256 | **251** / 251 (Stastny, Staal, Malkin, Zherdev, Andersson … listed) | "other" 10 000–10 099 (3 712) | 3 712 / 3 712 | 2 705 / 2 705 · 1 / 1 263 | 214 / 129 |
| NHL 10 | 3 933 | 3 637 | 742 (`player_<id>` + `head_<id>`) | 270 | 270 / 269 (Schenn) | 10 000–10 099 (3 663) | all | 2 954 / 2 954 · 4 / 979 | 679 / 372 |
| NHL 11 | 5 310 | 5 397 | 821 | 255 | 255 / 255 | 10 000–10 181 (5 055) | all | 4 269 / 4 269 · 1 / 1 041 | 1 130 / 397 |
| NHL 12 | 5 395 | 6 158 | 795 | 253 | 253 / 253 | to 30 100 (5 142) | all | 4 427 / 4 427 · 1 / 968 | 1 730 / 363 |
| NHL 13 | 5 412 | 6 823 | 817 | 268 | 268 / 267 (Grigorenko) | 10 000–10 181 (5 144) | all | 4 298 / 4 298 · **93** / 1 114 | 2 433 / 375 |
| NHL 14 | 5 374 | 7 678 | 841 | 262 | 262 / 262 | < 60 000 (5 112) | all | 4 542 / 4 542 · 0 / 832 | 3 138 / 400 |
| NHL 15 (= PS3) | 5 534 | 8 362 | 861 | 259 | 259 / 259 | ≥ 60 000 (5 275) | all | 4 358 / 4 358 · 0 / 1 176 | 4 004 / 421 |
| Legacy (= PS3) | 5 632 | 9 224 | 904 | 246 | 246 / 246 | ≥ 60 000 (5 385) + 1 other | all | 4 376 / 4 378 (2 flagged without file: Nagy …) · **127** / 1 254 | 4 724 / 457 |

Silhouettes: `silhouettes/p60000..60003.big` (4) — the pool heads' portrait stand-ins (459 / 466 pool players on 15 / Legacy map to one).

## 2. Contract input (developer, Phase 3.x gen7)

- **Portrait / head listing:** resolve `artid` → `fe/ion/artassets/playerheads/p<bucket>/p<artid>.big` (bucket by range 0–4 000 /
  4 001–8 000 / 8 001–12 000) and `headid` → `rendering/player/player_<headid>.rx2` (NHL 10+; `head_<headid>.rx2` on 09 / 10; `.rpsgl` on
  PS3); classify bespoke / pool by `headid == artid` and the era's pool range (10 000+ on 09–13, 60 000+ on 14+ — Studio's 60 000 rule is
  wrong for 09–13).
- **`hasportrait`:** display it as the DB flag it is; never compute it from the file and never write it from a file check (93 / 127
  contradictions shipped).
- **Reassignment:** an existing-id reassignment validates against the title's shipped id sets (the unused shipped portraits / heads are
  legitimate targets); a new id = a new archive / container (portrait archives are BIGF + RefPack + XPR2 per the memory note; head
  containers per the mesh work-stream — both outside this handoff).
- **Refusals:** NHL 09 bespoke players without a head file (5) and portraits (5) — list as "shipped without asset".

## 3. Not claimed

What the game shows for a flagged-0 player who has a file (runtime); the PS3 08–14 trees (archive-only or not walked); NHL 07 / 08 (no
`exhibitionplayerbiotable` `artid` / `headid` census here); hair / facial-hair textures (`czhead`, `player/hair_*` — not joined).

## 4. Addendum — the portrait archive chain per title (r57, 60 archives per title from `p0_4000`)

| Title | outer | members | texture member | plain size |
|---|---|---|---|---|
| NHL 09 / 10 | `BIGF` | `0` Apt (296 / 240 B), `1` Apt (32 B), `<n>` texture, `<n+1>` 288-byte record (magic bytes `00 00 01 20`) (the Apt geometry record of the C6 receipt) | **raw `RW4xb2` texlib** | 135 168 |
| NHL 11 | `BIGF` | + empty `sg1` / `sg2` markers | **RefPack → `RW4xb2`** | 135 168 |
| NHL 12–Legacy | `BIGF` | same + `sg1` / `sg2` | **RefPack → `XPR2`** (a few archives on 13 / 14 also RefPack the Apt member) | 131 168 (22–34 of 60) or 262 240 (26–39 of 60) |

The same container family as the league-logo packages (BIGF → Apt record + texture member; RefPack'd XPR2 from NHL 12), so the developer's
C1 / C6 logo routes are the portrait routes with the per-title texture member class above; NHL 09–11 portraits are `RW4xb2` texlibs
(handoff 02's rules), not XPR2. Probe `r57_gen7_portrait_archive_chain.py` (≈ 10 s), data `r57-gen7-portrait-archive-chain.json`.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r53_gen7_player_asset_containment.py docs/research/overnight-2026-09-21-pm/data/r53-gen7-player-asset-containment.json   (≈ 1 min)
python local/runs/overnight-research-2026-09-21-pm/probes/r57_gen7_portrait_archive_chain.py docs/research/overnight-2026-09-21-pm/data/r57-gen7-portrait-archive-chain.json --per-title 60   (≈ 10 s)
```
Executed twice (the second run adds the NHL 09 / 10 `head_<id>` naming).
