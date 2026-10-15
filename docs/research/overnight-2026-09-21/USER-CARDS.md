# Deferred user review cards — research lane, overnight 2026-09-21 (batched by title; none requested now)

Cards from earlier campaigns stay as written there (2026-09-20 closing receipt §"User review cards"; 2026-09-19 / 09-18 atlases). This
campaign adds none that require a writer. Each card below is a **research-only** review: it names a baseline, the observation and the
bounded verdict; no executable treatment is prepared unless separately authorized.

> **Correction R2 (applied 2026-09-21 17:02 MDT).** The cards R32-TEX-1, R31-GNF-1 and R40-AUD-1 below offered *diagnoses* as verdicts
> ("not loaded", "renders wrong (mips / pitch)", "audioid is consumed / is not"). A visual or listening review can only record an
> **observation**; the diagnosis is the developer's / researcher's afterwards. The three cards are **superseded** by their restatements
> in [overnight-2026-09-21-pm/USER-CARDS.md](../overnight-2026-09-21-pm/USER-CARDS.md) (observation vocabulary, positive controls, separate
> player / team treatments, named next discriminator). The text below is preserved as written; no user testing is requested now.

## Xbox 360 NHL 14

- **R25-SRC-1 — truncated loose bank stream (no treatment).** Baseline: `X:\Games\Emulation\Roms\XBOX360\NHL 14\NHL 14\audio\speech\pbp_eng_samples.sbs`
  is 650 117 120 B (620 MiB exactly) while the `nocache.big` member (entry 2) is 820 131 756 B and the bank's maximum sample offset is
  820 125 120 (handoff 02 §2). Question for the user (no launch needed): is the loose tree a partial copy the user made, or the emulator's
  extracted state? Verdict bounds: "loose copy incomplete — use the archive member" vs. "loose tree is authoritative and the game reads
  only up to 620 MiB" (the latter would need a runtime observation, not proposed). Restore: nothing to restore; read-only.

## PS3 / Xbox 360 NHL 15 (gen7 commentary — only if a listing surface is built)

- **R24-CTX-1 — context listing sanity (observation only).** Once the developer's `CTX0` listing exists, the user can compare one known
  in-game situation (e.g. a Stanley Cup presentation after a series win) with the listed contexts for `trigger_CUP_ENDINGS`
  (`Cup_End_Cup_Presentation_NHL` → `CUP_END_CUP_NHL`; AHL / WHL / QMJHL / OHL variants exist as records 2–5 with `cup_type_*` conditions).
  Bounded verdict: "the listed trigger / condition names are consistent with what was heard" or "inconsistent" — neither proves the
  selection rule (w1 / windows / run-once undecoded). No treatment, no restore.

## Xbox 360 NHL Legacy (only after the developer places a member through the EB append route)

- **R30-LZX-1 — uncompressed-LZX `chunklzx` member acceptance.** Defined in [handoff 06 §4](06-chunklzx-store-route.md): treatment A =
  the shipped `rendering/logo/texlib_27_0_0.rx2` plaintext re-encoded by `chunklzx_store.py` (content unchanged) in a copied
  `cacherender.big`; observable = the Toronto rink centre logo unchanged; control = the untouched archive; treatment B = a visibly edited
  crest through the same route; bounded verdicts as stated there; restore = original archive by hash.

## PS3 NHL 15 (after a DB edit path for the audio-id cells exists)

- **R35-AUD-1 — announcer name reassignment.** Defined in [handoff 07 §4](07-gen7-name-audio-bindings.md): one roster player's `exhibitionplayerbiotable.audioid` set to another player's value from the announcer `playerid` key set in a copied DB; observable = the arena announcer names the other player; control = unedited copy; next discriminator = the same on `playernames.audioid` for the play-by-play; restore by hash.

## Xbox 360 NHL Legacy (only after the developer builds a descriptor-regenerating resize)

- **R32-TEX-1 — resized `RW4xb2` texture acceptance.** Defined by [handoff 11 §2](11-gen7-texture-descriptor-rules.md): treatment = one
  rink / logo texture (≥ 128 × 128 DXT, single-texture container, e.g. a `rendering/logo/texlib_<artid>_0_0.rx2` crest) re-emitted at 2× its
  size with `DW7` / `DW9` / `DW11` / `DW12` / `DW13` recomputed by the rules and the chain re-tiled, in a copied archive through the EB
  append route; observable = the crest renders (sharper, not garbage / black) at centre ice; control = the untouched archive; second
  discriminator = the same on a multi-texture container (the `DW8` slot rule); bounded verdicts: "renders" / "renders wrong (mips /
  pitch)" / "not loaded". Restore = original archive by hash. Nothing to review until the developer builds the writer.

## PS4 NHL 21 (only after the developer builds the GNF resize writer)

- **R31-GNF-1 — resized `GNF` texture acceptance.** Defined by [handoff 15 §3](15-ps4-gnf-header-rules.md): treatment = one 2D BC7 / BC1
  logo (a `teamlogos/…/size256` member) re-emitted at the `size512` dimensions through the rules and placed in that directory (copied
  install); observable = the logo renders at the new size in the menu / on ice; control = untouched copy; bounded verdicts: "renders" /
  "renders wrong (mips / pitch)" / "not loaded"; restore by hash.

## PS4 NHL 21 (only after the developer ships PS4 team-name audio reassignment; listening review)

- **R40-AUD-1 — does the commentary consume `exhibitionplayerbiotable.audioid`?** Defined by [handoff 14 §4](14-ps4-player-name-audio-binding-negative-results.md):
  in a copied DB set one AHL club's `audioid` to another club's (the established team binding, as R14-AUD-1) *and* one roster player's
  `audioid` to another commonly-named player's value; observable = whether the play-by-play's player name changes in an AHL exhibition;
  bounded verdict "audioid is consumed" / "is not" — it does not establish the resolution rule. No treatment until then.
