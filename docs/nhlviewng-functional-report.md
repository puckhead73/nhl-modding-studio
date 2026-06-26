# NHLViewNG Functional Report

Consolidated functional/feature map of the reference editor **NHLViewNG** — how it edits, how it saves, and
the metadata it edits through. This report is the feature-parity baseline for the studio's edit/save redesign.

> **Relationship to existing audits.** This report builds on and cross-references three prior evidence docs;
> it does not repeat their detail:
> - [`nhlviewng-validation-audit.md`](nhlviewng-validation-audit.md) — the 3-level validation contract.
> - [`nhlviewng-line-visibility-audit.md`](nhlviewng-line-visibility-audit.md) — eGlu 1-bit position-flag +
>   ZBac-bridge line storage mechanism.
> - [`nhlviewng-semantic-audit.md`](nhlviewng-semantic-audit.md) — table/field display-name recovery from the
>   EA DB Editor XML.
>
> **Evidence basis.** Static analysis only (printable ASCII + UTF-16 strings, DFM/RTTI class names) of
> `…\Reference Data\Database Definitions\nhlviewng_unpacked.exe` (6.3 MB, PE32/i386, Delphi/VCL,
> v3.0.2211.12, built 2022-11-12, author http://www.artemkh.com), plus the companion XML schema files. EXE
> findings are research signals, not parser truth — consistent with the architectural rule in the prior audits.

---

## 1. Game & format coverage

- Per-game RTTI classes `TNHLDatabaseNG2008 … 2016` → one binary reads/edits **NHL 09 through 16**.
- TDB format **version 6**; internal db shortnames `emag` / `etir`.
- Relational schema held in memory via `TtdbDatabaseHeader` + `TtdbMetaDatabase / TtdbMetaTable /
  TtdbMetaField / TtdbMetaIndex` — a **full relational meta-model**, not a flat field list.
- **134 tables** known (player/roster/bio, skater + goalie AI, equipment, custom face/palette/jersey,
  9 league schedules + future/favorites, draft, contracts/RFA/retirement, awards, news/tweets/interviews,
  arenas, GM logic, season/playoff/career stat blocks).

## 2. File I/O — load & save

| Action (DFM) | Meaning |
|---|---|
| `actFileOpenPS3 / Accept` | Open exported PS3 save dir; **auto-decrypts and re-encrypts** on save (`TPS3ProtectedFile`, `TPS3Pfd`, `CloneEncryptDecrypt`). |
| `actFileOpenPS4 / Accept` | Open exported & decrypted PS4 save directory. |
| `actFileOpenXbox360 / Accept` | Browse for a roster extracted from an Xbox360 con-file. |
| Plain `.db` open | "Opens the roster file stored in a plain tdb-file." |
| `actFileSave` | Single save action across all container types; `AutoSave` flag present; `.bak` backup strings present. |

- Container primitives present: `zlib`, `compress`, `encrypted/decrypted`, `CRC`, `checksum`, `magic`,
  `endian` → NHLViewNG owns container framing, integrity tags, and byte-order across platforms.
- **One save model.** Edits are held against the in-memory relational schema; on save the **whole container is
  re-serialized** (header preserved, payload recompressed, integrity tags recomputed) — the same shape as the
  studio's `pack_db`, but NHLViewNG additionally writes **v1 (NHL10/11)** and **PS4** containers the studio
  cannot.

## 3. The metadata it edits through (key insight)

NHLViewNG drives editing off **rich per-field metadata**, not raw 4-char keys. The companion XML files are the
externalized form of that metadata:

- **`NHL 14 (EA DB Editor).xml`** — keyed by the **real DB raw IDs** (`ubPc`=Player Bios, `dSvy`=Skater AI,
  `kOtt`=Teams, `eGlu`=roster/lines). Provides human table/field names, per-field `ControlType`
  (TextBox/dropdown), **grid column order per table**, and FK edges (`<Source><Table>…</Table><Field>…`). It
  encodes the **eGlu line-editor layout**: the `(ID)` block (`dXSB` team, `XSWT` player-instance, `sVRt`
  jersey, `mCcl` captaincy, `QgFs` archetype, `toWX` unique-id) followed by **19 named line groups** —
  `C, LW, RW, LD, RD, G, PP5-1/2, PP4-1/2, PK4-1/2, PK3-1/2, SO, X, OT1/2/3` — each a set of 1-bit position
  flags (see [`nhlviewng-line-visibility-audit.md`](nhlviewng-line-visibility-audit.md)).
- **`nhlng-meta (DB Converter).xml`** (444 KB) — `type` (FIELDTYPE_INTEGER…), `depth` (bit width),
  `rangelow/rangehigh`, `key`, `update` (writable flag), `savegroups`. This is the **validation backbone**:
  every cell knows its width, legal range, whether it is editable, and which save-context includes it.
- In practice **each game ships its own `db/nhlng-meta.xml`**, loaded per-game (defs also embedded as binary
  RCDATA). Field semantics are therefore **data-driven per game** — the reason one binary spans 09–16.

## 4. Editing features (from DFM actions/components)

| Capability | Evidence |
|---|---|
| Create player | `actDatabaseCreatePlayer` — "Create a new instance for this player and copy to the right team." |
| Delete player | `DatabaseDeletePlayer` (+`_Disabled`). |
| Copy / paste player | `DatabasePastePlayer`; clone family (`Clone`, `WritableClone`, `Duplicate`). |
| Trade / move | `btnTradeMoveLeft/Right`, `btnTradeCopyLeft` — two-team move/copy UI; `keepcustomstickiftraded`, `BensKeepCustomIfTraded` preserve appearance across trades. |
| Roster types | `LoadRoster`, `LoadFreeAgentsRoster`, `LoadHiddenRoster` (main / free agents / hidden). |
| Roster assignment | `SetRosterPlayerStatus`, `SetRosterPlayerCaptaincy`; live list `lstTmRoster*` (edit/compare/custom-draw). |
| Lines | eGlu 19-group line layout (§3) — full line editor. |
| Jersey number | `actTeamJerseyNumber`. |
| Attributes | 60+ rating fields (skater `s_*`, goalie `m_*` incl. `m_gsl/gsh/ssl/ssh/5hole/potential/poise…`). |
| Equipment | jersey/helmet/visor/stick + goalie 9-zone RGB (blocker/trapper/pads zones 1–9 `_r/_g/_b`). |
| Appearance | face/eyes/hair/skin/eyebrow/shoulderpads; custom face/palette tables. |
| Draft / contracts | `DraftPos/Draftable/DraftTeam`, `leaguedraftpick`, RFA/retirement/contract-offer tables. |
| AI styles | `TNHLPlayerAiStyle(s)`, `TNHLGoalieAiStyle(s)` per game year. |
| Schedules | 9 league schedules + future + favorites. |

## 5. Model takeaways that matter for parity

1. **Entity-first, metadata-validated.** Edits target logical entities (player, roster slot, team) and are
   validated against per-field width / range / `update` / FK *before* they are accepted.
2. **Structural ops auto-maintain consistency.** "Create player" creates the instance *and wires it onto the
   right team*; trade *moves the instance*. The user never hand-maintains bridge/companion rows.
3. **One re-serialize save** across PS3/PS4/Xbox/plain-TDB with integrity tags + `.bak` backup; writes v1
   (NHL10/11) and PS4 — both of which the studio currently cannot.
4. **The validation contract is explicit and layered** (file/schema integrity → whole-DB NHL integrity →
   operation-specific restrictions); see [`nhlviewng-validation-audit.md`](nhlviewng-validation-audit.md).

These four properties are exactly what the studio's reactive, raw-field edit model lacks; the comparison and
the redesign direction are in [`studio-vs-nhlviewng-comparison.md`](studio-vs-nhlviewng-comparison.md).
