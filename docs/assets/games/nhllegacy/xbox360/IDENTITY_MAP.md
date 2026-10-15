# NHL Legacy — Xbox 360 — asset identity map (Phase 5.1)

> **Purpose.** Tie every on-disk graphical-asset id to the **database entity** it belongs to and the
> **resolved human-readable name** (team / player), so the studio surfaces real names — never raw EA
> ids — per the human-readable-surfaces rule (`CLAUDE.md`). Raw ids stay available only behind
> progressive disclosure.
>
> **Source of truth:** the committed anchor `anchors/xbox360/nhllegacy/databases/nhlng.db` (TDB v6, BE)
> + its metadata; the on-disk asset tree is the user-provided read-only mount (see
> [`ASSET_PROVENANCE.md`](ASSET_PROVENANCE.md)). Field four-char codes below are the studio
> (canonical) ids from `docs/audits/data/schema/xbox360-nhllegacy.json`; the `name` column is the
> metadata name EA ships.

## 1. The identity links

All three families key off an **art id** stored on a DB table. The resolution is done in the domain
layer (`crates/studio-domain/src/assets.rs`), which has the name data.

| Family | On-disk path | Asset id ← DB field | Entity | Resolved name ← DB field |
|---|---|---|---|---|
| **Portrait** | `fe/ion/artassets/playerheads[small]/p{bucket}/p{id}.big` | `ubPc.lOnr` (`artid`) | player (`ubPc` record) | `ubPc.HdeP`+`QbMR` (`firstname`+`lastname`) |
| **Head mesh + texlib** | `rendering/player/player_{id}.rx2` + `rendering/player/texlib_{id}.rx2` | `ubPc.pPaD` (`headid`) | player (`ubPc` record) | same |
| **Team logo** | `rendering/logo/texlib_{id}_{type}_{variant}.rx2` | `kOtt.lOnr` (`artid`) | team (`kOtt` record) | `kOtt.YmkJ` (`fullname`), `QNTI` (`shortname`), `xsnn` (`abbrname`), `rbPR` (`artabbr`) |

### Key rules (verified)

- **`ubPc.lOnr` (portrait art id) and `kOtt.lOnr` (team-logo art id) are the same four-char field
  (`artid`) on two different tables** — never conflate them. A portrait id resolves to a *player*; a
  logo id resolves to a *team*.
- **Bespoke head test:** a player has a modelled head of their own **iff `ubPc.pPaD == ubPc.lOnr`**;
  otherwise `pPaD` points at a shared **pool head** (`>= 60000`, `POOL_HEAD_MIN`) that belongs to no
  single player and must not be labelled as one. (Verified in `assets.rs`: exact on the reference
  anchor, not a heuristic.)
- **Portrait presence** is flagged by `ubPc.SvcL` (`hasportrait`) and is decoupled from heads — far
  more players have a portrait than a bespoke head.
- **Logo id components:** the logo texlib stem is `texlib_{artid}_{type}_{variant}` — `{artid}` is the
  team's `kOtt.lOnr`; `{type}`/`{variant}` index the logo/wordmark/crest set (e.g. `texlib_10_0_0` is
  the primary and `texlib_10_1_0..4` are variants for the same team).

## 2. Verification

**Logo id = `kOtt.lOnr` (artid), proven against the anchor + the on-disk tree (2026-08-03).** Test
`assets::tests::legacy_x360_team_logo_identity_map` (studio-domain, reads the committed anchor — runs
in CI):

- The team resolving to **"Detroit Red Wings®" carries `lOnr = 10`** — matching the eyeballed
  `texlib_10` → *Red Wings* wordmark from the preview confirmation (§6 of `ASSET_PROVENANCE.md`).
- The **artid space is sparse and reaches `max = 20054`** across only **252 team records** — a record
  index or the team key (`vfEq`) could never exceed 251, so the logo token is neither. On-disk logo
  files occupy that exact high range (`texlib_20066 … texlib_20085` are present), confirming the id
  space is the art-id space.
- **`lOnr != vfEq` on 219 / 252 teams** — so the logo id is specifically the artid, not the team key.
- The divergence is visible in the AHL block: e.g. team **record 32 = "Albany Devils"** but
  `artid = 141 → texlib_141`. Record index ≠ artid; the logo uses artid.

There are **348 distinct on-disk logo ids vs 252 teams** because the same art-id space also holds
non-team marks (all-star / historical / league logos) that no `kOtt` record references.

## 3. Data

The resolved team-logo map is generated deterministically from the anchor into
[`identities.csv`](../../../../audits/data/assets/nhllegacy/xbox360/identities.csv) (drift-guarded;
`QA_WRITE=1` to regen) — **all 252 `kOtt` records**, each row: `team_record, logo_artid,
logo_texlib_stem, fullname, shortname, abbrname, artabbr`. Names are the inline TDB strings the game
ships (e.g. `Detroit Red Wings®`), i.e. the human-readable surface.

**Player portrait/head identity** (`ubPc.lOnr`/`pPaD` ↔ player name) is **not dumped** here — it is
already resolved live for all 5632 players by `scan_player_assets` (`assets.rs:730`) and surfaced
through `PlayerAssetsDto` / the app's player-asset pages, so it needs no static CSV.

## 4. Uniforms + equipment (Phase 5.2 / 5.3)

- **Uniforms** (`rendering/{jersey,pant,sock}`) — **team-keyed**. The numeric team-id token's position
  in the filename is title-specific, so it is **derived at runtime** by `derive_team_index`
  (`assets.rs:2136`) and resolved to the team name by `export_team_outfits` (`assets.rs:2321`) — not
  hard-coded. `identify = pass`. (Filenames also carry non-team ids — e.g. jersey `name_{style}_…`
  reaches the 400s — which is why the position is derived, not assumed.)
- **Equipment** (`rendering/{helmet,glove,blocker,trapper,goaliepad,goaliehelmet,goaliestick,
  playerstick}`) — **mixed keying**. Goalie masks are **team-keyed** (`goaliehelmet_{team}_…`;
  `goaliehelmet_0` → Anaheim, verified), but gloves / sticks / pads are **brand / equipment-keyed**.
  The player↔equipment link (via the equip tables — cf. `xmja`/`fMVl` on later titles) is **deferred**
  → `identify = partial`.

## 5. Not yet mapped

- Front-end logo contexts (`cache.big fe/ion/artassets/teamlogos{,dynasty,wide,small,calendar}`) —
  same `kOtt.lOnr` id is expected to key these too, pending confirmation.
- Appearance / heads (Phase 5.4), and the `cards/playerheads_special` + `createplayer/head` portrait
  variants.
- The player↔equipment identity (brand/equipment ids on the player equip tables).
