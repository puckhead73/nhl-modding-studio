# C0-BULK1 · Checkpoint 6 — Canonical Project and adapter contracts

> **Package:** C0-BULK1, checkpoint 6 of 7. **Date:** 2026-09-07. **Author role:** developer.
> **Status: Partial — corrected by review.** Nothing staged, committed or pushed. No support status
> promoted; `game-verified` is structurally unrepresentable in every file this checkpoint defines.

---

## ⚠ Review correction (C0-BULK1-R1, 2026-09-07)

This receipt originally called the contract **complete**. **That claim is withdrawn.** The adapter
format cannot yet represent facts Q07a.1 explicitly requires, so it cannot express every proven
database field and asset constraint — which is exactly C0's item-7 exit criterion:

| Missing representation | Q07a.1 requirement |
|---|---|
| enum vocabulary **and its stored code** | "enum vocabulary and stored code" |
| null sentinel | "null sentinel" |
| string encoding / pool behaviour | "string encoding/pool behaviour" |
| companion / cardinality rules | "companion/cardinality rules" |
| capacity | "capacity and presence/applicability" |

These were already registered as U-A3/U-A4/U-A6 in
[the contract document §5.3](../validation/c0-06-canonical-project-and-adapters.md); the error was in
this receipt calling the surrounding contract "complete" while they are absent. The accurate status
is: **the contract's *rule and validation machinery* is implemented and executing; the contract's
*expressive coverage* is incomplete, and so is its population.**

## Status

**Partial.** What is implemented and executing: the canonical schema, the versioned adapter format,
the asset profile format, fifteen rules, three validators and worked evidence-backed examples with
negative tests. What is **not** complete: the adapter format's expressive coverage (above) and its
population — 2 of 28 units and 1 of 14 asset families, stated as a measured figure rather than
implied.

## Files changed

| Path | Change |
|---|---|
| `defs/canonical/project-schema.toml` | **New.** `canonical-project/1` — 4 domains, 2 entities, 10 properties. Target-neutral by construction. |
| `defs/adapters/nhl15-xbox360.toml` | **New.** `target-adapter/1` — 4 mappings, 1 omission, declared coverage 4/2969. |
| `defs/adapters/nhl10-xbox360.toml` | **New.** `target-adapter/1` — the adjacent-title divergence, 2 mappings, 1 omission. |
| `defs/adapters/asset-profiles/assets-portraits.toml` | **New.** `asset-output-profile/1` — 2 target profiles, one a registered wholly-unknown gap. |
| `crates/tdb-canonical/` | **New crate.** Types, 15 contract rules, three validators, 16 tests. |
| `Cargo.toml` | Added `crates/tdb-canonical` to the workspace members. |
| `Cargo.lock` | New workspace member entry (see below). |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | **New.** The contract, coverage enumeration, and the C0 item-6 completion mapping. |
| `docs/receipts/c0-bulk1-06-canonical-contracts.md` | **New.** This receipt. |

## Implemented behavior

**Three versioned document kinds**, each with a schema id and its own validator. An adapter declares
`canonical_schema_version`; a mismatch is reported (`unsupported-version`), never reinterpreted.

**Layer separation is structural, not conventional.** The canonical schema has no field in which a
table id, field id, bit offset or endianness could be written; an adapter has no syntax that writes
back into a canonical property. So *"canonical data retains the richest known semantics"* and
*"reconciliation corrections affect target Build mappings, not canonical data"* (Q07a.3a) cannot be
violated by a malformed file — only by changing the types.

**Fifteen contract rules**, each with a negative test. The load-bearing ones:

| Rule | Effect |
|---|---|
| `inverse-write-missing` | a value-changing read with an unknown write is refused |
| `inverse-unverified-but-qualified` | `automated-tested` needs `inverse_verified` **and** a named case |
| `unknown-default` | `default = "unknown"` is refused — an unknown default blocks export |
| `adapter-claims-game-verified` | **no schema file can ever promote support** |
| `unknown-identity-but-qualified` | a proven *transform* is not a proven *mapping* |

**The validator caught this package's own over-claim.** The first draft mapped
`nhl15-xbox360/player.rating_overall` at `automated-tested` — citing the genuinely proven ratings
write codec — while its table and field ids were `unknown`. The rule refused it. **The data was
corrected, not the rule**, in both adapters.

**Worked evidence-backed examples.** NHL 15 stores birth month/day 0-indexed; NHL 10 stores them
1-indexed on the same platform and generation, with byte-identical EA metadata. Each adapter states
its own transform, and a test asserts they disagree — the concrete demonstration that an adjacent
title never inherits a neighbour's fact.

## Registry coverage — measured

| Layer | Coverage |
|---|---|
| Physical (existing snapshots) | **27/27 targets**, 3,121 tables, **69,737 fields**, 20 big-endian / 7 little-endian, TDB v6 throughout |
| Semantic adapters | **2 / 28 catalog units** |
| Semantic mappings | **6 mappings + 2 omissions** against 69,737 fields — **under 0.01%** |
| Mappings reaching `automated-tested` | **1** |
| Asset output profiles | **1 / 14 families**; **2 / 27** target profiles within it |

**C0 item 7 is therefore not complete**, and this receipt does not claim it is — neither in
population **nor in the contract's expressive coverage** (see the review correction). Seven
enumerated unknowns (U-A1 – U-A7) are registered, including three that are **format gaps** the
adapter schema must grow before it can be called complete: enum stored codes, null sentinels /
string-pool behaviour / companion rules, and roster capacity.

## C0 item 6 — the completion mapping

The evidence-cell half is now **mechanised**: the R2 declared universe emits **2,016 rows**, each
carrying applicability, required/optional status and `unresolved` tokens, regenerating instead of
drifting. The document maps all ten milestones (C1–C10) onto those families with their current test
route or explicit blocker.

**Ownership is the unmet half.** Every completion item has a milestone, an evidence cell and either a
test route or a recorded blocker; **no item has an accountable owner**, and assigning one is an
architect action. Recorded here, in the register §8 and in the mapping §6.

## Exact checks and results

| Check | Command | Result |
|---|---|---|
| Crate tests (16: 5 positive incl. all committed files, 11 negative) | `cargo test -p tdb-canonical` | **Passed** — 16/16 |
| Strict clippy, scoped | `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, scoped | `cargo fmt -p tdb-canonical -- --check` | **Passed**, exit 0 |
| Every committed `defs/` file validates | asserted by `the_committed_*` tests | **Passed** |
| Schema-snapshot totals | computed from the 27 committed JSON files | **Passed** |
| Repo-wide checks | — | **Not run** here; checkpoint 7 |

Negative tests, one per enforceable rule: missing inverse write, unknown default, `game-verified`
claim, missing provenance, unknown canonical property (mapping and omission), mapped-and-omitted,
duplicate mapping, `automated-tested` without inverse or case, unknown identity with a proven
transform, wrong canonical version, malformed target.

## Evidence / log locations

Everything is in-tree: `defs/canonical/`, `defs/adapters/`, `crates/tdb-canonical/src/lib.rs`
(tests inline), and
[docs/validation/c0-06-canonical-project-and-adapters.md](../validation/c0-06-canonical-project-and-adapters.md).

## Preservation checks

| Check | Result |
|---|---|
| HEAD / index | `a2bbdc4…` / **empty** |
| Existing crates | **none modified.** The only change outside the new crate and new `defs/` files is one line in the workspace `members` list |
| `defs/` existing files (`metadata/`, `enums/`, `appearance/`, `qa/scopes.toml`) | untouched; the new files are in new directories `defs/canonical/` and `defs/adapters/` |
| `docs/audits/**` | read only (schema snapshots, audit documents cited as provenance) |
| Anchors, game trees | not touched |
| `Cargo.lock` | `12b777ff…` → `d612b0f9…`, adding the `tdb-canonical` member entry and its `serde`/`toml` edges. Additive only; no locked version altered |

## Unresolved decisions / blockers

### User-facing product questions — recorded, not decided

| # | Question |
|---|---|
| **P1** | Does the canonical model carry a **stored** overall rating, a **computed** one, or both? Currently stored only. |
| **P2** | Are target-native extensions retained as canonical provenance and round-tripped? Q08a implies yes; `target-adapter/1` has **no syntax** for one. |
| **P3** | Is `assets-jerseys` the family the legacy `uniforms` rows belong to? (Same question as checkpoint 4's M1.) |
| **P4** | Do asset profiles need per-target overrides parallel to Q07a.3a's database target overrides? |

Each would change what the product does. None is answerable from approved guidance, so each is
recorded and the dependent branch stopped; all independent work continued.

### Format and coverage gaps

U-A1 – U-A7 in the contract document §5.3 — 26 units with no adapter, 13 families with no profile,
and four named format gaps (enum stored codes, null sentinels/pool behaviour, companion rules,
capacity).

## Prerequisites for the next checkpoint

None. Checkpoint 7 assembles and verifies; it needs a fresh isolated snapshot before any broad
command with side effects.
