# Handoff R130 (P26) — Presentation 5.3 scorebug: no team-keyed texture member was found in the PS4 NHL 21 `scoreclock` folders or the gen7 overlay packages (negative scope)

Research lane, 27 September 2026, published 05:44 MDT (system clock). Builds on 2026-09-21 pm handoff 16 (R59 overlay inventory, counts
only). Claim `research-p26-scorebug-team-assets`. Archive member **names** only (PS4 EB listing; gen7 EB listing + the `logo_pill` BIGF
directory); nothing extracted beyond one BIGF header, nothing decoded.

Machine-readable: [`data/r130-scoreclock-member-names.json`](data/r130-scoreclock-member-names.json),
[`data/r130b-gen7-overlay-members.json`](data/r130b-gen7-overlay-members.json).

## Plain-language summary

If the scorebug had its own picture for each team, changing a team's look would need a second set of files. On PS4 NHL 21 the 259
scorebug textures are animation frames, clock graphics and point markers — none is named after a team. On the Xbox 360 NHL 15 / Legacy
overlay packages, including the two "logo pill" packages, the inner files are numbered entries, not team numbers. So the scorebug most
likely draws team logos from the ordinary team logo files at run time; that last step is an inference, not an observation.

## 1. Measured

| Source | Members | Name patterns | Team tokens |
|---|---|---|---|
| PS4 NHL 21 `…/overlays/*/scoreclock/` (EB archives) | 259 | `goal_i#`, `goal_fr_i#` (50 each), `scoreclock_i#`, `puck_points_#`, letter variants, `scoreclock.vvm`, `scoreclockoverlay.layout`, `scoredivider` | none: no token equals a team abbreviation; the only numeric matches are `puck_points_0…5` (small numbers that coincide with low team ids) |
| X360 NHL 15 / Legacy `cacheoverlay.big` → `fe/ion/game/overlays/` | 67 packages (`overlay_<n>` 0–100 with gaps, `logo_pill`, `logo_pill_small`, `timer`, `fight_bar`, `line_change`, `snow`, `overlayfontcache`, …) | package numbers are overlay ids, not team ids (gaps, 0–100) | — |
| `logo_pill_small.big` (BIGF, 35 572 B; same on both trees) | 32 inner entries | ordinal names `0 … 49` + `sg1` / `sg2` — the portrait / logo package layout (Apt + texture members) | the numbers are member ordinals; coincidences with small team art ids are not keys |

## 2. Consequence

- No separate scorebug team-art family to register: team identity in the scorebug is **candidate: the team logo assets** (Teams 2.4
  routes), selected at run time — unobserved.
- Scorebug-owned pictures (frames, goal animations, clock plates) are ordinary GNF members on PS4 (R59) — replaceable through the existing
  GNF member route as neutral "scorebug texture <name>"; layout / text stay inside undecoded Scaleform (`.gfx`) / `.layout` / `.vvm` files.
- Not measured: PS4 15–20 scoreclock names (NHL 21 only); gen7 NHL 09–14 overlay packages; the contents of `logo_pill.big`.

## 3. Addendum 06:25 — PS4 NHL 15 → 20 ([`data/r130c-scoreclock-member-names-all-ps4.json`](data/r130c-scoreclock-member-names-all-ps4.json))

Same listing on every PS4 title: NHL 15 486 `scoreclock` members (`scoreclockframe_i#…` variants), NHL 16 24, NHL 17 26, NHL 18 44, NHL 19 55,
genuine NHL 20 260, NHL 21 259. No member is named after a team: the numeric "hits" are `puck_points_0…5` (18+) and NHL 15's single token
hit is the frame suffix `iec` (`scoreclockframe_iec`), a coincidence with an abbreviation. NHL 16 / 17 carry generic logo plates
(`logofader`, `logocover_gradientcover`) — consistent with team logos drawn from the team logo assets (candidate, unobserved).
