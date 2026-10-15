# Audits

Verified findings from the pre-engine audit of the `anchors/` data and the NHLViewNG EXE. Everything
here was independently reproduced from ground truth (the anchor binaries); nothing was taken from a
staging file or reference doc without verification.

## Documents

| Doc | What it establishes |
|---|---|
| [staging-tsv-verification.md](staging-tsv-verification.md) | **Verdict: all 5 staging TSVs verified EXACT** against the binaries |
| [schema-drift-matrix.md](schema-drift-matrix.md) | Field-set drift of all 7 games vs Legacy (extends the TSVs to nhl14) |
| [nhlng-schema.md](nhlng-schema.md) | Verified field-level schema corpus (7 games) overview |
| [roster-relationships.md](roster-relationships.md) | Verified roster→player bridge chain, companions, eGlu line layout, enum-field depths |
| [cross-platform-format-audit.md](cross-platform-format-audit.md) | Xbox360 (=PS3 DBs) + PS4 (LE TDB v6) format verification; save containers net-new |
| [roster-capacity.md](roster-capacity.md) | **Spike S3** — per-(title, platform) roster-save capacity ceilings (`max_records`) from real bytes; gates Phase 4.4 |
| [anchors-manifest.md](anchors-manifest.md) | Inventory of every anchors file (sha256) |
| [qa-ledger.md](qa-ledger.md) | **Phase 0** — fixture freeze, per-anchor schema snapshots, QA-matrix ledger + how to regenerate them |
| [surface-capability-audit.md](surface-capability-audit.md) | **Phase 1** — every DB surface's requirements, the capability contract, frontend gating, and known-defect disposition |
| [scalar-mutation-audit.md](scalar-mutation-audit.md) | **Phase 2** — the codec taxonomy, the read→edit→save→reopen→undo→redo→reject contract, string/identity policy, and the editable-key defect fix |
| [relational-operation-audit.md](relational-operation-audit.md) | **Phase 3** — the roster operation set, schema-derived applicability (cluster vs occupancy ops on gen6), the invariant battery + reversibility, and the clean-refusal negative gate |
| [save-container-audit.md](save-container-audit.md) | **Phase 4** — content-based save detection, the v1/v2/v3/v4/PS4-LZ4 container round-trip contract (checksums, opaque regions, deterministic no-op, modified-payload fidelity), ROSTER app-seam edits, and malformed-input rejection |
| [nhlviewng-internals.md](nhlviewng-internals.md) | Static EXE audit; cross-references the functional report |
| [headmesh-uv-compat.md](headmesh-uv-compat.md) | **HM-S1** — measured head-UV atlas classes per (title, platform): X360 RW Legacy≡15, Kettle/NHL24 one class, NHL 10 distinct; cross-class texture transplant not interchangeable |
| [legacy-head-donor-census.md](legacy-head-donor-census.md) | **HM (M1)** — which Legacy palette slots vanilla heads actually weight (39 used, exclusions 20/49/80 confirmed, slot 5 contested); gates Path A remap targets |
| [nhl24-legacy-patha-roundtrip.md](nhl24-legacy-patha-roundtrip.md) | **HM (M2)** — NHL24→Legacy Path A conversion round-trip verified on a real head (strict re-parse, exact texlib shape, UV hash matches EA's own PS4 build, palette usage contained); first `roundtrip_verified` matrix row |
| [nhl21-ps4-head-asset-binding.md](nhl21-ps4-head-asset-binding.md) | NHL 21 PS4 — full head/hair/eyebrow/portrait binding chain from `ubPc` to the Attrib/VLT registry and on-disk assets; Attrib/VLT container decoded; pool-head + hair + eyebrow + portrait links verified, bespoke `headid`→collection binding recorded as **unresolved** with the negative evidence |

Binary format specs produced by the audit live under [../formats/](../formats/): the verified
[TDB v6 format](../formats/tdb-v6-format.md), [PS3 save container](../formats/ps3-save-container.md),
and [metadata XML](../formats/metadata-xml.md).

## Machine-readable data ([`data/`](data/))

Regenerate the Phase-0 set with `cargo run -p tdb-qa --bin qa-inventory` (see
[qa-ledger.md](qa-ledger.md)); the others below are earlier hand-made artifacts.

| File | Contents |
|---|---|
| `schema/<platform>-<game>.json` (×27) | **Phase 0** — per main-DB schema snapshot: endian, table/field ids+names, kind, bit width/offset, ranges, update flag, key, FK, current/max records (capacity) |
| `schema-diff.csv` | **Phase 0** — field-level cross-title/cross-platform diff (added/removed/resized/retyped/renamed + PS3↔Xbox360 parity) |
| `game-qa-matrix.csv` | **Phase 0** — QA ledger: game×platform×feature-family with status/evidence/blocker |
| `surface-capabilities.csv` | **Phase 1** — probed availability state of every surface on every main-DB anchor (drift-guarded) |
| `mutation-matrix.csv` | **Phase 2** — representative field + baseline/edited value proving every codec round-trips on every main-DB anchor (drift-guarded) |
| `operation-matrix.csv` | **Phase 3** — applicability + outcome (pass / refused-clean / no-fixture) of every roster operation on every main-DB anchor (drift-guarded) |
| `save-matrix.csv` | **Phase 4** — content-detected type + round-trip outcome (edit-roundtrip / experimental / roundtrip / read-only) of every committed save fixture (drift-guarded) |
| `anchors-manifest.csv` | **Phase 0** — every anchors file: path, platform, game, role, bytes, sha256 |
| `drift-matrix.csv` | (earlier) all 7 PS3 games × tables, table-level drift category + counts vs Legacy — superseded field-wise by `schema-diff.csv` |
| `savefile-inventory.csv` | (earlier) every PS3 save: type, title, container magic, layout, inner-TDB flag |

## Method

A from-scratch, read-only TDB v6 parser (the namespace reversal, header, directory, field-count at
`+0x1c` LE, and 16-byte descriptors) was validated to **100% agreement** with the meta XML on table
sets, field sets, and bit-depths before any finding was recorded. Analysis tooling was throwaway
(scratchpad, not committed) per the audit plan; only verified docs + data are committed.
