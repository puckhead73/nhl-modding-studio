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

## Enum-shaped fields (depths CONFIRMED; labels CANDIDATE)

The fields are real and sized as expected, but the **value→label mappings are NOT locked** — they need
operator confirmation in NHLViewNG (or a controlled diff), per the staging docs:

| Field (real) | depth | meta name | Candidate labels (UNCONFIRMED) |
|---|---:|---|---|
| `eGlu.mCcl` | 2 | captain | `0=None, 1=Captain, 2=Alternate` (rcNone/rcCaptain/rcAlternate) |
| `ZBac.RREB` | 2 | table | roster status — `Scratched/Dressed/InMinors` + a default; ordering unconfirmed |
| `eGlu.QgFs` | 4 | playerstyle | TNHLPlayerAiStyle (~10 labels) |
| `eGlu.hSZj` | 1 | rosterstatus | dressed/active flag |

## Confirmed vs still-open

- ✅ **Confirmed (schema-level, all games):** table identities + roles (via meta names); the
  eGlu→ZBac→ubPc bridge (XSWT FK declared); eGlu→kOtt team link; the four 1:1 companions via `wBIz`;
  eGlu field shape + the 60→71 slot-flag growth at NHL13; the enum-field depths.
- ⏳ **Needs the record decoder (data-level):** value-resolution checks (that every `ZBac.ykFq`/`XSWT`
  actually resolves; the 2 sentinel orphans). The TDB record-layout (descriptor bytes 8:16) isn't
  decoded yet — see [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md#open-items).
- 🙋 **Needs operator confirmation:** all enum value→label mappings, and the per-slot hockey-position
  names for the 71 line-assignment flags.
