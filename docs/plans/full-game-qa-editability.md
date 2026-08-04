# Full-game QA and editability plan

> **Status:** planned 2026-08-01  
> **Scope:** all 17 catalog entries: NHL 07–21, NHL Legacy, and NHL Legacy Recompiled.  
> **Ground truth:** the 27 committed main-database anchors under `anchors/`, their save fixtures,
> metadata XML, extracted asset trees where available, and explicit emulator/game validation.
>
> Companion: [codebase game-parity audit](../audits/codebase-game-parity-audit.md) and
> [scope/compatibility matrix](../01-scope-and-compatibility.md).

## Objective

Establish evidence-backed editing parity without assuming that adjacent titles share schemas,
container rules, or asset layouts. Every supported game/platform must:

1. show only data its loaded database actually contains;
2. label, decode, validate, and encode that data correctly;
3. expose every intended editable field and operation supported by that schema;
4. save and reload without unrelated byte or relational changes;
5. produce a file accepted by the target game/emulator; and
6. advertise asset capabilities only where discover/export/replace/install are verified.

“The screen opens” is a smoke test, not parity. “Fully editable” requires all applicable gates in this
plan to pass. A feature absent from a game's source data is correctly **omitted**, not counted as a
failure and not rendered as an empty placeholder.

## Support units and status model

Track support by **game × platform × feature family**. Do not use one blanket title flag.

### Catalog entries

| Family | Entries | Platforms represented by anchors |
|---|---|---|
| Gen6 | NHL 07 | Xbox 360; PS3 research DB copy is not a catalog target |
| Early Gen7 | NHL 08–10 | PS3, Xbox 360 |
| Gen7 | NHL 11–15, NHL Legacy | PS3, Xbox 360 |
| Recompiled Gen7 | NHL Legacy Recompiled | Recomp, using the Xbox-derived Legacy schema |
| Gen8 | NHL 15–21 | PS4 |

### Independent status axes

Each cell records one of: `not-applicable`, `missing-fixture`, `discovered`, `read-pass`,
`edit-roundtrip-pass`, `game-verified`, or `blocked`.

- **Database presentation:** players, teams, arenas, career, attributes, and raw field disclosure.
- **Database operations:** scalar/string/enum/FK edits plus structural roster operations.
- **Save path:** raw DB, inner save container, and outer platform package where applicable.
- **Assets:** portraits, logos, jerseys, equipment, heads/appearance, arenas, and audio.
- **Onboarding:** install discovery, metadata/localization resolution, save discovery, extraction.

Only `game-verified` satisfies release parity for write paths. Automated round trips are necessary but
cannot prove that an integrity word or semantic encoding is accepted by the game.

## Definition of fully editable

A game/platform is database-editable when all applicable items below pass:

- Every metadata field is classified as surfaced-curated, surfaced-raw, intentionally read-only, or
  intentionally omitted with a documented reason.
- Curated UI contains no field whose table/field is absent and no empty section or unavailable tab.
- Every updatable field has a working control matching its semantics: string, signed/unsigned number,
  boolean, enum, date/offset value, foreign key, rating scale, or asset reference.
- Friendly display and friendly input are inverse operations for all encoded fields.
- `update=False`, keys, identity fields, and unsafe structural pointers cannot be edited as scalars.
- Player/team/arena edits, attributes, career rows where supported, lines, strategies, captaincy,
  jerseys, contracts, draft picks, roster status, and memberships work when their backing data exists.
- Create, duplicate, move, trade, release/sign, retire/unretire, position flip, and delete/hide either
  preserve all invariants or are hidden/disabled with an explicit schema-specific reason.
- Undo/redo, validation, save, reopen, and a second no-op save all succeed.
- The edited file loads in the game/emulator and the edited values appear correctly.

Asset “fully editable” is evaluated separately per asset family. A database can reach full DB parity
while an asset family remains unsupported.

## Phase 0 — Freeze inventory and build the QA ledger

### Work

1. Generate a canonical manifest of every main DB, metadata file, localization DB, auxiliary DB, save
   fixture, archive, and representative extracted asset.
2. Record hashes and treat anchors as immutable; all mutations occur in temporary copies.
3. Create `docs/audits/data/game-qa-matrix.csv` with one row per game/platform/feature family and these
   columns:
   `game,platform,generation,fixture,status,automated_test,manual_case,last_verified,evidence,blocker`.
4. Add a checked-in schema snapshot for every main DB: table ids/names, field ids/names/types,
   ranges, update flags, FK declarations, record counts, and capacity.
5. Diff schema snapshots by generation and title. Every added, removed, renamed, resized, or re-encoded
   field becomes an explicit review item.

### Gate

- Every catalog choice maps to a documented install layout and at least one schema fixture.
- Every claimed platform has a database and/or save fixture, or is marked `missing-fixture`.
- CI detects anchor removal/hash changes and schema-snapshot drift.

## Phase 1 — Presentation and capability audit

### Automated matrix

Extend the existing 27-anchor profile gate to inspect:

- player summaries and full skater/goalie views;
- team summaries/views and arena joins;
- player/team curated profiles and raw-field disclosure;
- attributes, traits, career stats, memberships, lines, strategies, contracts, draft picks, retired
  numbers, jerseys, and equipment;
- field record targets, companion record targets, FK targets, enum membership, decoded ranges, and
  duplicate labels/ids;
- DTO conversion and frontend rendering contracts.

Run more than the first entity. Select deterministic representatives for each relevant shape:

- skater positions LW/C/RW/D;
- goalie;
- rostered, free-agent, inactive/hidden, multi-team, and missing-companion players;
- stock/custom/international/all-star teams;
- populated and empty optional tables.

### Capability expansion

Replace unconditional tab rendering with probed capabilities for each surface:

`players, teams, arenas, career, lines, strategies, captaincy, contracts, draft_picks, jerseys,
team_equipment, player_equipment, portraits, logos, heads, audio`.

A capability is derived from required tables/fields and verified joins, not from `game_id`. Tabs are:

- omitted when the schema does not carry the feature;
- disabled with a precise reason when data exists but writing is not verified; or
- enabled only after the corresponding read/edit gate passes.

### Manual UI sweep

Open every game/platform and inspect each enabled route at desktop and narrow widths. Confirm labels,
empty states, controls, enum ordering, derived values, error messages, pending-edit summaries, and
navigation. Capture one screenshot set per materially different schema family.

### Gate

- Zero phantom fields, raw ids on normal surfaces, broken joins, invalid enum labels, or unconditional
  tabs without backing data.
- Every actual database field has an explicit presentation disposition.

## Phase 2 — Scalar field mutation matrix

Build a data-driven test harness that runs on a temporary copy of every main DB anchor.

### Field classes

For at least one field of every distinct encoding in every schema:

1. Read baseline raw and friendly values.
2. Choose a different valid value, including boundary cases.
3. Stage the edit through the same domain/API path used by the UI.
4. Confirm the pending edit identifies the correct entity and friendly values.
5. Serialize, validate CRC/size, reopen, and confirm raw and friendly values.
6. Undo and prove byte equality with baseline.
7. Redo, save again, and prove deterministic output.
8. Confirm an invalid value is rejected without changing bytes or undo history.

Cover fixed strings, signed integers, booleans, enums, ratings, date offsets, salary/year scales,
nullable sentinels, team/arena FKs, and little-/big-endian fields. Include maximum-length and Unicode
policy tests for strings.

### Gate

- Every distinct editable codec has an inverse read/write proof on every schema family using it.
- No edit changes unrelated records, fields, table headers, or string-pool bytes.
- Read-only and unknown-semantic fields remain non-editable.

## Phase 3 — Entity and relational operation matrix

Run each applicable high-level operation on every materially different roster schema, then expand to
every anchor once the family tests pass.

### Player lifecycle

- Create skater and goalie with complete index/AI/equipment companions required by that game.
- Duplicate with and without appearance/assets.
- Change position within skater positions and skater ↔ goalie; verify companion creation/removal or
  reject safely where the operation is not implemented.
- Move, trade, release, sign, hide, restore, and delete.
- Change main team and add/remove extra memberships without confusing the two.

### Team/roster lifecycle

- Fill and vacate roster slots at normal and capacity boundaries.
- Lines: place, swap, clear, best-lines, dressed/scratched/minors, and line visibility.
- Jersey uniqueness/autocorrect, captain/alternate rules, and retired-number operations.
- Strategies and their career mirrors where present.
- Contracts, rights, affiliates, draft picks, and team/arena relationships.

### Invariants after every operation

- no dangling FK or duplicate player/team identity;
- correct bridge and companion cardinality;
- correct team block alignment and slot ownership;
- no duplicate roster membership or jersey where prohibited;
- exactly zero/one captain as required;
- table capacity, CRC, and record counts valid;
- audit delta contains no newly introduced finding.

### Gate

- Applicable operations pass edit → validate → save → reopen → validate on all anchors.
- Non-applicable operations are absent/disabled, never allowed to fail halfway through a transaction.

## Phase 4 — Save/container and onboarding matrix

### Automated

For every save fixture:

- detect type by content, not folder name;
- extract and validate the inner DB;
- perform a representative scalar and structural edit;
- repack, reopen, and verify the edit;
- verify checksums, tags, framing, preserved opaque regions, and deterministic no-op behavior;
- test overwrite backup/recovery and save-as behavior;
- confirm malformed/truncated input fails without damaging the original.

Cover Gen6, v1, v2, v4, PS4 framing variants, raw DBs, and each supported multi-file layout. Treat Xbox
STFS and multi-part dynasty/season as blocked until explicitly implemented and fixture-tested.

### Manual

For each game/platform, load the edited save in the intended emulator/game, inspect the changed player
and team, play or simulate enough to force another game save, then reopen that new save in Studio.
Record emulator/game build, fixture hash, operations, result, screenshots, and resulting save hash.

### Gate

- Structural round-trip and game acceptance both pass.
- NHL 10/11 v1 and PS4 remain visibly experimental until their manual cases pass.

## Phase 5 — Asset capability and editability matrix

Assets use observed format capabilities, not database-title assumptions.

### Per asset family

For portraits, logos, jerseys, equipment, heads/hair, arenas, and audio:

1. Discover and match assets to human-readable player/team/slot identities.
2. Decode and preview every encountered format variant.
3. Export to a documented interchange format.
4. Re-import an unchanged export and prove semantic or byte identity as appropriate.
5. Replace with a visibly modified fixture, rebuild archive/index/pool metadata, and reopen it.
6. Install into a copied game tree and verify in-game.
7. Exercise size, mip, palette, compression, atlas/pool, duplicate-name, and capacity boundaries.

Track `discover`, `preview`, `export`, `replace`, `install`, and `game-verified` independently for each
asset family and game/platform. Unsupported ChunkLZX or pixel/mesh/audio variants must disable only the
affected operation and explain the exact format encountered.

### Gate

- No generic “Assets supported” claim. Each asset family has its own green matrix row.
- Original archives remain recoverable and modified archives reopen cleanly.

## Phase 6 — Import, formulas, and cross-game conversion

- Map every importable identity, enum, birthplace, style, bias, contract, draft, and rating field to
  the target's probed schema.
- For absent target fields, omit them and show the omission in preview; never synthesize phantom data.
- Validate formula ranges against the destination field's actual scale.
- Run the same historical player into each schema family and compare the preview with committed values.
- Test matching, create, overwrite, duplicate prevention, batch atomicity, undo, and ledger behavior.
- For cross-game assets, qualify each source → target pair independently.

### Gate

- Preview exactly predicts commit.
- Every dropped value is itemized with a reason.
- Imports produce audit-clean, reloadable, game-accepted files.

## Phase 7 — Release hardening and continuous enforcement

### CI suites

- `fast`: unit tests, DTO generation/diff, TypeScript build, lint, schema snapshot check.
- `anchor-read`: all database presentation/capability gates.
- `anchor-edit`: scalar and relational mutation matrix on temporary copies.
- `container`: all save extraction/repack fixtures.
- `asset`: codec/round-trip fixtures, split by generation to keep runtime manageable.
- `manual-required`: machine-readable check that every release-target matrix cell has current evidence.

Shard large anchor suites by platform/title and publish JUnit plus the generated QA matrix as artifacts.
Never run destructive tests directly against `anchors/`.

### Promotion policy

A feature moves to supported only when:

- automated gates pass on the exact game/platform fixture;
- a manual game/emulator verification record exists where writes are involved;
- no unresolved critical/high defect affects that feature;
- UI capability gating and documentation match the evidence; and
- the verification is rerun after changes to parsing, metadata, editing, validation, packaging, or the
  relevant asset codec.

## Execution order

1. Phase 0 inventory/ledger and schema snapshots.
2. Phase 1 presentation/capabilities, because it prevents the UI from lying while deeper work proceeds.
3. Phase 2 scalar mutations across all anchors.
4. Phase 3 relational operations, proven by schema family then expanded across the matrix.
5. Phase 4 containers/onboarding in parallel with the manual game-validation campaign.
6. Phase 5 assets, one family at a time: portraits/logos → jerseys → equipment → appearance/heads →
   arenas/audio.
7. Phase 6 import/conversion after destination edit codecs are green.
8. Phase 7 release enforcement stays active throughout.

## Defect policy

Every failure records:

- game/platform and exact fixture hash;
- surface/operation and expected versus actual behavior;
- table/field/record or asset/archive entry involved;
- whether the failure is presentation, semantic decode, validation, mutation, serialization, container,
  discovery, or in-game acceptance;
- reproducible steps and smallest fixture;
- severity: `critical` corruption/data loss, `high` wrong edit or unusable feature, `medium` missing or
  misleading surface, `low` cosmetic/documentation;
- regression test required before closure.

Critical/high defects demote the affected matrix cell immediately. Neighboring titles are not demoted
automatically, but must be rechecked when they share the changed schema or codec path.

## Completion deliverables

- `game-qa-matrix.csv` with no ambiguous blanket statuses.
- Generated schema/capability snapshots for all 27 anchors.
- Full presentation and mutation suites over immutable anchor copies.
- Save-container verification reports and manual in-game evidence per supported target.
- Asset-family matrices and round-trip fixtures.
- UI capability gating that exactly follows the matrix.
- A release report listing supported, experimental, not-applicable, and blocked features for every
  catalog entry and platform.

The practical finish line is simple: if Studio shows an edit control, the value survives save/reload
and appears correctly in that game. If the game does not have the field or the write path is not proven,
Studio does not bluff.
