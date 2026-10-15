# Deferred user review cards — research lane, overnight 2026-09-21 pm (batched by title; none requested now)

Vocabulary rule (coordinator review R2): a card records **observations** — *changed appearance*, *unchanged appearance*, *rendering
failure* (garbage / black / wrong colours), *boot or load failure*, *name heard* / *other name heard* / *no name heard* — never a diagnosis
("not loaded", "mips wrong", "audioid is consumed"). Each card names its baseline, a **positive control** (a treatment already known to
produce a visible / audible change through the same route), the treatment, the observable, and the next discriminator the observation
selects. Restore is by hash of the untouched original. Cards from earlier campaigns stay as written there except the three superseded
below. No executable treatment is prepared unless separately authorized; nothing here is requested now.

## Xbox 360 NHL Legacy — only after the developer's descriptor-regenerating resize exists (supersedes R32-TEX-1 of 2026-09-21)

- **R32-TEX-1 rev. 2 — resized `RW4xb2` texture: observation card.** Baseline: the untouched `cacherender.big` (hash recorded) and the
  centre-ice crest of one club as it renders. Positive control: the same crest replaced **at its shipped size** with a visibly different
  image through the same EB append route (a route the developer's P2-A / P3 work already exercises) — if the control does not show a changed
  appearance, the route is the problem and the resize observation is void. Treatment: the same crest re-emitted at 2 × its size (single-
  texture container, ≥ 128 × 128 BCn) with `DW7` / `DW9` / `DW11` / `DW12` / `DW13` recomputed and `DW8` preserved, chain re-tiled, through the
  same route. Observables, recorded separately: (a) menu / arena loads or not; (b) crest appearance: unchanged / changed / rendering failure;
  (c) if changed, sharper-than-baseline or not (a screenshot at the same camera). What each observation selects next: *unchanged* → the
  member the game reads is not the one written (source-identity question, not a descriptor question); *rendering failure* → a descriptor
  or chain-layout discrepancy (developer: compare the written descriptor with the census shapes of that size); *changed and sharper* →
  proceed to the multi-texture-container discriminator (a resize inside a container whose `DW8` values are not `2 × ordinal`,
  handoff 02). No verdict about `DW8`, mips or pitch is drawn from appearance alone.

## PS4 NHL 21 — only after the developer's GNF resize writer exists (supersedes R31-GNF-1 of 2026-09-21)

- **R31-GNF-1 rev. 2 — resized `GNF` texture: observation card.** Baseline: untouched copied install, one `teamlogos/…/size256` member and
  where it appears (menu team select). Positive control: the same member replaced at its shipped size with a visibly different image
  (the accepted NHL 21 E1 route); no changed appearance in the control voids the card. Treatment: the member re-emitted at the `size512`
  dimensions by the rules of handoff 15 and placed in the `size512` directory (the registry's size-directory rule is the outer constraint).
  Observables: load / no load; appearance unchanged / changed / rendering failure at each surface where the logo is drawn. Selection:
  *unchanged* → placement / directory question; *rendering failure* → header or chain discrepancy (developer: `check_resize_rule` on the
  written file and a census-shape comparison); *changed* → the size class is accepted at that surface only (other surfaces are separate
  observations). No "mips / pitch" diagnosis from appearance.

## PS4 NHL 21 — only after the developer ships PS4 team-name audio reassignment; listening review (supersedes R40-AUD-1 of 2026-09-21)

- **R40-AUD-1 rev. 2 — player-row `audioid`: observation card with separate team and player treatments.** Baseline: untouched copied DB;
  an AHL exhibition where the play-by-play names a chosen player at least once (a goal by that player is the name-call opportunity —
  record whether an opportunity actually occurred; a game with no opportunity is *no observation*, not "unchanged"). **Treatment T (team,
  positive source / placement control):** one AHL club's `exhibitionteams.audioid` set to another club's keyword id (the established
  binding, 2026-09-20 handoff 07 §3). Observable T: the club is called by the other club's name / by its own name / not called. A successful
  T shows only that the edited DB copy is the one the game reads and that *team* keywords are consumed; it says nothing about the player
  consumer. **Treatment P (player):** in the same copy, one roster player's `exhibitionplayerbiotable.audioid` set to the value of another
  commonly-named player. Observable P (only when a name-call opportunity occurred): the player's own surname heard / the other surname
  heard / no surname heard (a generic call). Recording rule: T and P observed and written down separately; P without an opportunity is
  discarded. Selection: *other surname heard* → the column is read by the commentary path; next discriminator = the same on
  `playernames.audioid` via `surnameid` to separate the two columns; *own surname heard with T successful* → this direct treatment
  produced no change in this situation (it does **not** show the column is unused: a namespaced / conditional / code-resident use remains
  possible); next discriminator = the `0TVE` parameter typing (handoff 14 §4 item 2) or a treatment on a player with `audioid` ≤ 5 363
  vs. one above; *no surname heard* → no observation. Nothing here establishes the resolution rule.

## Carried forward unchanged (2026-09-21 campaign)

R25-SRC-1 (Xbox 360 NHL 14 truncated loose stream — a question, not a test), R24-CTX-1 (context listing consistency), R30-LZX-1
(uncompressed-LZX member acceptance; its verdict vocabulary already names the observable — the centre logo unchanged / changed / failure),
R35-AUD-1 (PS3 NHL 15 announcer name reassignment; observable = which name the arena announcer says; a control = the unedited copy).

## PS4 NHL 18–21 — observation only (no treatment): L-CHL-1

- **L-CHL-1 — CHL domestic participants.** Defined in [handoff 13 §2](13-league-blockers-status-and-deferred-cards.md): start the CHL tournament
  twice from the same state and record the 32 participants; identical → fixed table in code; different → runtime selection. No restore.

## Xbox 360 NHL 09 — observation only: L-SET-09

- **L-SET-09 — settings labels.** Defined in [handoff 13 §2](13-league-blockers-status-and-deferred-cards.md): do the settings-screen labels
  match the dictionary strings under `<Setting>.<n>` keys (developer lists the keys first; absent keys void the card).
