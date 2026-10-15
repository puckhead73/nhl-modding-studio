# Consolidated receipt 4 — research lane, overnight 27 September 2026 (04:40 → 06:12 MDT, system clock; updated 06:21 with handoff 26 and addenda)

Prompt [`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) (review
[`research-20260926-r119-r121-review.md`](../../validation/research-20260926-r119-r121-review.md) + the October 1 overnight extension). Beta
blocker view: [BETA-HANDOFF.md](BETA-HANDOFF.md). Everything below is researcher-verified and **pending independent review**; nothing is
desktop-observed, listened to or game-verified; no application, defs or bindings file was edited; nothing staged, committed or pushed.

## Plain-language summary

I first corrected last night's nationality claims: Studio's player page was already right on NHL 08 / 09, and the real problems were on
NHL 10 and NHL 11 / 12 (the developer has since fixed them). The night then produced developer-ready inputs for several beta operations:
exact rules for generating grouped league schedules; what it takes to give a player a different existing portrait or head; what each goalie
gear number points at (including that NHL 08 / 09 keep the mask paint in a column called `maskcolor`); full evidence that the Xbox NHL 09
sounds can be previewed with the new XMA helper once one header check is switched off; and the measured rules that would let Studio resize
team and league logos on NHL 07–11. It also found that the Lines page labels skater styles wrongly on NHL 07–10, and confirmed that the
Recompiled install ships byte-identical game data to Xbox NHL Legacy.

## Package receipts (handoffs)

| # | Handoff | Result | Queue |
|---|---|---|---|
| 0 | [00d](00d-checkpoint-r119-r121-review.md) | C16 scope corrected (NHL 08 / 09 field view uses `GEN6_COUNTRIES`), England vs Great Britain narrowed, D2 / D3 reclassified, R121 made deterministic (counts unchanged), C15 disposition | C15, C16 |
| 2 | [15](15-native-label-consumers.md) | style codes per era from position split + dictionary sets; Lines-page / roster labels wrong on NHL 07–10 (S1 / S2); inferred NHL 10 table; NHL 07 country residual only code 78; §7 eye colour negative | C17 |
| 1 | [16](16-schedule-generation-contract.md) | grouped / matrix generation contract Q1–Q12 / R1–R10; 27 / 27 vectors; replay on 144 shipped tables, 0 failures | C18 |
| 3 | [17](17-portrait-head-reassignment.md) | portrait / head operation tables; texlib from NHL 13, `stockplayerheads` from NHL 12, PS4 `headtexture` = `headid`; card P34-1 | C19 |
| 4 | [18](18-goalie-gear-chain.md) | goalie gear value → file / catalogue per family; NHL 08 / 09 paint = `maskcolor`; team mask slots; PS4 pair + handedness keys; §7 skater gear 08–14 | C20 |
| 5 | [19](19-audio-admission-residuals.md) | audio admission matrix; A1 PS3 NHL 09 / 10 / 12 / 13 music contract-ready but not admitted | C21 |
| 6 | [20](20-gen7-board-art-venue-packages.md) | gen7 board / ring / jumbotron art per arena venue package; material → raster undecoded (four looks, negative scope) | C22 |
| + | [21](21-recompiled-source-identity.md) | Recompiled install = X360 Legacy bytes (20 / 20 files); archived main DB = X360 Legacy anchor | C23 |
| + | [22](22-animation-assignment-fields.md) | celebration / stance / skating-style fields (PS4 only; NHL 21 reshapes `skatingstyle`) | C24 |
| + | [23](23-scorebug-team-assets.md) | no team-keyed scorebug textures (PS4 21, X360 15 / Legacy) | C25 |
| 5+ | [24](24-x360-09-xma-sweep.md) | X360 NHL 09 XMA: 471 / 471 streams, 1 983 / 1 985 in-bank exact; R84 header refusal must be off on NHL 09 | C26 |
| + | [25](25-old-logo-package-resize-inputs.md) | logo resize inputs X360 07–11 / PS3 08–11: Apt quad rule holds; ARGB8 RW4xb2 descriptor rule 6 926 / 6 926 | C27 |
| + | [26](26-nhl08-11-group-names.md) | NHL 08–11 conference / division names: executable lists recovered; conferences positional; division binding neither positional nor reversed; consumer trace negative | C28 |
| + | [20 §6–7](20-gen7-board-art-venue-packages.md) | **B13 decoded**: material → raster = u32 at +256 of the preceding `0xEF0001` chunk (4 628 / 4 728 Legacy materials; every board-ad material resolves; PS3 identical); per-arena venue-art list for 360 packages; §8 same-size write inputs (DW8 pairing, no checksum); §9 byte-identical archive copies in `cacherender.big` | C22 |
| + | addenda | 17 §9–10 PS3 NHL 08–14 archive-held and X360 NHL 07 / 08 portrait / head containment (PS3 10–12 add `head_<id>_cm`; NHL 07 / 08 heads without files unexplained); 18 §8 PS3 NHL 08–14 goalie gear = X360; 20 §5 PS3 venue packages = X360; 23 §3 PS4 NHL 15–20 scorebug names; 25 §5–6 PS3 08–11 and X360 07 logo packages; 26 §5 consumer-trace negative | C19, C20, C27 |
| 7 | [BETA-HANDOFF.md](BETA-HANDOFF.md) | 15 blocker rows with class, lane, smallest action, dependency | — |

Durable checkpoint: [CHECKPOINT-2026-09-27.md](CHECKPOINT-2026-09-27.md). Queue: [`overnight-2026-09-25/DEVELOPER-QUEUE.md`](../overnight-2026-09-25/DEVELOPER-QUEUE.md) C1–C28.

## Developer-consumable items, in priority order

1. **C26** — admit X360 NHL 09 EA-XMA with the count rule only; disable the R84 packet-header refusal on NHL 09; refuse the two short
   in-bank records; rerun a sample through the built helper (the developer's current XMA work).
2. **C18** — the user-supplied pair-quota / partition generator the developer plan authorises: Q / R refusal codes, vectors, replay.
3. **C17 S1 / S2** — scope `player-style.toml` and the Lines rail constant per title (wrong labels on NHL 07–10 today).
4. **C20 G1–G3** — goalie gear: `maskcolor` / `hascustommask` on NHL 08 / 09, team mask slots, PS4 (`maskid`, `masktype`) validation.
5. **C21 A1** — add the PS3 NHL 09 / 10 / 12 / 13 music tuples (R82 vectors).
6. **C27** — ARGB8 RW4xb2 resize + NHL 07–11 package route (Apt rewrite unchanged; re-wrap per title).
7. **C19** — portrait / head reassignment refusal list (behind card P34-1 if selection evidence is required first).
8. **C23** — cite measured Recompiled database identity where receipts state coverage.
9. **C22 / C24 / C25** — neutral listings only (board-art gallery, animation fields, scorebug textures).
10. **C28** — NHL 08–11 divisions: neutral member-list labels; conference names by position only as "inferred".

## Coverage and executed checks

Sources: the 27 anchor directories (PS3 aliases of X360 measured once; `anchors/ps4/nhl20` reported as the NHL 19 base), the genuine NHL 20
(`nhl20-ps4-cusa15439`), NHL 19 patch 56, NHL 15 PS4 patch 00, the Xbox 360 loose trees NHL 07–Legacy, PS3 NHL 08–11 / 15 / Legacy trees
and archives, the PS4 installs (vaults, EB archives), the Recompiled install, the gen7 / PS4 dictionaries named per handoff. Probes
`r122*`–`r132d` in `local/runs/overnight-research-2026-09-26/probes/` (each handoff lists its command and how often it ran, including
discarded first runs). Oracle for audio: local FFmpeg 8.1.1 (not Studio's helper).

## Withdrawn or corrected conclusions

- C16 / handoff 14: "NHL 08 / 09 field views use the Legacy labels" withdrawn; Croatian / French / Slovak examples are NHL 10 only; "all 21
  teams match from NHL 13" narrowed to stable codes; D2 / D3 reclassified; birth-state = continuity only (00d).
- R121: rankings deterministic (one tied top destination changed; counts unchanged).
- R53 summary: NHL 14's pool heads are 10 000–10 182, not 60 000+ (dated note).
- Receipt 13 describes X360 NHL 07–10 logo packages as "RefPack'd packages": on the **loose trees** only NHL 08's are whole-file RefPack;
  NHL 07 / 09 / 10 are plain BIGF (handoff 25 §2, §6). The receipt may describe archive-held or fixture copies — not compared; recorded
  for the developer, not edited there.
- Own mid-run corrections (never published wrong): R122 identifier canonicalisation, R124 pool floor, R127 dictionary target, R132b pitch
  word; handoff 21 had two control characters introduced by a patch — repaired within minutes, and `safe_patch.py` now refuses newly
  introduced control characters.

## Remaining October 1 blockers (details in BETA-HANDOFF.md)

Grouped schedule generation (coordinator acceptance → developer, B1); style-label defect (B2); creation / cloning / deletion (user decision
+ runtime, B4); re-key (contract missing, B5); gen7 rules (exe, B6); NHL 08–11 conference names (exe trace, B7); XMA wiring (developer, B8);
every manual acceptance card (B9); portrait / head writer + P34-1 (B10); goalie gear labels (B11); PS3 archived music admission (B12);
gen7 board-ad replacement (B13, ~~binding undecoded~~ edge decoded by handoff 20 §6, writer contract absent — corrected 28 Sep, checkpoint 00 K3); Recompiled runtime (B14); older-title logo resize (B15).

## Decisions and manual observations that research cannot settle

- **User decisions:** whether league / team creation, cloning and deletion are in the beta (B4); whether gen7 rules may be patched in the
  executable (B6).
- **Optional user cards (copies only):** P34-1 (reassigned head / portrait on X360 Legacy), G-MASK-1 (goalie `maskid` vs team slot on X360
  NHL 15), one NHL 10 skater per style code (confirms the inferred NHL 10 table), NHL 08 nationality of Kopitar (code 90) / Vas (83), a
  board-ad raster identification once a read-only gallery exists.

## Changed paths and cleanup

Research-owned only: `docs/research/overnight-2026-09-26/` (00d, 15–25, BETA-HANDOFF, CHECKPOINT-2026-09-27, RECEIPT-4, STATUS, dated
notes in 14 / RECEIPT-3, data `r121` (regenerated + superseded copy), `r122*`–`r132d`), `docs/research/overnight-2026-09-25/DEVELOPER-QUEUE.md`
(C15 / C16 notes, C17–C27), `docs/research/overnight-2026-09-21-pm/11-…md` (dated note), claims `research-p15` / `p16` (notes) and
`research-p17`–`p28` (all `ready`, which is not acceptance), probes `r122*`–`r132d` and `safe_patch.py` (control-character guard). Scratchpad
emptied after every package; no WAV / PCM / extraction kept; no `.tmp-*` sibling; research bytecode caches removed; no process running.

## Next exact task

Take the coordinator's review of this receipt (or, absent one, the developer's next receipt) and correct any finding first — every
research package this run is published and ~~no static item remains open~~ (withdrawn 28 Sep, [checkpoint 00](../overnight-2026-09-28/00-correction-checkpoint.md) K2: static residuals remain, listed there with their next question) without either a runtime observation, a user decision or
disassembly-grade work (see BETA-HANDOFF); the next research step is therefore driven by that review.
