# Roster Relationships & Entity Model (verified)

> **Verification status: VERIFIED against the anchors** (nhl11→legacy `nhlng.db` + meta XML, all
> games). This re-verifies, on our ground truth, the relationship/companion/line claims from the
> staging audits (`nhlviewng-validation-audit.md`, `nhlviewng-line-visibility-audit.md`) — which had
> analyzed a *different* `default.db`. Several points are now **stronger** than the staging evidence
> (the bridge FK is *declared in the meta*, not merely inferred from value overlap). Ids shown in the
> "real" convention with raw in parens; see [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md#the-id-reversal-rule-read-first).

## The roster → player bridge chain (CONFIRMED)

```
kOtt (exhibitionteams)            team, key vfEq (8-bit)
  ▲
  │  eGlu.dXSB → kOtt.vfEq        (eGlu.dXSB name="team"; implicit FK, naming-confirmed)
  │
eGlu (exhibitionrostertable)      roster + lines; key toWX
  │  eGlu.XSWT → ZBac.vfEq        ✅ DECLARED in meta (XSWT fkparenttable="exhibitionplayers")
  ▼
ZBac (exhibitionplayers)          player index; key vfEq (14-bit)
  │  ZBac.ykFq → ubPc.wBIz        (ZBac.ykFq name="playerid"; implicit FK; staging value-overlap 100%)
  ▼
ubPc (exhibitionplayerbiotable)   player bios; key wBIz (14-bit)
```

| Hop | Field (real/raw) | depth | meta name | Evidence |
|---|---|---:|---|---|
| eGlu→ZBac | `eGlu.XSWT` / `ulGe.TWSX` | 14 | `playerindex` | **meta `fkparenttable=exhibitionplayers`** — declared, all games |
| ZBac→ubPc | `ZBac.ykFq` / `caBZ.qFky` | 14 | `playerid` | implicit (no declared fk); name + staging 100% value-overlap |
| eGlu→kOtt | `eGlu.dXSB` / `ulGe.BSXd` | 8 | `team` | implicit; name="team"; staging-confirmed |

Keys: `eGlu.toWX` (name `key`, 14-bit) is eGlu's primary key; `ZBac.vfEq` (`index`), `ubPc.wBIz`
(`game_id`), `kOtt.vfEq` (`index`) are the respective primary keys.

## Player companion records (CONFIRMED — 1:1 via shared key `wBIz`)

A Player is the `ubPc` bio row plus companion rows that **share ubPc's primary key `wBIz`** (verified
key in every game):

| Companion (real/raw) | meta name | Role | For |
|---|---|---|---|
| `dSvy` / `yvSd` | exhibitionskaterai | skater ratings/AI | skaters |
| `xmja` / `ajmx` | exhibitionskaterequipment | skater equipment | skaters |
| `mHuy` / `yuHm` | exhibitiongoalieai | goalie ratings/AI | goalies |
| `fMVl` / `lVMf` | exhibitiongoalieequipment | goalie equipment | goalies |

Mechanism is **shared-key 1:1**, not a declared `fkparenttable` (companions carry `key=wBIz`, the same
id as `ubPc.wBIz`). This confirms the validation-audit's required-companion rule: a usable skater
needs `dSvy`+`xmja`; a goalie needs `mHuy`+`fMVl`.

## eGlu field shape (CONFIRMED)

| Game | eGlu fields | 1-bit fields | Line-slot flags |
|---|---:|---:|---:|
| nhl11, nhl12 | 66 | 60 | ~59 (60 − `hSZj`) |
| nhl13, nhl14, nhl15, legacy | 78 | 72 | **71** (72 − `hSZj`) |

The jump from 60→72 one-bit fields at **NHL13** matches the staging doc's "NG2013 added 12 OT slots".
The non-flag eGlu fields (NHL13+): `XSWT`(14, playerindex), `toWX`(14, key), `dXSB`(8, team),
`sVRt`(7, **jersey number**), `QgFs`(4, **playerstyle**), `mCcl`(2, **captaincy**); plus `hSZj`(1,
**rosterstatus**) sits among the 1-bit fields. The remaining **71 one-bit fields are per-player
line-assignment activation flags** ("this roster-player occupies slot S"); the slot→hockey-position
label mapping for all 71 is **not** fully resolved here (needs the NHL14 defs or a controlled-diff).

## Enum-shaped fields (Spike S4 — mostly resolved)

The fields are real and sized as expected. Spike S4 (2026-07-13) recovered the authoritative
vocabularies from the **NHLViewNG EXE Delphi RTTI** (unit `uDatabaseTypes`, per-instance clean-room
exception) — the same provenance path that locked captaincy. Status of each:

| Field (real) | depth | meta name | Label status |
|---|---:|---|---|
| `eGlu.mCcl` | 2 | captain | ✅ **Confirmed** — `TNHLRosterPlayerCaptaincy` `0=None, 1=Captain, 2=Alternate` (shipped [roster-captaincy.toml](../../defs/enums/roster-captaincy.toml)) |
| `eGlu.QgFs` | 4 | playerstyle | ✅ **Confirmed** — the Phase 5T Player Style mapping, **re-verified 1-based** against the anchor (D=1..4, F=5..10, 0=unset); now bound to `eGlu` + `dSvy` + career mirrors ([player-style.toml](../../defs/enums/player-style.toml)) |
| `eGlu.hSZj` | 1 | rosterstatus | ✅ **Resolved** — the **real** dressed/scratched flag: 1-bit Delphi `Boolean` (anchor split `{0:1092, 1:4440}`); no enum vocabulary needed |
| `ZBac.RREB` | 2 | table | ✅ **Resolved (S4 follow-up)** — **player TYPE, not roster status**: `TNHLPlayerType = {Built-In, User-Created, Special, Developer}`, confirmed by EXE RTTI + the field's "table" meta name + a 1:1 correlation with the pointed-to bio `active` flag (shipped [player-type.toml](../../defs/enums/player-type.toml)). See [roster-pools.md](roster-pools.md#zbacrreb-is-player-type-not-roster-status--resolved-spike-s4-follow-up-2026-07-13). |

> The QgFs offset is **field-specific**: the skater code (`dSvy`/`eGlu`) is **1-based** (0=unset,
> D=1..4/F=5..10), while the sibling goalie code (`mHuy.QgFs`) is **0-based** (0=Standup). The EXE
> RTTI enum `TNHLPlayerAiStyle` is NHLViewNG's own 0-based editor vocabulary — it corroborates the
> skater codes at a +1 offset but is not the field codec (it carries an internal `psDangler` the
> 4-bit field never uses, and disagrees at code 9, where the anchor distribution confirms Two-Way
> Forward). `TNHLPlayerContractStatus = {csUnrestricted, csSigned, csUnsigned}` and
> `TNHLRosterPlayerStatus = {Scratched, Dressed, InMinors}` were also recovered from the EXE but have
> no confirmed field home in `nhlng.db` — the on-disk dressed/scratched signal is the 1-bit
> `eGlu.hSZj` boolean, and `RREB` turned out to be player type (not status).

## Confirmed vs still-open

- ✅ **Confirmed (schema-level, all games):** table identities + roles (via meta names); the
  eGlu→ZBac→ubPc bridge (XSWT FK declared); eGlu→kOtt team link; the four 1:1 companions via `wBIz`;
  eGlu field shape + the 60→71 slot-flag growth at NHL13; the enum-field depths.
- ⏳ **Needs the record decoder (data-level):** value-resolution checks (that every `ZBac.ykFq`/`XSWT`
  actually resolves; the 2 sentinel orphans). The TDB record-layout (descriptor bytes 8:16) isn't
  decoded yet — see [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md#open-items).
- ✅ **Confirmed via EXE RTTI + anchor data (Spike S4):** captaincy, player style, goalie style,
  `hSZj` (boolean), and `ZBac.RREB` (player type). The only enum-ish thing still open is the per-slot
  hockey-position names for the 71 line-assignment flags.
