# Receipt 35 — skater style labels on the roster cell and Lines rail are now per title (defect R122 S1 / S2)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged; nothing staged or committed). Claim `dev-style-label-scoping`. Direction:
[overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md) §4 / §6 (a focused
defect first). Research consumed: R122 = [2026-09-26 handoff 15](../../research/overnight-2026-09-26/15-native-label-consumers.md)
§2–§4 (observations S1, S2), R121. **Not in either review bundle.**

## 1. Plain-language summary

- **The defect:** the roster style cell (Lines page, roster table) and the Lines rail's style dropdown used
  NHL Legacy's ten style names on every title. The games number styles differently in three eras (7 styles
  on NHL 07–09, 9 on NHL 10, 10 from NHL 11), so on NHL 08 / 09 the same player read "Grinder" on the player
  page and "Enforcer Defenseman" on the Lines page, and on NHL 10 about a thousand forwards read
  "Two-Way Defenseman".
- **Now:** NHL 08 / 09 roster cells use the same NHL 07 names as the player page (research measured 98.7 %
  of skaters storing the same code in both places). NHL 07's roster cell and NHL 10 show the stored number,
  with a note saying why. NHL 11 onward keep the Legacy names. The Lines rail takes its choices from the same
  per-title vocabulary, or offers the field's own range as numbers — it no longer offers codes a 3-bit field
  cannot hold. Goalie rows on the rail show numbers (their roster code's meaning is unresolved, R122 S4).
- Not changed: the companion style on NHL 11–13 still shows a number (R122 S3 — a coordinator decision
  whether to carry the Legacy names back by continuity); the NHL 10 candidate table in R122 is inferred and
  was not adopted.

## 2. Files

| Path | Change |
|---|---|
| `defs/enums/player-style.toml` | `[scope] games` = NHL 11 → 21 and Legacy (Recompiled by identity), with the R122 provenance |
| `crates/studio-domain/src/attributes.rs` | `gen6_roster_style_options` (`eGlu.QgFs` on the NHL 08 / 09 schema families → the NHL 07 seven-style set) + note |
| `crates/studio-domain/src/view.rs` | `read_field` applies it when no scoped vocabulary resolves |
| `app/src/components/LinesTab.tsx`, `app/src/lib/teamPresentation.ts` | rail options from the cell (`archetypeChoices`); the Legacy-only constants removed |
| tests | `studio-domain/tests/style_label_scoping.rs` (**new**, 3 cases); `app/src/components/LinesTab.archetype.test.tsx` (**new**, 4); `studio-domain/tests/view.rs` and `tdb-enums/tests/enums.rs` now scope their registry to NHL Legacy (the anchor they read) and assert NHL 10 has no Legacy label |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- **Red first** (`r35-style-scoping-red.log`: the new test against the old unscoped table): NHL 08 roster
  code 1 read "Defensive Defenseman" (expected the player page's "Defensive"); NHL 07 roster code 3 read
  "Enforcer Defenseman" with 10 options.
- **Green** (`r35-style-scoping.log`): `style_label_scoping` 3 / 3 — X360 and PS3 NHL 08 / 09: codes 1 / 3 / 5 / 7
  read Defensive / Grinder / Sniper / Dangler on **both** the roster cell and the player page's `p_style`, the
  roster cell offers exactly codes 1–7 and its note cites R122; NHL 07 and X360 / PS3 NHL 10 (more than 500
  code-4 forwards) show numbers with "no catalogue for NHL 10 … shown as a number"; X360 NHL 11 / 13, PS3
  Legacy and PS4 NHL 21 keep "Two-Way Forward" for code 9 (10 options), PS4 21's companion style too.
- Affected suites: studio-domain `view` 16 (after scoping its registry), `nationality_labels` 4, `nhl07_encodings`
  8, `surfaces` 4, `surface_matrix` 1, `import_change_set` 3; `tdb-enums` 7; `tdb-domain-validation` 16;
  studio-api — see §6; vitest `LinesTab.archetype` 4 + the full run at closeout; `tsc` exit 0.

## 4. Limits

- The NHL 08 / 09 roster vocabulary is **inferred** for that cell (agreement with the player-page field, not
  an in-game observation); the NHL 07 vocabulary itself is the in-game-verified one.
- Import / corpus style writes are unchanged (they write the companion only where `QgFs` exists, NHL 14+).
- Wording ("TOUGH GUY", "2 WAY DEFENDER" in the games' own text) is not adopted.

## 5. Card

- **V-77** (desktop, a build that includes receipt 35): X360 or PS3 NHL 08 → a team → Lines → a skater on the
  rail → style dropdown: seven NHL 07 names, the same as that player's page; NHL 10 → numbers 0–15 with the
  hover note; NHL Legacy → the ten Legacy names as before.

## 6. Closeout checks

studio-api lib 31, `player_identity` 2, `player_ratings` 1 (all green, in `r35-style-scoping.log`); vitest 260 tests
(incl. `LinesTab.archetype` 4); `tsc` exit 0. No DTO change (bindings untouched).

## 7. Cleanup

The scratch copy of the fixed TOML used for the red run was restored and deleted; no temporary roots.
