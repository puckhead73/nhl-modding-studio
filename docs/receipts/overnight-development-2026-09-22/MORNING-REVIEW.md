# Morning review — developer lane, overnight 2026-09-22

Everything below is developer-verified only (cargo / jsdom evidence). Nothing was launched, nothing
was written to a live install, and **no card here claims the game accepted anything**. The cards of
the 19, 20 and 21 September campaigns remain valid and are not repeated. NHL 21 E1 stays closed.
Grouped by title; each card names its receipt.

## Runtime review (a console / emulator run — the only way these close)

| Card | Title | What to do | Expected | Failure means | Restore | Receipt |
|---|---|---|---|---|---|---|
| **R31-GNF-1 rev. 2** | NHL 21 PS4 (copied extracted tree) | Replace a real 1024² BC3 custom logo (`rendering/customlogos/customlogo_46_diff.gnf`) with a 512² image through Replace…, then open a screen that draws that logo | the logo draws at the lower resolution — the recomputed header (`pitch` / `last_level` / `d7` / `streamSize`) is accepted by the PS4 loader | blank / corrupt → the loader wants something the size rule does not carry, even though the rule reproduces EA's own header byte-for-byte at the shipped shape; refuse changed-dimension GNF writes on that family | the `.bak` Studio wrote (byte-exact restore is automated-tested) | 02 |
| **R31-GNF-2** (new) | NHL 16 PS4 (copied extracted tree) | The same on `rendering/textures/trophy_0_0_coeff.gnf` (1024² BC3, 11 levels) and `hat_0_0_nm.gnf` (512² BC5) | as above, on a second title and on a **two-channel normal map** | a BC5-only failure would separate "the header rule" from "the channel layout" | `.bak` | 02 |
| **R32-XB2-1 rev. 3** (23 Sep) | NHL Legacy Xbox 360 (copied install) | the changed-size step now applies to **single-level** rasters only — a mipped raster refuses a changed size before any write (receipt 23 Sep 01). The same-size partial replacement's appearance is a separate observation, not predicted | — | — | — | 11 (21 Sep), 23 Sep 01 |
| **R07-TEX-1** (new) | NHL 07 Xbox 360 (copied install) | *No action yet.* Studio now **refuses** a changed-dimension replacement in NHL 07's loose `rendering/*.rx2` containers (its table-at-EOF layout has no implemented reflow), while the *descriptor* rule is proven on that generation | nothing to observe until the NHL 07 container reflow is implemented | — | — | 03 |

## Desktop review (no game)

| Card | Title | What to do | Expected | Restore | Receipt |
|---|---|---|---|---|---|
| V-23 | NHL 21 or NHL 16 PS4 (extracted tree open) | File browser → a `.gnf` under `rendering/textures` → Replace… with an image of a **different** size; then look at the preview again | the dialog accepts it, names the GNF rule and its runtime card; after writing, the preview re-opens at the **new** size showing the new image | the `.bak` beside the file | 02 |
| V-24 | NHL 15 or NHL 19 PS4 (extracted tree open) | Replace… on one of the **two-stream** `.gnf` files (NHL 15 ships 1 115, NHL 19 5 342) | refused with "multi-stream file: N `GNF ` streams tile the … file … resizing one stream would drop the others" — not a size-mismatch message | none (refused) | 02 |
| V-25 | Any Xbox 360 title (install root open) | Replace… with a deliberately absurd size (e.g. a 9 000-pixel-wide image) on a loose `rendering/*.rx2` | refused with a message naming the requested value **and** the limit (`… 9000 cannot be stored in the descriptor (1–8192)`); the file and its `.bak` are untouched | none (refused) | 01 |
| V-26 | NHL 07 Xbox 360 (install root open) | Replace… with a different size on a loose `rendering/*.rx2` | refused with "this container keeps its buffers in the header region with absolute offsets (the NHL 07 table-at-EOF layout)…" — a same-size replacement still works | none (refused) | 03 |

## Desktop review added later in the campaign (no game)

| Card | Title | What to do | Expected | Restore | Receipt |
|---|---|---|---|---|---|
| V-27 | NHL 15–21 PS4 (extracted tree open) | Leagues → Commentary → the **Contexts** card | it now lists PS4 contexts (it used to say "not covered"): trigger → conditions → the lines it speaks. The note names the file version and says the condition argument *values* and the bytecode are not decoded | none (read-only) | 04 |
| V-28 | Any title (project open) | Team → an arena → the new **Venue parameters** card | fields grouped by kind with a plain-language gloss; the PS4 colour zones as swatches; "show N constant fields" folds away everything that never varies; unclassified fields marked read-only with the reason on hover | none (read-only) | 05 |
| V-29 | NHL 17–21 PS4 (extracted tree open) | The same card → the **goal horn** row | it names the club or generic horn (e.g. `$NHLTeamName_ANA (AnaheimDucks)`) instead of a bare number; hovering shows the rule. A New York Rangers arena and any value 690–693 say "not in the catalogue" with the reason — that is correct, not a bug | none (read-only) | 07 |

## Desktop review added by the R3 continuation (22 Sep 2026, no game)

| Card | Title | What to do | Expected | Restore | Receipt |
|---|---|---|---|---|---|
| V-30 | Any Xbox 360 / PS3 NHL 11–Legacy install (root-folder project open) | Team → Uniforms → pick a kit → the new **Kit textures (measured join)** card under the jersey metadata | the contract line names the title's sock grammar (two sock files on NHL 11 / 12, one on 13 → Legacy); the rendering tree line shows each folder's file count; the selected row lists jersey / pants / sock(s) with thumbnails and a **complete** verdict; the spare-candidates button shows complete *and* incomplete counts and lists them on click with what is missing; "Re-pointing a row is not offered" carries its reason. Switching teams quickly never shows the previous team's card. On NHL 09 / 10 the card says "no selector table"; on PS4 "not measured" with rows listed and no components | none (read-only) | 10 |
| V-30b | The same, with the install's `rendering/sock` folder temporarily **renamed** on a *copied* tree | every row's socks read **missing — the `sock/` directory is absent**, the verdict is **incomplete**, and no spare candidate is complete — the coordinator's F1 scenario, on screen | rename the folder back | 10 |
| V-32 | Any Xbox 360 / PS3 NHL 09–Legacy install (root-folder project open); then NHL 21 PS4 (extracted tree) | Team → an arena → the new **Arena assets** card under Venue parameters | gen7: the venue model (and jumbotron on 12+) read *present* with the arena art id named as the key; the centre-ice logo and banners say "home team … (art id N)" as their key (on NHL 09–11 they say the title has no home-arena link); the bounce map is badged **candidate key**, the other ice layers **no established binding**; the spare-venue button lists ids no arena row names. PS4: badge "vault reference", the venue folders listed with member counts, the shared ice families badged candidate, no family badged established | none (read-only) | 12 |
| V-33 | Any Xbox 360 / PS3 NHL 09–Legacy install (root-folder project open); then NHL 21 PS4 | Assets → Players: the index pane (no player picked) shows **Portrait & head containment**; pick a player → the **Containment** card | the summary counts own heads / pool heads with their file presence, the `hasportrait` × file cross-tab as four numbers, the unused shipped portraits / heads, and names any "shipped without asset" or flagged-without-file players (NHL 09: Nedved, Andersson …; Legacy: Nagy, Colella). A player's card shows art id, head id, portrait / head badges and `hasportrait = N (as stored)`; a disagreement carries a note saying it is shown, not corrected. On PS4 the head badge reads *unknown* and the note says heads are vault data. Nothing on either card can be edited | none (read-only) | 14 |
| V-34 | NHL 13 / 15 / Legacy Xbox 360 or PS3 (root-folder project open); then NHL 11; then NHL 09 | Player → Equipment → "Art files on this install" | each slot row now carries a set pill: "in-game set complete" / "front-end set missing feglove_N_0_dm" (gloves 15, 18, 20, 23, 24, 28, 33, 39 lack the front-end diffuse; pads / blockers / trappers 12, 14, 15, 17 … lack the third tint map) — an incomplete set is information, the id still has files. On NHL 11 the pill checks for the one `texlib_<id>` container; on NHL 09 the row says "set not measured on this title" | none (read-only) | 15 |
| V-35 | NHL 21 (or 17–20) PS4, extracted tree open; then NHL 15 | Team → any team → **Selectable songs (catalogue, read-only)** under Name calls | the card lists 422 songs (NHL 21) with title keys, composers, genre names and goal / power play / win eligibility; typing a composer filters; the subtitle and notes say it is what the catalogue offers, not the team's song, and that the team's song cells have no catalogue join. On NHL 15 the card is one line: no catalogue on this title | none (read-only) | 16 |
| V-31 (**revised 23 Sep**) | Any Xbox 360 title (root-folder project open) | Replace… a jersey texture (Uniforms) with a same-size image; then try a **different**-size image on the same (mipped) raster | before Write, the dialog shows the partial-replacement note (base + large levels rewritten, packed-tail levels keep the shipped bytes, "not a full-image replacement"); the result repeats it word for word. The different-size attempt is **refused before Write** with the reason (mipped raster; nothing written) and the Write button is disabled | the `.bak` beside the file | 11, 23 Sep 01 |

## Notes for the user

- The GNF cards are the **first** real-file evidence for PS4 resizing: six shipped NHL 21 / NHL 16
  textures now round-trip in automated tests on throwaway copies, and halving one then growing it
  back reproduces EA's shipped header byte-for-byte. That makes the codec side strong; it still says
  nothing about what the console loads, which is what R31-GNF-1/2 are for.
- Nothing in this campaign changed which sizes are *accepted* — only which requests are refused
  (and why) and how much of that is proven on real data.
- The later packages (Contexts on PS4, the venue inventory, the goal-horn name, the kit join) are
  **read-only inspection**. None writes anything, so none needs a console to be trusted — the desktop
  cards above are the whole review for them.
- **R3 (22 Sep, continuation):** the kit join's completeness rule was corrected (an absent sock folder
  now yields *incomplete*, never a complete spare) and the join is on the Uniforms tab (V-30). The
  Xbox 360 mip audit changed what a same-size Replace… writes: the small distant-LOD levels are no
  longer overwritten under a layout the console does not use — they keep the shipped bytes, which
  means the old image at distance until the packed-tail contract exists (V-31; runtime card
  R32-XB2-1 is unchanged for changed sizes). The later R3 cards (V-32 arena assets, V-33 player
  containment, V-34 equipment sets, V-35 song catalogue) are all **read-only inspection** — nothing
  writes, so the desktop cards are the whole review for them. Receipt 17 is the closeout.
