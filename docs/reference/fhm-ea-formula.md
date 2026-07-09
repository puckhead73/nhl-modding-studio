# FHM → EA rating formula — ported reference (for Stage 2 `tdb-formula`)

Captured from the prior `nhl-database-studio` app under explicit per-instance clean-room grant
(2026-07-05). This is the **seed material** for the Stage-2 formula engine: everything below is
schema-independent math + the default preset, so it can be reimplemented in our person-centric corpus
with no further reads of the prior app. The prior source (reference only) was
`crates/tdb-gui-api/src/{ratings.rs,archetype.rs}` + `assets/default_ea_formulas.json`.

The verbatim default preset is committed alongside this file as
[`default_ea_formulas.json`](default_ea_formulas.json).

---

## 1. Scale + normalization

FHM attributes are a **0..20 nominal** scale. EA standard ratings display **36..99** (stored raw as
`display − 36`, 6-bit; see §5). The map is linear:

```
ea_value = clamp(lo + weighted_avg × (hi − lo), lo, hi)     // lo=36, hi=99 for standard rows
```

`weighted_avg` is a weighted average of each component's **normalized** value (0..1), weights summed
as `Σ|weight|`. Two normalization **modes** (a profile-level toggle):

- **Nominal** (default, "Puckvault parity"): denominator is the fixed FHM cap `20`. FHM>20
  (generational) normalizes >1.0 and the final clamp pins it at 99.
- **PositionMax**: denominator is the **corpus-observed max FHM value for the (position, attribute)
  pair**, falling back to `20` when the corpus has nothing. A forward whose Puck Handling is the
  all-time-high forward value maps to EA 99. Buckets are per position (forward / defenseman / goalie).

## 2. Formula profile shape

Per **position** (`forward`, `defenseman`, `goalie`), a map: **EA attribute → list of components**,
each component = `(fhm_key, weight)`. `fhm_key` is either a raw FHM column name (e.g. `"Passing"`) or
a prebuilt reference `"prebuilt:<key>"` (§4). Plus a top-level `normalisation_mode`. This is exactly
the shape of [`default_ea_formulas.json`](default_ea_formulas.json).

**EA attribute catalog** (component order = puckvault display order):
- Skater (28): Speed, Body Checking, Endurance, Puck Control, Passing, Shoot/Pass Bias,
  Defense/Offense Bias, Pass/Carry Bias, Slapshot Power, Slapshot Accuracy, Wristshot Power,
  Wristshot Accuracy, Agility, Strength, Acceleration, Balance, Faceoffs, Durability, Deking,
  Aggressiveness, Poise, Hand-Eye, Shot Blocking, Offensive Awareness, Defensive Awareness,
  Discipline, Fighting Skill, Stickchecking.
- Goalie (19): Glove Low, Glove High, Stick Low, Stick High, Speed, Agility, Consistency,
  Pokechecking, Endurance, Breakaways, Rebounds, Shot Recovery, Poise, Passing, Angles,
  Puck Playing Frequency, Aggressiveness, Durability, Vision.

**FHM attribute vocabulary** (the `attrs_json` keys in our corpus, from `player_ratings.csv`):
Character/Mental (Aggression, Bravery, Determination, Teamplayer, Leadership, Temperament,
Professionalism, Mental Toughness); Physical (Acceleration, Agility, Balance, Speed, Stamina,
Strength, Fighting); Offensive (Screening, Getting Open, Passing, Puck Handling, Shooting Accuracy,
Shooting Range, Offensive Read); Defensive (Checking, Faceoffs, Hitting, Positioning, Shot Blocking,
Stickchecking, Defensive Read); Goalie (Goalie Stamina, G Positioning, G Passing, G Pokecheck,
Blocker, Glove, Rebound, Recovery, G Puckhandling, Low Shots, G Skating, Reflexes); plus FHM
composites (Skating, Shooting, Playmaking, Defending, Physicality, Conditioning, Character,
Hockey Sense, Goalie Technique, Goalie Overall Positioning) and position grades G/LD/RD/LW/C/RW.

## 3. Compute algorithm (per EA attribute)

```
components = profile[position][ea_attr]      // [] → result is None (row shows "—")
lo,hi = scale (36,99 standard; overrides in §5)

if scale is NON-standard (bias/tier rows, lo≠36 or hi≠99):
    # direct-output: return the first prebuilt component verbatim on its native scale
    for c in components:
        if prebuilt(c) and eval(c) is Some(v): return round(v).clamp(lo,hi)
    return None

# standard 36..99 rows: weighted average of normalized components
total_v = total_w = 0
for c in components:
    if prebuilt(c):
        v = eval_prebuilt(c); if None: continue
        normalized = v / native_max            # native_max per §4
    else:
        v = fhm_values[c.fhm_key]; if absent: continue   # missing → drops out of BOTH sums
        denom = (mode==Nominal ? 20 : position_max[(pos,key)] or 20)
        normalized = v / denom
    total_v += normalized × c.weight
    total_w += |c.weight|
if total_w == 0: return None
return round(clamp(lo + (total_v/total_w) × (hi−lo), lo, hi))
```

Key rule: a **missing FHM column contributes nothing** (dropped from numerator *and* denominator),
never a zero.

## 4. Prebuilt formulas

`(key, label, native_max, direct_output)` — `direct_output` rows override scale to their native range;
others normalize against `native_max` and join the weighted pool.

| key | native_max | direct | math |
|---|---|---|---|
| `pass_shoot_bias` (FWD) | 15 | ✓ | `clamp(round(8 − career_G/5 + career_A/10), 0, 15)` |
| `pass_shoot_bias_def` (DEF) | 15 | ✓ | `clamp(round(8 − career_G/3 + career_A/10), 0, 15)` |
| `def_off_bias` | 15 | ✓ | `career_GP==0 → 8`; else `ppg=(G+A)/GP; clamp(round(8 + (ppg−0.5)×10), 0,15)` |
| `pass_carry_bias` | 15 | ✓ | needs Puck Handling, Speed, Passing, Hockey Sense; `carry=(PH+SP)/2, pass=(PA+HS)/2; clamp(round(8 + (carry−pass)/20×14), 0,15)`; any missing → None |
| `vezina_consistency` | 2 | ✓ | Vezina wins → tier: `0→0, 1–2→1, ≥3→2` (hard-coded winners table, name-key `"<last> <first>"`) |
| `gp_durability` | 20 | ✗ | `recent_team_schedule==0 → None`; else `clamp(recent_GP/recent_team_schedule × 20, 0, 20)` |
| `shooting_pct` | 30 | ✗ | `est = recent_GP × spg(pos)`; `est≤0 or recent_goals==0 → None`; else `min(recent_goals/est × 100, 30)` |

`spg(pos)` NHL shots-on-goal/game: LW 1.75, C 1.62, RW 1.79, D 1.28, else 1.72.

**StatsContext** the prebuilts need (derive from our `career_stat` rows, NHL regular-season only):
`career_G_pro`, `career_A_pro`, `career_GP_pro` (career-wide sums), `recent_GP`, `recent_goals`
(most-recent NHL RS year), `recent_team_schedule` (max GP of any player on that team that year),
`position` label, and `name_key` (normalized `"<last> <first>"` for the Vezina lookup). The full
Vezina winners list + `name_key` normalization are in the prior `ratings.rs` (port verbatim — pure
data). Prebuilts that lack data fall back to the neutral value (biases→8) or None (durability/pct).

## 5. Offset encoding (write-back — used by Stage 3, but the engine tags it)

- **Standard ratings** (Speed … everything showing 36..99): on-disk raw = `display − 36` (6-bit).
- **Bias / tier rows** — Shoot/Pass Bias, Defense/Offense Bias, Pass/Carry Bias → scale `(0,15)`;
  goalie Consistency → scale `(0,2)`: stored **raw, no offset**.

So `ea_attr_raw_bits_offset(key) = 0` for those four override rows, `36` otherwise. This matches our
`attributes.rs` `rating_offset` (PS3/Xbox +36; PS4 stores 0..99 directly — apply the PS4 branch there).

## 6. Archetype classifier (→ EA `QgFs` style)

Position-split weighted-average scorer; **goalies get no style** (None). Score each archetype =
`Σ(fhm_value × weight) / Σ(weight)` over its columns (missing columns drop out); **winner = max**, ties
break to declaration order. Forwards have an **enforcer pre-check**: if `Fighting ≥ 17.0` AND
`Aggression ≥ 17.0`, short-circuit to Enforcer (raw 10) before scoring.

**Forward archetypes** (label, EA raw, weights):
- Sniper (7): Shooting Accuracy 3.0, Getting Open 2.5, Offensive Read 1.5, Speed 1.0, Acceleration 1.0
- Playmaker (6): Passing 3.0, Puck Handling 2.5, Offensive Read 2.0, Teamplayer 1.0, Getting Open 1.0
- Power Forward (8): Strength 2.5, Screening 2.0, Shooting Accuracy 1.5, Hitting 1.5, Stamina 1.0
- Two-Way Forward (9): Defensive Read 2.5, Offensive Read 2.0, Checking 1.5, Stickchecking 1.5, Faceoffs 1.0
- Grinder (5): Checking 3.0, Hitting 2.5, Stickchecking 2.0, Defensive Read 1.5, Stamina 1.0
- Enforcer (10): threshold-gated (Fighting≥17 ∧ Aggression≥17), not scored

**Defenseman archetypes**:
- Offensive Defenseman (2): Speed 2.0, Acceleration 2.0, Passing 2.0, Offensive Read 2.0, Shooting Accuracy 1.5, Puck Handling 1.0
- Two-Way Defenseman (4): Defensive Read 2.5, Offensive Read 2.5, Positioning 2.0, Passing 1.5, Stickchecking 1.5
- Defensive Defenseman (1): Defensive Read 2.5, Positioning 2.5, Stickchecking 2.0, Checking 2.0, Hitting 1.5, Shot Blocking 1.5

EA raw codes match `defs/enums/player-style.toml` (D 1–4, F 5–10, G 0–2). Position group from FHM
grades: goalie if G tops all; else defenseman if max(LD,RD) > max(LW,C,RW); else forward.

## 7. Overall

Computed from the produced EA ratings, not stored. Our `attributes.rs` already carries the skater
regression (`SKATER_OVERALL_TERMS`, D-intercept + forward-scale); goalie overall is the stored field
in the EA DB but the prior engine derives it from ratings via `ea_goalie_overall_from_ratings`. Reuse
our existing regression for the Stage-2 preview overall.

## 8. Adaptation notes (prior app → our corpus)

- **Portable verbatim** (pure math/data): EA attr catalog, FHM vocabulary, the compute algorithm
  (§3), all prebuilt math + Vezina table + `name_key`, the offset rule (§5), the archetype tables
  (§6), and the default preset JSON.
- **Adapt** (was coupled to the prior app's SQL schema `season_player_raw_ratings`/`rating_columns`/
  slug): our FHM values come straight from `ratings_season.attrs_json` (already a name→value map) +
  the G/LD/RD/LW/C/RW grade columns; **StatsContext** is derived from our `career_stat` rows;
  **PositionMax** maxima come from a new `tdb-fhm-store` query over `ratings_season` grouped by the
  computed position + attribute (mirror the prior `compute_maxes`, cached).
