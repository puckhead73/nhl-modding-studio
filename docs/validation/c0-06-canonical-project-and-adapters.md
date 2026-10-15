# C0.6/C0.7 — Canonical Project schema, target-adapter contract, and the completion mapping

> **Package:** C0-BULK1 checkpoint 6, fulfilling roadmap [C0](../plans/project-completion-roadmap.md)
> items **6** and **7**. **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
>
> **Amended 2026-09-07 by package C0-CAN1** ([receipt](../receipts/c0-can1-adapter-constraints.md)):
> the adapter format is now `target-adapter/2` and represents every **database** constraint Q07a.1
> names. §2 (rules), §4 (examples) and §5 (coverage) are updated below; §9 is new and is the
> representation-versus-population statement. The asset-profile half of item 7 is **unchanged**.
>
> **Amended again 2026-09-07 by review package C0-CAN1-R1**
> ([receipt](../receipts/c0-can1-r1-evidence-strength.md)): C0-CAN1 over-stated the evidence behind
> nine constraint blocks and one omission in its own examples. `asserted` can no longer make a
> constraint operative, a proposal now lives in a **candidate** channel that nothing reads, and a
> name search that finds nothing is a discovery note rather than a proof of absence. §2.1, §4.3 and
> §9.2 record it. **No representation was withdrawn** — the format still covers 20 of 20.
>
> **Amended a third time 2026-09-07 by review package C0-CAN1-R2**
> ([receipt](../receipts/c0-can1-r2-target-specific-derivations.md)): the architect's derivation rule
> — *derived means a conclusion supported by established premises applicable to this target; a
> cross-title analogy with no proven equivalence bridge is a candidate* — moved nine more blocks to
> candidate (both handedness enums and their transforms, both position discriminators, the ratings
> applicability predicates and both companion rules) and added `derivation-unexplained`, which
> requires every derived block to record its premises. §2.1, §4.4 and §9.2 record it. **No third
> confidence class was added and no representation was withdrawn** — still 20 of 20.
> **Amended a fourth time 2026-09-08 by package C0-AS1**
> ([receipt](../receipts/c0-as1-asset-profile-constraints.md)): the **asset** half of item 7, until
> now untouched. `asset-output-profile/2` replaces v1's seven free-text strings and one target-wide
> evidence label with **fifteen** evidence-qualified constraint blocks carrying the same three
> states, the same candidate channel and the same derivation rule as `target-adapter/2`, plus a
> tested v1→v2 transition that promotes no prose. Seven rules were added (46 → **53**). §1, §2.4,
> §5, §9.3 and the new §10 record it. **Re-authoring the one committed profile against the evidence
> rules moved every one of its facts to unknown or candidate: the asset registry now declares zero
> established constraints, which is the honest state and was previously hidden behind
> `qualification = "implemented"`.**
>
> **Amended a fifth time 2026-09-08 by package C0-AS3**
> ([receipt](../receipts/c0-as3-scoped-asset-variants.md)): the asset schema is now
> `asset-output-profile/3`. It adds the three things C0-POR1-R1 showed `/2` could not express
> without over-claiming — an explicit **evidence scope** and **constraint force** on every block
> bound to the inspected **corpus** it rests on; **correlated variants** with deterministic,
> structured selection; and **partially known values** whose unmeasurable sub-properties stay
> absent instead of being filled. Ten rules were added (53 → **63**), one constraint block
> (`runtime_lookup`, 15 → **16**), and a tested `/2`→`/3` transition that demotes every block. §1,
> §5, §10.1, §10.2 and the new §10.5 record it. **Nothing was measured**: this package re-represents
> the C0-POR1/R1 evidence at the scope that evidence actually has, and no target reaches a
> demonstrated target requirement.
>
> **Amended a sixth time 2026-09-08 by package C0-AS3-R1**
> ([receipt](../receipts/c0-as3-r1-source-backed-variants.md)): four corrections to what C0-AS3
> built, none of them a new measurement. **Variant selection is bound to the current source** —
> the caller supplies facts read off it, the override record's own selector is compared against
> those as an expectation, and a contradicted or unconfirmable expectation is refused; the
> approval fingerprint covers the observation, so version 1 approvals are invalidated rather than
> re-read. **Containers may differ per variant** where they genuinely do, shared at target level
> where they do not, and never decided at both. **An unattributable corpus is no longer evidence
> of nothing**: it may support a `specimen-observation` about its own files — established, and
> operative for nothing — while placement facts still need a `[corpus.bridge]`. And **observed
> bucket membership moved to the census**, referenced here by document and digest. One rule was
> added (63 → **64**), the population report now separates observed facts from target
> requirements, and the stale roadmap C0 item 7 row is corrected. §1, §5, §10.2 and the new §10.6
> record it.
>
> **Amended a seventh time 2026-09-08 by package C0-AS3-R2**
> ([receipt](../receipts/c0-as3-r2-observation-authority.md)): the **observation authority
> boundary**. A selector fact obtained by `operator-entered` is representable, keeps its provenance
> and **cannot decide a variant or reach an approved mapping** — variant resolution that would rely
> on one returns `BlockedByUnverifiedObservation`, at the step before approval is consulted, so
> approving it changes nothing. This is a rule about where a fact came from, not about who ran the
> tool: a parser reading a header at a person's request is still a reading, and stays operative.
> Codex has approved the fingerprint-v2 invalidation of all v1 approvals. Bridge documentation is
> corrected: validating one checks the **declaration**, and verifies neither the underlying evidence
> nor any runtime lookup precedence. No schema block, rule, profile or constraint changed. §2.4 and
> the new §10.7 record it.
>
> **Specification and validation only.** No Project UI, no converter and no Build engine is
> implemented, and nothing here promotes any support status.
>
> **The registry is not comprehensive, and this document says exactly how far it goes.** §5 is the
> coverage enumeration. Two evidence-backed adapters and one asset profile do not constitute a
> comprehensive registry, and nothing here claims they do.

---

## 1. The three document kinds

| Kind | File | Schema id | Validated by |
|---|---|---|---|
| Canonical Project schema | `defs/canonical/project-schema.toml` | `canonical-project/1` | `tdb_canonical::validate_project_schema` |
| Target adapter | `defs/adapters/<target>.toml` | `target-adapter/2` | `tdb_canonical::validate_adapter` |
| Asset output profile | `defs/adapters/asset-profiles/<family>.toml` | `asset-output-profile/3` | `tdb_canonical::validate_asset_profile` |

All three are versioned. An adapter declares `canonical_schema_version`; an adapter written against
an older canonical version keeps referring to it and is **reported** (`unsupported-version`), never
silently reinterpreted against the current one.

A `target-adapter/1` document is **not validated in place**: it does not even parse as v2, because v2
refuses unknown keys rather than dropping them. It reaches v2 only through the tested transition
`tdb_canonical::upgrade_v1`, which is deliberately conservative — every new constraint block comes out
`unknown`, and every v1 field with no v2 destination (the `"unknown"` identity placeholders,
`storage_bits`, `storage_range`, `human_range`) is returned in `Upgrade::notes` with the reason it
did not become a constraint. The transition promotes nothing; re-authoring against evidence does.
Both former v1 example files are kept verbatim at
`crates/tdb-canonical/tests/fixtures/v1/` and the transition is exercised over both.

The same discipline applies to asset profiles as of C0-AS1, and now applies twice: an
`asset-output-profile/1` document does not parse as `/3`, reaches it only through
`tdb_canonical::upgrade_asset_profile_v1`; an `asset-output-profile/2` document does not parse as
`/3` either, and reaches it only through `tdb_canonical::upgrade_asset_profile_v2`. Each refusal
names **its own** transition, because pointing at the wrong one would send an author through a
translation that cannot read their document. The preserved `/2` document is at
`crates/tdb-canonical/tests/fixtures/asset-profiles/v2/assets-portraits.toml`. The v1 file's
v1 bytes are preserved verbatim at
`crates/tdb-canonical/tests/fixtures/asset-profiles/v1/assets-portraits.toml`.

### 1.1 What lives where

The canonical schema is **target-neutral**. It contains no table id, no field id, no bit offset, no
endianness and no game name. Everything physical lives in an adapter. This is what makes
Q07a.1's "no game/platform database is the master representation" structural rather than aspirational:
there is no field in the canonical schema in which a target fact could be written.

Conversely an adapter can only **map** or **omit** a canonical property. It has no syntax for
redefining one, so a reconciliation correction cannot reach canonical data (Q07a.3a).

---

## 2. The contract rules

**Forty-six** named rules, all enforced by the validator and all covered by a positive and a
negative test. The first fifteen predate the constraint layer:

| Rule | Statement | Accepted decision it implements |
|---|---|---|
| `unsupported-schema` | the file's `schema` id must be the supported one | — |
| `unsupported-version` | an adapter's `canonical_schema_version` must match the schema's `version` | versioned adapters (Q07a) |
| `unknown-canonical-property` | every `canonical` path in a mapping *or* an omission must exist in the canonical schema | canonical is the single source of property identity |
| `duplicate-mapping` | one canonical property, at most one mapping per adapter | determinism of Build generation (Q07a) |
| `mapped-and-omitted` | a property is either mapped or omitted, never both | omissions are a distinct reported category (Q07a.3) |
| `inverse-write-missing` | a **non-identity** read transform with `write_transform = "unknown"` is refused | *"every read transform must have a verified inverse write transform"* (Q07a.1) |
| `inverse-unverified-but-qualified` | `automated-tested` requires `inverse_verified = true` | the same, at the evidence level |
| `qualified-without-case` | `automated-tested` requires a named `automated_case` | CLAUDE.md: automated-tested means *a named automated case actually executed* |
| `unknown-default` | `default = "unknown"` is refused | *"unknown target defaults block rather than silently substitute"*; **Clear to Defaults** may only use evidence-backed defaults (Q07a.3) |
| `missing-provenance` | a fact with `confidence != "unknown"` must cite evidence | *"facts carry fixture provenance, confidence and qualification"* (Q07a.1) |
| `unknown-confidence` / `unknown-qualification` | closed vocabularies | evidence rules |
| `adapter-claims-game-verified` | **no schema or profile file may declare `game-verified`** | CLAUDE.md: *documentation never promotes support*. Qualification comes from the evidence system alone |
| `malformed-target` | `target` is `<title>-<platform>` | joins the evidence contract's coverage keys with no translation |
| `unknown-identity-but-qualified` | a mapping with no established table or field identity cannot be `automated-tested`, even with a proven transform | a proven *transform* is not a proven *mapping* |

The remaining twenty-six arrived with `target-adapter/2` (package C0-CAN1) and govern the constraint
layer. Grouped by what they protect:

| Group | Rules | What they prevent |
|---|---|---|
| **Nothing silently ignored** | *(parse-level)* every struct is `deny_unknown_fields` | a misspelled `provenence` or `bit_ofset` is a **parse error**, not a dropped fact |
| **Placeholders are not values** | `placeholder-as-value` | `"unknown"`, `"tbd"`, `""` used where an identity belongs. v1 spelled an unestablished identity as the literal string `unknown`; v2 spells it as an **absent key**, so a placeholder can never be mistaken for a fact |
| **The three states stay distinct** | `established-without-value`, `established-without-confidence`, `value-without-established-state`, `not-applicable-without-reason` | an established fact with no payload, no confidence or no provenance; a value smuggled in under `unknown`; an absence claimed with no account of how it was determined |
| **Closed vocabularies** | `unknown-vocabulary-term` | free text where a term is required — storage kind, signedness, endianness, transform kind, sentinel encoding, string storage/encoding/padding/overflow, FK declaration, applicability kind, capacity source, cardinality, `applies_when`, omission reason |
| **Internal references resolve** | `undeclared-table`, `duplicate-table-declaration` | a `table`, FK parent, capacity subject, companion or discriminator naming a table the adapter never declared, so a typo cannot invent one |
| **Bounds and relationships hold** | `storage-bounds-inconsistent`, `enum-code-out-of-range`, `string-capacity-conflict`, `capacity-below-population`, `sentinel-conflict`, `transform-declaration-conflict` | a range that does not fit its width; inverted bounds; a field endianness contradicting the adapter's; an unsigned field with a negative range; an enum code the field cannot store; a byte budget contradicting the storage width; a ceiling below the population it must hold; a sentinel contradicting its own encoding, its storage regime or an enum code; a structured transform that does not spell the `read_transform`/`write_transform` it sits beside |
| **No duplicate or conflicting declarations** | `enum-code-conflict`, `duplicate-capacity-declaration`, `duplicate-companion-rule`, `duplicate-companion-table`, `position-code-conflict`, `composite-identity-conflict` | two meanings for one stored code, two ceilings for one table, a companion listed twice, a position code that is both skater and goalie, a mapping that is both single-field and composite |
| **Constraints fit their property** | `constraint-kind-mismatch`, `enum-value-not-in-domain` | an enumeration on a non-enum property, string storage on a non-string, a foreign key on a non-reference, or an enum entry naming a value the canonical domain does not define |
| **Rules stay actionable** | `position-discriminator-missing`, `applicability-incomplete`, `default-on-non-editable-field`, `omission-without-proof` | a `position-*` companion rule with nothing establishing how position is decided; a predicate with no operands or no codes; a Clear-to-Defaults value offered for a key or non-updatable field; a `target-absent` omission asserting proof of absence without citing it |

### 2.1 Evidence strength (C0-CAN1-R1)

Four further rules separate what a document may *act on* from what it may merely *record*:

| Rule | Statement |
|---|---|
| `asserted-as-operative` | an `established` constraint must be `measured` or `derived`. `asserted` — a documented expectation, a name-based inference, a fact confirmed on a neighbouring title — cannot make a constraint operative |
| `candidate-on-decided-constraint` | a `[<block>.candidate]` is legal only under `state = "unknown"`; proposing a value for something already decided is a contradiction |
| `candidate-incomplete` | a candidate must carry a value, a confidence and a citation. A proposal with no citation is not a proposal |
| `unresolved-transform-conflict` | `read_transform`/`write_transform` of `"unresolved"` must agree on both sides, and cannot coexist with a verified inverse, an `automated-tested` qualification, a `default`, or an established `[mapping.transform]` |
| `derivation-unexplained` | an `established` block at `confidence = "derived"` must record a `derivation`: the premises, why each is applicable to **this target**, and what the argument does not establish. A confidence label and a citation string are not an argument |

**The candidate channel.** An unknown constraint may carry `[<block>.candidate]` with a proposed
value, its confidence and its provenance. Nothing reads it: `Constraint::established()` — the single
accessor every derivation, default and export decision goes through — cannot see a candidate, and
`candidate_value()` exists only for reporting to a human. A proposed index origin therefore cannot
reach a human range, a Clear-to-Defaults value or a Build mapping.

`"unresolved"` on the transform names is distinct from `"unknown"` on the write side. `"unknown"`
still means *this read has no declared inverse* and is still refused; `"unresolved"` claims nothing
in either direction, which is what a mapping needs when its field identity is measured but its
encoding is not.

### 2.2 Derived, never declared

Two facts the format deliberately refuses to let an author state, because stating them creates a
second place to be wrong:

* the **human range** — computed by `tdb_canonical::human_range` from the structured transform and
  the storage range (`0..=63` plus the proven `+36` regime is `36..=99`). v1 declared `human_range`
  beside `storage_range`; the transition reports that field as having no v2 destination;
* **scalar editability** — `updatable && !key`, the rule the scalar mutation audit proved and the
  presentation layer already applies, computed by `tdb_canonical::scalar_editable`.

### 2.3 The last rule caught a real over-claim in this package's own examples

The first draft of `defs/adapters/nhl15-xbox360.toml` mapped `player.rating_overall` at
`qualification = "automated-tested"`, citing the proven ratings write codec — while its table and
field ids were the literal `unknown`. The validator refused it. **The data was corrected, not the
rule**: the transform is proven and stays recorded as such; the mapping is now `implemented`, because
nothing in the repository says which field carries the stored overall on that target. That is the
distinction the rule exists to enforce, and it fired on its author.

### 2.4 The asset constraint layer (C0-AS1)

Eighteen further rules govern `asset-output-profile/3` — the seven C0-AS1 added, ten from
C0-AS3, and `asset-provenance-bridge-invalid` from C0-AS3-R1: `asset-evidence-scope-missing`, `asset-evidence-scope-insufficient`,
`asset-corpus-incomplete`, `asset-corpus-undeclared`, `asset-corpus-attribution-unsupported`,
`asset-variant-selector-incomplete`, `asset-variant-selector-contradiction`,
`asset-variant-block-conflict`, `asset-property-evidence-conflict`,
`asset-path-family-unresolved` and `asset-provenance-bridge-invalid`. The envelope rules of §2 and §2.1 —
`established-without-value`, `value-without-established-state`, `not-applicable-without-reason`,
`missing-provenance`, `asserted-as-operative`, `candidate-on-decided-constraint`,
`candidate-incomplete`, `derivation-unexplained`, `placeholder-as-value`, `unknown-vocabulary-term`,
`constraint-kind-mismatch`, `malformed-target`, `adapter-claims-game-verified` — are **reused
verbatim**, because the evidence discipline is the same discipline.

| Rule | Statement |
|---|---|
| `unknown-asset-family` | a profile's `family` must be one of the fourteen QA-matrix families. A test asserts the crate's vocabulary still equals `tdb_qa::catalog::FAMILIES`, so a profile cannot register itself against a coverage axis the ledger does not have |
| `duplicate-target-profile` | one target, at most one profile per family |
| `duplicate-source-role` | two `[[canonical_source]]` entries cannot share a role |
| `asset-bounds-inconsistent` | numbers that contradict each other or another **established** block: a zero extent, a `power_of_two` claim that is not, block-compressed dimensions that are not whole blocks, a mip count a surface cannot have, an atlas cell outside its page, a vertex attribute past its stride, a palette larger than its rig, a channel set contradicting its alpha mode, an install kind contradicting its size policy |
| `asset-operands-incomplete` | a constraint missing what its own kind requires: an atlas with no cell, a pool with no slot, `fixed-count` with no count, a container chain with no layers, a path template and token list that disagree in either direction, an operation requiring nothing |
| `asset-constraint-dependency-unmet` | an established block resting on one that is **not** established: an archive install rule with no container, a slug path with no `player-slug` identity, more UV sets than the vertex layout carries, more skin weights than it stores |
| `duplicate-operation-declaration` | one operation, at most one requirement set per target |

**Class applicability is enforced, and it distinguishes the two absences.** A profile declares its
canonical sources (`raster`, `mesh`, `audio`), and a block belonging to an undeclared class is
`constraint-kind-mismatch` **whichever state it carries** — a portrait family does not have a
`not-applicable` rig, it has no rig question. Within a declared class, `unknown` and
`not-applicable` remain the distinct things §2 makes them.

**Format capability is kept apart from converter capability, structurally.** A v2 target profile has
**no key** in which a support level, an implementation status or an operation result could be
written: v1's per-target `qualification` and `confidence` simply do not exist, and `deny_unknown_fields`
makes writing one a **parse error**, which a test asserts. What a profile may say is which format
constraints an operation would need (`[[target_profile.operation]].requires`), and
`tdb_canonical::operation_constraint_readiness` reports which of those are unknown. Its two variants
are `ConstraintsEstablished` and `BlockedByUnknownConstraints`; **neither is about Studio**, and a
test asserts the type never renders as a capability claim.

---

## 3. Preserved principles

| Principle | Where it holds |
|---|---|
| Canonical data retains the richest known semantics | canonical ratings are the human 0..99 scale and canonical dates are 1-indexed; every target's narrower storage is an adapter transform, so exporting to a poorer target cannot erase Project data |
| Standard mode remains a direct workflow | nothing in this specification is on the Standard path. Standard mode opens a target-native file and edits it; no canonical Project, adapter or profile is consulted |
| Projects/Builds belong to opt-in Developer Mode | these documents are Developer Mode inputs only |
| Reconciliation corrections affect target Build mappings, not canonical data | structurally impossible to violate — an adapter has no syntax that writes a canonical property |
| Omissions/loss require explicit approval | `[[omission]]` carries a reason and is data for the Q07a reconciliation report; the validator refuses an omission of a property the schema does not define, so an omission cannot be used to hide a typo |
| Unknown target defaults block rather than silently substitute | `unknown-default` |
| Inverse-write transforms require evidence | `inverse-write-missing`, `inverse-unverified-but-qualified` |
| Schema/profile facts cite provenance and qualification | `missing-provenance`, closed vocabularies, and `game-verified` structurally absent |

---

## 4. Worked evidence-backed examples

Two adapters exist, chosen to demonstrate the one thing a registry must get right.

| Adapter | Fact | Evidence |
|---|---|---|
| `nhl15-xbox360` | `player.birth_date` — month/day **0-indexed**, so canonical (1-indexed) maps `+1` on read and `-1` on write | `crates/studio-domain/tests/birthdate_encoding.rs` |
| `nhl10-xbox360` | `player.birth_date` — month/day **1-indexed** on the same platform and generation, so the transform is **identity** | the same case, verified against all committed anchors |

EA's metadata is byte-identical either way (`rangelow="0" rangehigh="15"`), so the convention is
decidable only from stored values and is **probed**. Copying NHL 15's transform to NHL 10 would move
every player's birthday by a day and a month, silently, in the display *and* the edit form. This is
`R-ADJACENT` in practice: **an adjacent title never inherits a neighbour's fact**, and the test
`an_adjacent_title_does_not_inherit_its_neighbours_transform` asserts the two adapters disagree.

### 4.1 Four divergences, on the same platform and generation (C0-CAN1)

Re-authoring both examples against `target-adapter/2` turned one demonstrated divergence into four,
each read out of committed evidence, and the test
`an_adjacent_title_does_not_inherit_its_neighbours_transform` now asserts all four:

> **Corrected by C0-CAN1-R1.** Two of the four axes originally listed here rested on claims the
> repository does not prove — the main-team index origin (a runtime-probed expectation) and the
> presence or absence of a null sentinel on that field (an inference from a declared range). Both are
> now candidates and neither is operative; §4.3 records the correction. The table below lists only
> **measured** divergences, and the test asserts only those.

| # | Axis | NHL 15 Xbox 360 | NHL 10 Xbox 360 | Evidence |
|---|---|---|---|---|
| 1 | birth month/day index origin | 0-indexed (`index-origin-0`) | 1-indexed (`identity`) | `crates/studio-domain/tests/birthdate_encoding.rs`, a named case over the committed anchors |
| 2 | `ubPc.dXSB` declared range and signedness | `-1..254`, `neg1-sentinel` (the all-ones pattern decodes to -1) | `0..255`, plain unsigned | the two schema snapshots' `range_low` / `range_high` |
| 3 | the field id carrying the same canonical rating | `dSvy.AdEb` (`c_speed`) | `dSvy.qPWY` (`p_speed`) | the two snapshots; the `p_*` → `c_*` block renaming |
| 4 | name **byte budget** | 25 bytes (200 bits) | 40 bytes (320 bits) | the two snapshots' `ubPc.HdeP` |

Axis 2 is worth reading precisely, because it is exactly the distinction C0-CAN1-R1 turned on: the
snapshots measure how the eight bits **decode**, and that genuinely differs between the two titles.
They do not establish what any decoded value **means** — whether -1 is "no team" on NHL 15, or
whether NHL 10 can express "no team" at all. That question is open on both targets.

### 4.2 Corrections the evidence forced

Three claims in the v1 examples did not survive being checked against the repository:

| v1 claim | Correction | Evidence |
|---|---|---|
| `player.first_name` used `read_transform = "string-pool"` | **Wrong.** `ubPc.HdeP` is `kind_code = 0` / `meta_type = "string"`, a byte-aligned fixed-width field, and `tdb_core::set_inline_str` implements exactly that: NUL-filled, byte-budgeted, over-length **rejected**. There is no pool. Now `inline-string` with a full `[mapping.string_storage]` block | schema snapshots; `crates/tdb-core/src/lib.rs`; `docs/audits/scalar-mutation-audit.md` |
| `player.rating_overall` and `player.rating_speed` carried `table = "unknown"`, `field = "unknown"` (and NHL 15 omitted speed as `unknown-field-identity`) | **Resolvable from evidence already in the repository.** The committed schema snapshot names `dSvy.eWUF` = `c_overall` and `dSvy.AdEb` = `c_speed`. Both identities are now established; the mappings stay `implemented` rather than `automated-tested`, because the named case proves the *codec*, not this field | `docs/audits/data/schema/xbox360-nhl15.json` |
| NHL 10 mapped `player.rating_overall` with an unknown identity | **The property is target-ABSENT, not unknown.** No field in NHL 10's snapshot has a name containing `overall` — 0 hits across 86 tables / 1769 fields, against 15 in NHL 15's. NHL 10's skater block is the earlier `p_*` set. It is now an `[[omission]]` with `reason = "target-absent"` **and its proof**, which is a different Build behaviour from an unknown: an absence is reported for approval, an unknown blocks | `docs/audits/data/schema/xbox360-nhl10.json` |

The third correction is the one the constraint layer exists for. v1 had no way to say "the target does
not have this", so a proven absence and an unexamined gap were spelled identically.

### 4.3 What C0-CAN1-R1 corrected

C0-CAN1's own examples over-stated their evidence in three ways. All three are fixed in the adapters,
each with the correction written into the file beside the block it applies to.

| # | Over-claim | Correction |
|---|---|---|
| 1 | NHL 10's `player.rating_overall` was declared `target-absent` — a claim the target has no stored overall — on the strength of a metadata **name search** finding 0 hits across 86 tables / 1769 fields | A search that finds nothing establishes that **the search found no named match**, not semantic absence. EA metadata names are not a reliable index of meaning — this adapter pair turns on `p_*`/`c_*` name drift between adjacent titles. Now `unknown-field-identity`, with the search retained as an explicit **discovery note** recording where someone already looked |
| 2 | Five `[mapping.null_sentinel]` blocks declared at `measured` on "no spare code" arguments | A declared range does not prove that every code in it carries an ordinary value. All five are `unknown` with the reasoning kept as candidates. The `ubPc.dXSB` case is the instructive one: the codec taxonomy classes `range_low == -1` as "the nullable encoding", but `capabilities.rs` documents `0` — not -1 — as the free-agent marker under the origin-1 encoding. Two contested readings, so neither is operative |
| 3 | Four blocks established at `asserted` — the `ubPc.dXSB` index origin and its inferred foreign key on both targets — plus operative `read_transform` names that spelled the proposed origin | `asserted` can no longer make a constraint operative (§2.1). These are candidates; the transform names are `unresolved` and the mappings are `qualification = "blocked"` with the blocker recorded |

A citation audit over every populated block in both adapters also corrected fifteen confidences that
were higher than their source supported. The most substantive: **NHL 10's companion rule** was
`measured` citing `roster-relationships.md`, but that audit states its own range — *VERIFIED against
nhl11 → legacy* — and NHL 10 is outside it. The mechanism is still supported by NHL 10's own snapshot
(`wBIz` is the declared key of `ubPc` and of each companion table), so the rule is now `derived`, and
says what would raise it. The full retained/downgraded table is in
[the R1 receipt §4](../receipts/c0-can1-r1-evidence-strength.md).

**Nothing supported was deleted.** Bit layouts, capacities, the `+36` rating regime, both birth-date
origins, string budgets and the companion mechanism all remain operative, and the human range still
derives from a measured storage range plus a proven transform.

### 4.4 What C0-CAN1-R2 corrected

R1 left several conclusions operative at `derived` on the strength of a citation anchored to another
title. The architect's rule settles when that is legitimate:

> **Derived** means a conclusion supported by established premises **applicable to this target**. A
> cross-title analogy with no proven equivalence bridge is a **candidate**, even when the layouts or
> field names match.

The operational test that follows is whether the cited source **says who it speaks for**:

| Source | Says | Verdict |
|---|---|---|
| ratings-write-codec audit | scopes its 6-bit/offset-36 row to `NHL 10 → Legacy / 11-21 (PS3/X360)` **by name**, and the offset is resolved by a per-database probe rather than a per-title constant | established premise **here** — retained derived |
| anchors manifest | `PS3 = Xbox360 databases are byte-identical` per game, **verified for all of NHL 10-15 + Legacy** | a proven equivalence bridge — capacities retained derived |
| scalar mutation audit | its matrix carries a row for **this** title and platform | established premise here — status blocks retained derived |
| `defs/enums/handedness.toml` | a 22-player audit of NHL Legacy `default.db`; **no wider scope claimed** | analogy — **candidate** |
| `defs/enums/position.toml` | a five-player audit of the same file; no wider scope claimed | analogy — **candidate** |
| roster-relationships audit | `VERIFIED against nhl11 → legacy` | established for NHL 15; **silent** on NHL 10 |

Nine blocks moved to candidate: both handedness enumerations and their `enum-code` transforms, both
position discriminators, the three ratings applicability predicates, and both companion rules. Two
operative surfaces moved with them — `player.shoots` is now `unresolved` / `blocked` on both targets,
because leaving `read_transform = "enum-code"` while the vocabulary became a candidate would have
changed nothing.

**The dependency was propagated, not papered over.** When the discriminators became candidates, the
pre-existing `position-discriminator-missing` rule immediately refused **NHL 15's** companion rule —
the one whose companion evidence is strongest, since the audit's range covers it. That refusal is
correct and the data was changed to match: a position-conditioned rule that cannot be evaluated
cannot be operative. No rule was relaxed to keep an example passing.

**The two companion rules are candidates for different reasons, and each says which.** NHL 15's is
`derived` and blocked on the discriminator alone — it promotes the moment that does, with no further
companion evidence. NHL 10's is `asserted`: the audit's stated range excludes the title, and the
shared-key declarations R1 leaned on establish neither cardinality, nor mandatory presence, nor the
skater/goalie condition.

**Independently supported physical facts were preserved separately.** `ubPc.GRkp` remains measured at
1 bit with range 0..1; `ubPc.vjla`'s 3-bit width and 0..7 range are recorded inside the
discriminator's candidate. Only the meaning of the codes was withdrawn.

### 4.5 The synthetic fixture

`crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` exercises the representations for which
**no real fact is established in this repository**: a pooled string with `truncate` overflow,
two's-complement and offset-binary signedness, a `literal-code` sentinel, a canonical-value
applicability predicate, a metadata-declared foreign key, a one-to-many companion, a
`main-db-max-records` capacity source and the `not-applicable` state. Every value in it is invented
and labelled so.

It carries `synthetic = true`, its target is `fixture-synthetic` (not a catalog unit), it lives
outside `defs/adapters/`, and `tdb_canonical::registry_population` **excludes** synthetic adapters. A
test asserts no file under `defs/adapters/` carries the flag. Exercising a representation therefore
cannot inflate coverage by a single row.

C0-AS1 adds the asset equivalent, `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml`,
under exactly the same rules. It exercises what no real portrait evidence can: mesh vertex layouts,
UV conventions, rigs and palettes, audio bank/sample constraints, atlases **and** pools, a
`fixed-count` mip policy, tiling alignment, the `not-applicable` state with its reason, and a
candidate on an asset block. Its targets are `fixtureone-synthetic` / `fixturetwo-synthetic`, and
`asset_registry_population` excludes it.

---

## 5. Registry coverage — enumerated, not claimed

### 5.1 The physical layer that already exists

The 27 committed schema snapshots are a genuine **physical** registry:

| Measure | Value |
|---|---:|
| Snapshots | **27** |
| Tables | **3,121** |
| Fields | **69,737** |
| Fields per target | 1,547 – 3,519 |
| Endianness | 20 big, 7 little |
| TDB version | 6 across all 27 |

Per field they already carry: raw and canonical id, name, kind code, meta type, **bit offset**,
**bit width**, range low/high, key flag, updatable flag and FK parent. Per table: record length,
current and max records.

### 5.2 What the semantic layer adds, and how much of it exists

Q07a.1 requires, beyond the physical facts: semantic canonical property, signedness, enum vocabulary
**and stored code**, scale/offset/index origin, null sentinel, string encoding and pool behaviour,
companion/cardinality rules, capacity, presence/applicability, a **verified inverse write transform**,
and per-fact provenance/confidence/qualification.

| Layer | Coverage |
|---|---|
| Physical facts | **27 / 27 targets**, 69,737 fields — already generated |
| Semantic adapters | **2 / 28 catalog units** (7%) |
| Semantic mappings | **13 mappings + 1 omission** against 69,737 fields — **under 0.02%** |
| Mappings that reach `automated-tested` | **2** (each adapter's `player.birth_date`) |
| Established constraint blocks | **61** across the two adapters (31 derived, 30 measured), against **63** unknown, **20** of those carrying a non-operative candidate (see §9) |
| Asset output profiles | **1 / 14 families**; within it **3 / 28 target profiles** (C0-POR1 added `nhllegacy-ps3`). **7 established constraints**, all on that one target: 3 at target level and 4 inside one of its variants. 45 unknown target-level blocks across the three, 22 of them carrying a non-operative candidate. **Every established constraint is `observed-source-format` over an `inspected-corpus-invariant`** — none is a demonstrated target requirement, and none of them authorises refusing a Build anything. **Superseded denominator (C0-AS6, §11): only 7 of the 14 families are asset-bearing; the row reads 1 populated / 3 represented / 3 absent of 7 as of 2026-09-16** |

**The registry is therefore not comprehensive, and C0 item 7 is not complete.** As of C0-AS1 the
*format* half of the contract is complete on both sides: the canonical schema, `target-adapter/2`
representing every database constraint Q07a.1 names, `asset-output-profile/3` representing every
asset constraint it names, **sixty-four** rules, three validators, **three** tested schema
transitions (adapter v1→v2, asset v1→v3, asset v2→v3), worked evidence-backed examples and two
labelled synthetic fixtures. What remains is largely **population**: 26 of 28 units have no adapter,
13 of 14 families have no profile, and the one family that has a profile establishes seven
constraints on one of its three targets. §9 and §10 state representation and population apart on
each side.

**A number that has not moved, and must not be read as if it had.** No asset constraint anywhere
reaches `supported-target-constraint` scope. Everything established is what an inspected copy
*carries*, which authorises reproducing it and nothing else. Establishing what a target **requires**
needs a load, acceptance or shipping-manifest test, and none has been performed on any target.

As of C0-AS3-R1 that is a reported figure rather than a claim in prose:
`asset_registry_population` counts `operative_constraints` and `target_required_constraints` apart
from `established_constraints`, and a test pins the third at **0** across the registry. "Seven
established constraints" and "seven proven target requirements" are different sentences, and only
the first is true.

### 5.3 Enumerated unknowns

| # | Unknown | Class |
|---|---|---|
| U-A1 | 26 of 28 catalog units have **no adapter at all** | coverage |
| U-A2 | ~~13 of 14 asset families have **no output profile at all**~~ — **RESTATED by C0-AS6 (§11)**: the axis has 7 asset-bearing families and 7 database/save/onboarding workflow families that cannot carry a profile (refused by `asset-family-not-asset-bearing`). Of the 7: portraits populated; logos, jerseys and equipment (`9e02acf`) represented; heads, arenas, audio absent — heads awaiting the external conversion workstream, arenas stretch by decision, audio behind the C7 audit. C0-AS1 closed the *format* half; population stays deliberately evidence-bound: no profile is written for a family with no committed record behind any block | coverage |
| U-A3 | ~~Enum vocabularies and their stored codes are recorded nowhere in the semantic layer~~ — **CLOSED as a format gap by C0-CAN1**: `[mapping.enumeration]` records the vocabulary and its stored codes, validated against the canonical domain, against the field's storage range and against the null sentinel. The **evidence** caveat stands and is now expressed rather than hidden: `defs/enums/handedness.toml` and `position.toml` are anchored on NHL Legacy, so both adapters record those codes at confidence `derived`, never `measured`. Whether a label registry is a valid source for a stored code on *another* title remains an evidence question, not a format one | evidence |
| U-A4 | ~~Null sentinels, string-pool behaviour and companion/cardinality rules have no representation~~ — **CLOSED by C0-CAN1**: `[mapping.null_sentinel]`, `[mapping.string_storage]`, `[[companion_rule]]` + `[position_discriminator]`, plus `[mapping.status]`, `[mapping.storage]`, `[mapping.transform]`, `[mapping.foreign_key]`, `[mapping.applicability]` and `[[mapping.component]]` | format — closed |
| U-A5 | `legacyrecomp-recomp` has no anchor and therefore no physical snapshot to build an adapter over (register FR4) | fixture |
| U-A6 | ~~Capacity is in the snapshots as `max_records` per table, but per-title roster ceilings are not expressed~~ — **CLOSED by C0-CAN1**: `[[capacity]]` records ceiling, population and — load-bearingly — the `source` file, because on gen7 the main DB is packed and only the ROSTER save exposes the ceiling. Both adapters declare all seven roster-bearing tables at confidence `derived`, and register `kOtt` as `unknown` rather than reading it off the packed main DB | format — closed |
| U-A8 | **New (C0-CAN1).** Composite canonical values are representable (`[[mapping.component]]`, used by `player.birth_date` over three fields), but a composite mapping has no single `[mapping.storage]`, so a mapping-level bounds check does not apply to it. Per-component storage is validated instead | format — accepted design |
| U-A9 | **New (C0-CAN1).** `[mapping.applicability]` expresses *which records* a mapping applies to. It does **not** express a per-record write ordering or a cross-field consistency rule (for example "month and day must both be written or neither"). No accepted decision requires one yet | format |
| U-A7 | ~~Asset profile fields (`dimensions`, `mipmaps`, `raster_codec`, `atlas`) are `unknown` even for the one family with a profile~~ — ~~**WIDENED and made precise by C0-AS1**: all fifteen blocks on both targets~~ — **NARROWED by C0-POR1/R1 and C0-AS3.** Seven constraints are now established on `nhllegacy-ps3`. What has **not** narrowed, and is the more important half: every one of them is `observed-source-format` over one inspected corpus, so none says what the target requires. `mipmaps`, `tiling` and `path_rule` remain unknown on all three targets, and `runtime_lookup` is unknown everywhere | evidence |
| U-A10 | ~~**New (C0-AS1).** A v2 profile describes one (family, target, variant) triple…~~ — **CLOSED by C0-AS3**, and it was not hypothetical: NHL Legacy carries three portrait forms at once, two of them inside one folder, and `/2` could hold one. `[[target_profile.variant]]` holds them correlated, with deterministic structured selection that refuses an ambiguous query rather than preferring a form | format |
| U-A12 | **New (C0-AS3).** `PATH_TOKEN` still has no token for a directory **computed** from an asset id. `[[path_family]]` can record that such a directory exists and which ids were observed in each, but if a real id-to-bucket rule is ever established there is nowhere to write it, and `bucket_resolution = "derived-directory"` is refused for exactly that reason. This is a format gap, not an evidence gap — no such rule is known for any portrait target | format |
| U-A13 | **New (C0-AS3).** No asset constraint anywhere reaches `supported-target-constraint` scope, so no profile can carry `force = "target-required"` and no Build can be **refused** a value on the strength of a real target. The refusal path exists and is tested, but only against the labelled synthetic fixture. Closing this needs a load, acceptance or shipping-manifest test, which is a separate authorized step | evidence |
| U-A11 | **New (C0-AS1).** `[[target_profile.operation]].requires` states which format constraints an operation needs. It does **not** express a per-operation *tolerance* (the visual tolerance C3 item 5 asks for, or C4's rasterization tolerance for nameplates), because no accepted decision fixes what those tolerances are | format |

None of these is filled with a default. Every one is registered.

---

## 6. C0 item 6 — completion mapping

Roadmap C0 item 6: *"Map every completion item to a milestone, evidence cell/case, accountable
implementer or manual tester, and unresolved decision."*

The evidence-cell half is now **mechanised rather than tabulated**: the R2 evidence contract's
declared universe emits **2,016 rows** (28 units × 14 families × their non-conversion operations),
each already carrying its applicability, its required/optional status and its `unresolved` tokens.
That is the map, and it regenerates instead of drifting. The table below maps the *milestones* onto
it and records what the mechanised map cannot supply.

| Milestone | Families it owns (evidence keys) | Test route today | Owner |
|---|---|---|---|
| C1 database correctness/parity | `db-presentation/*`, `db-operations/*` | `studio_domain::{read,mutation,operation}_matrix` over 27 anchors | **owner-unassigned** |
| C2 saves and runtime writes | `save-raw-db/*`, `save-inner-container/*`, `save-outer-package/*` | `studio_domain::save_matrix` (26 of 27 units); **outer packaging has no route at all** | **owner-unassigned** |
| C3 / C3a shared asset writes, Ignite archives | `assets-*` `replace-modified`, `rebuild` | `tdb-archive-big`, `tdb-archive-codec`; 7 cases mounted-gated | **owner-unassigned** |
| C4 2D asset workflows | `assets-portraits`, `assets-logos`, `assets-jerseys` | `tdb-rx2` — **26 fixture-dependent cases not run** | **owner-unassigned** |
| C5 mesh/appearance/headmesh | `assets-heads` (14 conversion pairs, 8 destinations) | `tdb-headmesh` `matrix_guard`, `cc5_*`; pair-specific keys all `unqualified` | **owner-unassigned** |
| C6 schedules and localization | `onboarding-localization/*` | discovery only | **owner-unassigned** |
| C7 audio | `assets-audio/*` | **none — unimplemented** | **owner-unassigned** |
| C8 historical/Legends/shell | not an evidence family — Project-level | `tdb-fhm*`, `studio_domain` import cases (corpus-gated) | **owner-unassigned** |
| C9 qualification campaign | every `game-acceptance` key | **none can qualify**: every runtime build identity is `unknown` (register FR6), so R-IDENTITY withholds | **owner-unassigned** |
| C10 release engineering | — | CI exists; Windows CI open (C0.2 R6) | **owner-unassigned** |

**Ownership is the unmet half of C0's exit criterion.** Every completion item now has a milestone, an
evidence cell and either a test route or an explicit blocker. **No item has an accountable owner**,
and assigning owners is an architect action, not a developer inference. This is recorded here and in
[the fixture/runtime register §8](c0-05-fixture-runtime-register.md).

---

## 7. Questions that change user-facing functionality — recorded, not decided

Per the package's limits, each of these would change what the product does, and none is answered
from approved guidance. Independent work continued around them.

| # | Question | Why it cannot be answered here |
|---|---|---|
| **P1** | Does the canonical Project model store a **stored** overall rating, a **computed** one, or both? The schema currently declares `rating_overall` as the stored value only, with the computed estimate deliberately absent. C1 item 2 requires distinguishing them in the UI; whether the *canonical model* carries both is a product decision. | affects what a Project holds and what a Build writes |
| **P2** | When a target has a field but the canonical model does not (a target-native extension), does the Project **retain** it as provenance, and is it round-tripped on export? Q08a says promotion retains "target-specific values as provenance/extensions", but `target-adapter/1` has **no syntax** for a target-native extension today. | affects whether a Standard→Project promotion is lossless |
| **P3** | Is `assets-jerseys` the coverage family the legacy `uniforms` asset rows belong to? Two migration claims are unmapped on exactly this (checkpoint 4, M1), and the answer decides which cell that evidence lands on. | a mapping is a claim about what evidence covers |
| **P4** | Does an asset output profile need a **per-target override** mechanism parallel to Q07a.3a's database target overrides, or are asset derivations always recomputed from the canonical source? | affects the reconciliation UI's shape. **Still open after C0-AS1**, which was scoped out of deciding it: `asset-output-profile/2` represents a target's **output constraints** and has no override syntax, so it neither presumes nor forecloses an override workflow |

---

## 8. What this checkpoint does **not** do

- It does not implement the Project UI, any converter, or the Build engine.
- It does not populate the registry for 27 targets, and §5 states the coverage numerically rather
  than implying completeness.
- It does not promote any support status; `game-verified` is structurally unrepresentable in these
  files.
- ~~It does not add enum stored codes, null sentinels, string-pool behaviour, companion/cardinality
  rules or capacity to the adapter format~~ — **added by C0-CAN1**; see §9.
- ~~It does not revisit `asset-output-profile/1`~~ — **replaced by C0-AS1**; see §10. C0-AS1 still
  implements no asset editor, no conversion engine, no archive write and no UI, adds no target
  population, and performs no fixture measurement.
- It decides none of P1–P4. C0-CAN1 did not decide them either: `[[mapping.component]]` was added
  because a *date* spans three target fields, which is a target-encoding fact and not an answer to
  P1's question about what the canonical model carries.

---

## 9. Representation completeness vs registry population (C0-CAN1)

These are two different numbers and this document keeps them apart. **Supporting a field in the
format does not populate it for 28 targets.**

### 9.1 Representation — a statement about the schema

Every database constraint Q07a.1 names, and where `target-adapter/2` represents it. The list is
`tdb_canonical::Q07A1_REPRESENTATION` and a test asserts it against the types:

| # | Q07a.1 constraint | Representation |
|---|---|---|
| 1 | table identity | `mapping.table` + a `[[table]]` declaration that every reference must resolve to |
| 2 | field identity | `mapping.field`, or `[[mapping.component]]` when one canonical value spans several fields |
| 3 | semantic canonical property | `mapping.canonical` |
| 4 | type | `[mapping.storage].value.kind` |
| 5 | endianness | `[mapping.storage].value.endian`, cross-checked against the adapter's |
| 6 | bit/byte offset | `[mapping.storage].value.bit_offset` |
| 7 | width | `[mapping.storage].value.bit_width` |
| 8 | signedness | `[mapping.storage].value.signedness` — the scalar audit's codec taxonomy |
| 9 | range | `[mapping.storage].value.range_low`/`range_high`; the **human** range is derived |
| 10 | enum vocabulary | `[mapping.enumeration].value.entry.canonical`, checked against the canonical domain |
| 11 | enum **stored code** | `[mapping.enumeration].value.entry.code` |
| 12 | scale / offset / index origin | `[mapping.transform].value.kind` / `offset` / `index_origin`, cross-checked against the transform names |
| 13 | null sentinel | `[mapping.null_sentinel]` |
| 14 | string encoding / pool behaviour | `[mapping.string_storage]` |
| 15 | update / key / structural status | `[mapping.status]` |
| 16 | foreign-key relationships | `[mapping.foreign_key]` |
| 17 | companion / cardinality rules | `[[companion_rule]]` + `[position_discriminator]` |
| 18 | capacity | `[[capacity]]` |
| 19 | presence / applicability | `[mapping.applicability]` and `[[omission]].reason` |
| 20 | verified inverse write transform | `write_transform` + `inverse_verified` + `automated_case` |

**20 of 20 represented.** This says nothing about any target.

### 9.2 Population — a statement about the registry

What the committed adapters actually declare, counted by
`tdb_canonical::registry_population` (synthetic fixtures excluded):

| Target | Mappings | Omissions | Established blocks | Unknown blocks | of which candidates |
|---|--:|--:|--:|--:|--:|
| `nhl15-xbox360` | 7 | 0 | 32 | 34 | 11 |
| `nhl10-xbox360` | 6 | 1 | 29 | 29 | 9 |
| the other **26 catalog units** | 0 | 0 | 0 | — | — |

Of the 61 established blocks, **31 are derived** — each recording its premises, their applicability
to that target and what the argument does not establish — and **30 are measured**. The trend across
the three packages (81 → 72 → 61 established) is the intended one: each review removed claims the
evidence did not carry, and none removed a claim it did.

`registry_population` reports `candidate_constraints` as a **separate** field, and a test asserts a
candidate never sits on anything but an unknown block. A proposal is not coverage; counting it beside
established blocks would restate the over-claim the channel exists to prevent.

Against a physical registry of **69,737 fields across 27 targets**, that is still **under 0.02 %** of
the semantic layer, and both adapters still carry more unknown blocks than a complete target would.
Populating the remaining 26 units is bounded implementation work this package does not attempt.

### 9.3 What is still open on item 7

| # | Open | Class |
|---|---|---|
| 1 | 26 of 28 catalog units have no adapter (U-A1) | population |
| 2 | ~~13 of 14 asset families have no output profile (U-A2), and `asset-output-profile/1` was **not** revisited — whether it needs an equivalent constraint layer is undecided~~ — the **format** half is **CLOSED by C0-AS1**: `asset-output-profile/2` represents 26 of 26 asset constraints (§10). The **population** half stands unchanged at 1 of 14 families, and P4 (asset overrides) remains undecided and was deliberately not decided | population |
| 3 | `legacyrecomp-recomp` has no anchor, so no snapshot to build an adapter over (U-A5) | fixture |
| 4 | Asset profile facts are established only as **observed source formats** on one target, and `mipmaps`, `tiling`, `path_rule` and `runtime_lookup` are unknown everywhere (U-A7, U-A13) | evidence |
| 5 | U-A8, U-A9 above | format |
| 6 | ~~Several retained facts are `derived` from a source whose stated anchor range does not cover the target~~ — **ANSWERED and CLOSED by C0-CAN1-R2**: such a fact is a candidate, and no third confidence class was added. `derivation-unexplained` now requires every derived block to state its premises and their applicability to the target | evidence — closed |
| 7 | **New (C0-CAN1-R2), CORRECTED by C0-AS1.** ~~Both targets' companion rules, both position applicability sets and both handedness mappings are blocked on **one** unproven fact: the position code assignment. A single named case probing `ubPc.vjla` against known players would promote roughly a dozen blocks.~~ That scope estimate over-stated what one measurement settles, on three counts: **position evidence does not establish handedness** (`ubPc.GRkp` is a different field with a different vocabulary); **NHL 10's companion semantics need separate evidence** (the audit's stated range excludes the title, so cardinality, presence and the skater/goalie condition are unproven there independently of the discriminator — only NHL 15's rule promotes with it); and **one anchor does not establish both targets** (the anchors manifest's equivalence bridge is per-game cross-platform, and carries nothing between NHL 10 and NHL 15). The honest scope: a probe on **one** target promotes that target's discriminator, that target's ratings applicability predicates, and — for NHL 15 only — its companion rule. It remains the right first measurement, and it is still a fixture measurement no package so far has been scoped to perform. Full correction: [the R2 receipt's appended correction](../receipts/c0-can1-r2-target-specific-derivations.md#correction--appended-2026-09-08-by-package-c0-as1) | fixture |
| 8 | **New (C0-AS1).** The asset registry declares **zero** established constraints. Its critical path is a *read* of one shipped portrait on one target — dimensions, pixel codec, mip layout, tiling and install location — which would promote most of one target profile at once. That too is a fixture measurement | fixture |

---

## 10. The asset half — representation vs population (C0-AS1)

The same two numbers as §9, kept equally far apart. **Supporting an asset constraint in the format
does not populate it for 28 targets × 14 families, and it never means Studio can perform the
conversion.**

### 10.1 Representation — a statement about the schema

Every asset constraint Q07a.1 names, and where `asset-output-profile/2` represents it. The list is
`tdb_canonical::Q07A1_ASSET_REPRESENTATION` and a test asserts each entry parses out of a document:

| # | Q07a.1 asset constraint | Representation |
|---|---|---|
| 1 | discovery rules | `[target_profile.discovery]` — scope, root, recursion |
| 2 | identity rules | `[target_profile.identity]` — key, source, and an **ambiguity policy** whose vocabulary has no `first-match`, because C4 forbids silently binding an ambiguous match |
| 3 | archives | `[container].value.layer.format`, one entry per nesting level, outermost first |
| 4 | indexes | `[container].value.layer.index`; only the innermost layer may be index-free |
| 5 | paths | `[[target_profile.path_family]].template` — several per target — plus `[path_rule].value.families`, which cannot be established while any named family's `bucket_resolution` is `unknown` |
| 6 | slugs | `[[path_family]].tokens = "player-slug"`, which cannot be established unless `[identity].key` is the slug |
| 7 | IDs | `[[path_family]].tokens = "numeric-id"` / `"team-id"`, with **observed** directory membership in `[[observed_bucket]]` kept apart from the authoritative rule in `bucket_resolution` |
| 8 | raster formats | `[raster].value.codec`, or `[[variant]].raster` where a target carries several correlated forms |
| 9 | dimensions | `[dimensions].value.width`/`height`, or `[[variant]].dimensions` correlated with that variant's codec, mip layout, tiling and path family — Q10's release constraint, so an unknown here blocks replacement |
| 10 | channels | `[raster].value.channels`, with `alpha` and `color_space` **optional** and their absence explained in `[raster.property.<name>]`; where a mode is recorded it is still cross-checked against the channel set and the codec |
| 11 | mipmaps | `[mipmaps].value.policy`/`count`/`filter`, cross-checked against the surface size |
| 12 | compression | `[compression].value.payload` — the **stream** wrapper, deliberately distinct from the pixel codec, plus whether re-encoding is deterministic (C3 item 5's byte-identity question) |
| 13 | swizzle/tiling | `[tiling].value.kind` + alignment |
| 14 | atlases/pools | `[atlas].value.sharing` — whether a surface is shared between assets, which is what counting payloads per container measures — with the atlas/pool `kind` and its page/cell/slot geometry **optional**, and the cell required to fit its page wherever a kind is given |
| 15 | mesh vertex layouts | `[mesh_layout].value.vertex_attribute[]` + stride, index format, winding, coordinate system, unit |
| 16 | UV conventions | `[uv].value.origin`/`wrap`/`channel_count`, bounded by the layout's UV streams |
| 17 | rigs/bones | `[rig].value.bone_count` |
| 18 | weights | `[rig].value.max_weights_per_vertex` + `weight_quantization`, bounded by the vertex's weight slots |
| 19 | palettes | `[rig].value.palette_size`, never larger than the rig |
| 20 | LODs | `[mesh_layout].value.lod_levels` |
| 21 | audio banks | `[audio].value.bank` |
| 22 | audio codecs | `[audio].value.codec` |
| 23 | audio sample constraints | `[audio].value.sample_rate_hz`/`channels`/`bit_depth`/`max_duration_ms`, with the depth owned by the codec |
| 24 | repack rules | `[install].value.repack` |
| 25 | install rules | `[install].value.kind`/`size_policy`/`backup`, with archive kinds requiring an established `[container]` and C3a's fit-versus-relocate distinction enforced |
| 26 | fixture provenance, confidence, qualification | every block is an `AssetConstraint<T>`: state, confidence, **evidence scope**, **constraint force**, the `[[corpus]]` it rests on, provenance, derivation, candidate |

**26 of 26 represented**, unchanged by `/3` — an evidence scope, a corpus record and a variant
selector are not among the 26 constraints Q07a.1 names, and adding them to that count would inflate a
figure that measures something else. This says nothing about any target, and nothing about any
converter.

### 10.2 Population — a statement about the registry

Counts are `tdb_canonical::asset_registry_population` over the committed profile; a test pins them.
"Target-level" counts the sixteen blocks a target carries directly; variant constraints are counted
apart, because they describe one of the target's forms rather than the target as a whole.

| Family | Target | Established | of which **operative** | of which **target-required** | Unknown | of which candidates | Not-applicable | Variants | Established in variants | Unknown sub-properties |
|---|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| `assets-portraits` | `nhllegacy-ps3` | **3** | 3 | **0** | 13 | 3 | 0 | 3 | **4** | 7 |
| `assets-portraits` | `nhllegacy-xbox360` | **0** | 0 | **0** | 16 | 13 | 0 | 3 | 0 | 0 |
| `assets-portraits` | `nhl21-ps4` | **0** | 0 | **0** | 16 | 6 | 0 | 1 | 0 | 0 |
| the other **13 families** | — | 0 | 0 | 0 | — | — | — | — | — | — |

The two middle columns are C0-AS3-R1's, and they are the ones to read. **Operative** is what a Build
may treat as a fact about the target; an established `specimen-observation` is not operative and is
counted apart (`specimen_observations`, **0** across the registry — no real profile records one).
**Target-required** is what may *refuse* a Build another value, and it is **0** everywhere, because
reaching it needs a load, acceptance or shipping-manifest test and none has been run.

**Seven established constraints, all on one target, and every one of them `observed-source-format`.**
`nhllegacy-ps3` establishes `container`, `compression` and `atlas` at target level, plus `raster` in
each of its three variants and `dimensions` in one of them. What earns the promotion in every case is
the same thing: the property holds across **every** inspected file of that title's own installed
copy, in both the current and the backup-labelled group — so it is not conditional on a group whose
label is itself an inference.

**What the other two targets show is that exhaustive measurement is not the question.** The Xbox 360
`_compiled` tree and the PS4 extraction were measured as thoroughly as the PS3 copy, and establish
nothing, because a derived or extracted tree is not attributable to the shipped title. `/3` enforces
that rather than narrating it: `[[corpus]]` records the attribution, an `extracted-tree` claiming to
be attributable is refused, and an established block resting on an unattributable corpus is refused
unless it is scoped to the specimens themselves.

That last clause is C0-AS3-R1's correction, and it matters for what these two targets could yet
record. AS3 refused every established block on an unattributable corpus, which is one step past the
finding: measuring an extraction establishes something true **about the extraction**. What it never
establishes is a fact about the target, and what it cannot establish at all — without the extraction
record that ties each file to its archive member — is where the file sat. Neither target's
measurements were re-scoped here: no promotion of any kind was authorised by this package, and both
still establish nothing.
Their measurements are kept — 13 and 6 non-operative candidates respectively — because demoting for
provenance must not throw a measurement away.

**Four things stay unknown for reasons worth naming, because each was a promotion `/2` invited.**

| Unknown | Why |
|---|---|
| `dimensions` on the two `playerheads` variants | Those variants are **selected by** their stored size, because 512x512 and 512x256 co-exist in one folder and nothing observed separates them by path. Establishing the size there would report the definition as the discovery — the circularity C0-POR1-R1 caught in `[mipmaps]` |
| `mipmaps` everywhere | Not invariant on the PS3 copy: 206 current files carry mip chains where the rest carry none, and nothing establishes who wrote which. C0-POR1 promoted `none` by measuring the subset it had already defined as carrying no chain |
| `tiling` everywhere | Three specimen decodes. `/3` refuses `specimen-observation` as a basis for an established block outright, so this is now a schema refusal rather than a review finding |
| `path_rule` everywhere | Both blockers are structural now. Every declared family carries `bucket_resolution = "unknown"`, and `[path_rule]` cannot be established through an unresolved family; and each Gen7 target declares **two** families, which `/2` could not hold at all |

**`runtime_lookup` is unknown on all three, and it blocks `discover` and `install` everywhere.** It
is the block for what the **game** loads, as distinct from where a scan finds files. C0-POR1
established `[discovery]` from observed placement in one installed copy; the reason R1 withdrew that
is `patch.big`, present at that copy's `USRDIR` root, never opened, and able to shadow every loose
portrait. `/2` had nowhere to put that question, so the demotion had to live in a note.

**Partial values, and the seven sub-properties that stay unknown.** Each PS3 variant establishes
`raster` on its codec and channel set while leaving `color_space` and `alpha` absent, with
`[raster.property.<name>]` recording why: the `DDS` header carries no colour space at all, and alpha
was observed on three decoded specimens. `[atlas]` establishes `sharing = "per-asset-surface"` — the
fact a payload count actually measures — and leaves `kind` unknown. Under `/2` each of those bundles
had to be complete or unknown, and C0-POR1 completed them with guesses.

Because operations name what they need (`raster.codec`, not `raster`), an unknown colour space blocks
`replace` and not `preview`. **Every (target, variant, operation) triple is still blocked**, and
`operation_constraint_readiness` names the specific requirement rather than reporting a single
failure.

### 10.3 The transition promoted nothing

`upgrade_asset_profile_v1` translates the committed v1 file into v2 with **every block unknown and
no candidate manufactured** — not even where a v1 string happened to spell a v2 vocabulary term. All
ten populated v1 fields are returned as notes naming the v2 block they *would* belong to, and the
per-target `qualification` is reported as having **no v2 destination at all**, because an
implementation level describes Studio and not the target format. Promoting any of those sentences is
re-authoring against evidence, which a human does with a citation.

### 10.4 What C0-AS1 did not do

No asset editor, no conversion engine, no archive write, no UI, no Build engine. No fixture was
measured or acquired, no evidence publication changed, and no additional target or family was
populated — fabricating a profile for a target with no evidence, or qualifying a family from one
tested variant, is precisely what these rules exist to prevent. **P1–P4 remain undecided**, P4
(asset overrides) explicitly so.

### 10.5 What C0-AS3 changed, and what it deliberately did not

**Changed.** `asset-output-profile/3`:

| Addition | What it closes |
|---|---|
| `scope` on every block, from [`EVIDENCE_SCOPE`] | `/2` had one axis — measured/derived/asserted — which says how well a thing was measured and nothing about which population the measurement covers. A `specimen-observation` can no longer establish anything |
| `force` on every established block | Separates *this is what the source carries* from *the target requires this*. Only the second refuses a Build another value; the first authorises reproducing the observed value and is otherwise **blocked**, because reading a copy's bytes proves neither that another format is forbidden nor that it is supported |
| `[[corpus]]` and the `corpus` binding | Evidence identity, kept apart from the facts read off it. A derived or extracted tree is `not-attributable` by construction, and an established block resting on one is refused |
| `[[variant]]` with structured selectors | One target, several correlated forms. Dimensions, codec, colour format, mip layout and tiling stay inside one variant, so no unobserved cross-pairing is spellable, and there is no per-target list of allowed values |
| Deterministic selection | Exactly one definite match selects; nothing matching is an explicit no-match; anything else — several matches, or one match beside a variant the query cannot decide — is a **refusal** naming them. Never first-match, never nearest |
| Optional sub-properties + `[<block>.property.<name>]` | An unmeasurable field no longer blocks the block it sits in, and no longer tempts an author to fill it. Operations name what they need |
| `[[path_family]]` + `[[observed_bucket]]` | Several path families per target, and observed directory membership kept explicitly apart from an authoritative id-to-directory rule |
| `[runtime_lookup]` | What the runtime loads and what a patch layer takes precedence over — the question observed placement was standing in for |
| `upgrade_asset_profile_v2` | A tested, conservative `/2`→`/3` transition: **every** block is demoted to unknown with its value preserved as a candidate, and no corpus, path family or variant is invented |

**Approval safety.** `profile_content_digest` now covers the corpora and their attribution, the path
families and their resolution state, every variant with its selectors and constraints, each block's
scope, force and corpus, and which sub-properties are unknown — so any change that alters what a
Build may do invalidates an approval. `override_fingerprint` covers the record's **variant selector**.
The exclusions are unchanged and still hold: prose and the non-operative candidate channel decide
nothing and must not invalidate an approval.

**A caller cannot select a convenient variant.** An override record carries the observable *facts*,
never a variant id — there is no key for one, and `deny_unknown_fields` makes adding one a parse
error. Selection is the profile's, run over those facts, and a selector that does not pin one variant
down is refused rather than resolved to the nearest form.

**Deliberately not done.** Nothing was measured. No game data was read, no archive investigated, no
`patch.big` opened, no runtime launched, no fixture acquired. No additional family or target was
populated. No constraint anywhere was promoted on new evidence: the seven established constraints are
C0-POR1-R1's three plus four the `/2` schema could not express, all resting on the same inspected
corpus. And no target reaches `supported-target-constraint`, so **nothing here claims build
provenance, installation compatibility or game acceptance.**

**The database half is untouched.** `/3`'s keys live on `AssetConstraint`, not on the shared
`Constraint<T>` the adapters use; a test asserts a committed `target-adapter/2` document still parses
and validates unchanged, and still refuses an asset evidence key as unknown.

### 10.6 What C0-AS3-R1 corrected

Four corrections to C0-AS3, and one accounting change. **Nothing was measured, nothing was promoted,
and no target's evidence moved.**

| Correction | What was wrong | What it is now |
|---|---|---|
| Selection reads the **source** | The resolver selected the variant from `proposal.variant_selector` — a field in a document somebody wrote. A record could reach another variant's constraints, and another variant's approval, by describing its source as that variant | `BuildContext` carries a `SourceObservation`: facts read off the current source, with the identity and digest of the source they were read from. Selection runs on those. The record's selector is compared against them as an **expectation**, and a contradicted or unconfirmable one is refused. No fact is ever inferred from the value being proposed |
| Containers may differ per variant | `container` was excluded from variants outright, on the strength of the portrait targets sharing one chain. That is a fact about those targets, not about the schema | A `VARIANT_OPTIONAL_BLOCK`: shared at target level *or* stated per variant, never both. The portrait profile is unchanged — its one chain stays where it was, stated once |
| An extraction is not evidence of nothing | Every established block on an unattributable corpus was refused, which discards real measurements of real files | A `specimen-observation` about the corpus's own files is admissible, **established and non-operative**. A `PLACEMENT_BLOCK` still needs a `[corpus.bridge]`, and a wider scope still needs attribution |
| Bucket membership belongs in the census | The profile listed observed id ranges per directory. A column of ranges reads as a lookup table however it is labelled, and the next reader resolves an id with it | `[[observed_bucket]]` records that the directory was seen and references the census by document **and digest**, with the scope of that observation. The ranges live in `c0-por1-portrait-specimens.json`, where they are labelled as an observation |

**The accounting change.** `asset_registry_population` reports `operative_constraints`,
`specimen_observations` and `target_required_constraints` beside `established_constraints`, and a
test pins the last at 0. "Seven established constraints" was true and read as "seven proven target
requirements"; the report now makes that reading impossible rather than warning against it.

**Approval safety.** `APPROVAL_FINGERPRINT_VERSION` is **2**: the construction covers the observed
facts and the identity of the source they were read from. Every version 1 approval is invalidated
rather than re-read, which is what the version constant exists to do — a v1 approval was bound to
what a record *claimed* about its source, and that binding is precisely the one this package
removed. The profile digest additionally covers `[corpus.bridge]`, per-variant containers and the
census reference behind each observed bucket.

**Migration.** The `/2` → `/3` transition still demotes everything. What changed is that a demoted
`not-applicable` now carries, in the **document**, what `/2` asserted, the reason it gave verbatim,
and a statement that the demotion is about missing scope rather than a finding that the assertion
was wrong. Demoting is not overruling, and a later author re-deciding the block should not have to
rediscover why somebody wrote it down.

**A correction to the record itself.** The C0-AS3 receipt reported the canonical test suite as
"126 → 183". 126 was the C0-AS1-era figure; the state AS3 actually started from was **162**, and
its own additions took it to 183. C0-AS3-R1 leaves it at **190**.

### 10.7 The observation authority boundary (C0-AS3-R2)

C0-AS3-R1 moved variant selection off what an override record *claims* about its source and onto
what was **read** from it. It then admitted `operator-entered` to the method vocabulary at full
standing, which reopens the same door one step further along: a typed-in width and a width read out
of a header are both strings in the same struct, and only one of them is a measurement.

| | Before (R1) | Now (R2) |
|---|---|---|
| `header-read`, `path-read`, `index-read` | select a variant | unchanged — `OPERATIVE_OBSERVATION_METHOD` |
| `operator-entered` | selected a variant, and could reach `ApprovedMapping` | representable, keeps facts and provenance, **decides nothing**: `BlockedByUnverifiedObservation` |

**Approval cannot elevate it**, and that is structural rather than a policy check: the gate is at
step 3 of `resolve_build_override`, and step 5 is where an approval is first read. The test starts
from an approved synthetic mapping built on a real reading, changes **only** the method, and
recomputes the fingerprint so the approval genuinely matches — the method is inside the v2
construction — and it stays blocked. Removing the approval altogether gives the same answer.

**What this is not.** It is not a rule about who ran the tool: a person who opens a file and has the
parser read its header has produced a `header-read`, and it is operative. Recording that as
`operator-entered` would be mislabelling it. It is also not a verification workflow — nothing here
defines how a hand-entered fact could be confirmed, and no such route exists in this crate.

**Fingerprint v2, confirmed.** Codex has approved invalidating every version 1 approval. The
principle is recorded here rather than left in a receipt question: a change to the fingerprint
construction **invalidates** an approval carrying another version, and never reinterprets it under
today's rules.

**Provenance bridges, stated accurately.** `validate_asset_profile` checks a bridge *declaration*:
a known `method`, an `original_corpus` that is a declared and genuinely attributable corpus on the
same target, a citation, and no self-bridge or bridge onto a corpus that already claims attribution.
It does **not** open or verify anything the citation points at, does not check that the extracted
files are the members a manifest names, and establishes nothing about **runtime lookup precedence** —
`[runtime_lookup]` is a separate block and a bridge does not touch it. Where a file sat inside an
archive and which copy the runtime prefers are different questions. Populating a real bridge
therefore requires separately reviewed evidence tying that specific source corpus to that specific
extracted material; **no real bridge is authorized, and none exists.**

## 11. Asset-family coverage, reconciled (C0-AS6, 2026-09-16, committed at `9e02acf`)

**The denominator.** `tdb_qa::catalog::FAMILIES` has fourteen entries, and every earlier figure in
this record ("1 / 14", "3 / 14", "13 of 14 have no profile") divided by fourteen. Seven of those
families are not asset families: `db-presentation` and `db-operations` (database workflows, C1),
`save-raw-db` / `save-inner-container` / `save-outer-package` (save-container workflows, C2) and
`onboarding-install` / `onboarding-localization` (C1/C6). They have no raster, mesh or audio to
profile; their registry object is the database adapter and the QA matrix row, and an
`asset-output-profile` filed under one of them is a category error. `tdb-canonical` now says this
structurally: `FAMILY_CATEGORY` classifies all fourteen (a test asserts it partitions the matrix
axis exactly), `Rule::AssetFamilyNotAssetBearing` (`asset-family-not-asset-bearing`) refuses a
profile whose family is a workflow category, and `asset_family_coverage` reports over the **seven
asset-bearing families** with a three-state status — `absent` / `represented` / `populated` — none
of which is support.

| Family | Status (2026-09-16) | Declared targets | By reference | Operative blocks |
|---|---|---|---|---|
| `assets-portraits` | **populated** | `nhllegacy-ps3`, `nhllegacy-xbox360`, `nhl21-ps4` | `legacyrecomp-recomp` | 7 (all on `nhllegacy-ps3`, all `observed-source-format`) |
| `assets-logos` | represented | `nhllegacy-xbox360` | `legacyrecomp-recomp` | 0 |
| `assets-jerseys` | represented | `nhllegacy-xbox360` | `legacyrecomp-recomp` | 0 |
| `assets-equipment` | represented (`9e02acf`, C0-AS6) | `nhllegacy-xbox360`, `nhllegacy-ps3` | `legacyrecomp-recomp` | 0 |
| `assets-heads` | absent — external conversion workstream; Studio integration need recorded in the receipt | — | — | — |
| `assets-arenas` | absent — stretch by roadmap decision (the synthetic fixture borrows the family and is excluded) | — | — | — |
| `assets-audio` | absent — C7 audit precedes any profile | — | — | — |

**What C0-AS6 measured (database side only), and what it did not.** On the committed anchors:
the two Legacy main DBs are one set of bytes; `ubPc.lOnr` stores 5 450 distinct portrait ids on
Legacy (180 zero rows, three ids on two players each, **350 ids above the highest id the C0-POR1
census observed in either Gen7 copy**) and 7 244 on NHL 21 PS4; the eight equipment id fields on
Legacy have widths 7/7/8/7/7/7/7/9 (= the HEAD `SLOT_SPECS` ceilings, which the cross-anchor record
shows to be Legacy widths carried as code) and store 19 / 14 / 10 / 8 / 29 / 26 / 29 / 173 distinct
ids, every skater id among those the committed enum catalogues name; and the 19 asset-referenced
fields on all 27 anchors are recorded in `docs/receipts/data/c0-as6-asset-db-references.csv`
(drift-guarded: `xjgw` absent before NHL 11, `kOtt.lOnr` absent on gen6, `fMVl.JOlt` absent on
NHL 08/09, the `kOtt` style references single-valued 0 wherever present). These are the **DB half**
of an identity. The file half — an id beside a hashed file in an attributable corpus — is in no
committed record for any family, so the portrait identities are **candidates** (measured,
specimen scope) and the equipment identity has **no candidate value** (one `[identity]` per target
cannot carry eight slot sources — a recorded schema gap, not extended because nothing an extension
could spell would be operative). **Nothing became operative; no operation is unblocked;
`target_required_constraints` stays 0.**

**The only in-game observation.** `defs/enums/stick-model.toml` records an operator's 2026-05-18
test on a PS3 Legacy copy: `xmja.wXFx = N` rendered `rendering/playerstick/texlib_N.rpsgl` on ice
for 74 ids. It is prose — no digests, no copy identity, the copy not pristine afterwards — and is
carried in the equipment profile as asserted candidates and a `runtime_lookup` note with no value.
It is also the nearest thing the repository holds to runtime evidence for any asset family, and
the receipt names what re-recording it would take.

Receipt: [c0-as6-asset-family-coverage.md](../receipts/c0-as6-asset-family-coverage.md). 539
canonical cases on the isolated candidate (532 at `cc913f9` + 7); clippy `-D warnings` and rustfmt
clean; no manifest or lock change. Committed as `9e02acf` (C0-AS6-CLOSE) after Codex review closed two
findings in place (three unspellable equipment templates moved to prose; the NHL 08 width summary
corrected to 8 bits for the stick, 4–5 for the other present slots); nothing pushed.

## 12. The file half, measured (C0-AS7, 2026-09-16, committed at `8d60a6d`)

**What.** The C0-AS6 §D batch D1 + D4 + D3 on the two copies C0-POR1 identified, read-only, through
the committed `tdb_archive_big::eb` reader (all three archives are the `0x00000400` 16-byte-entry
variant) and two new examples; records under `docs/receipts/data/c0-as7-*` with specimen
attribution, archive digests (cross-checked against `sha256sum`), tool provenance, and joins
re-derived by `tdb-canonical` cases on every run. Receipt:
[c0-as7-asset-evidence-measurements.md](../receipts/c0-as7-asset-evidence-measurements.md).

| Item | Measured | Carried as |
|---|---|---|
| D1 `cacherender.big` / `nocacherender.big` | 12 845 / 3 237 literal member names, offsets, sizes, stored digests; all under `rendering/`, all `chunklzx`; no `fe/` member. Joined to the anchor: jerseys 698 / 698 (bundle, nameplate, sock — socks in `cacherender.big`), equipment all eight measured fields total (74 stick texlibs = the enum), logos 180 / 218 under `texlib_{id}_0_0` (all 218 under `texlib_{id}_{a}_{b}`; 348 distinct, 130 unstored), and a ninth comparison — goalie-stick members vs the `xmja.wXFx` field HEAD source says the goalie stick shares — 1 / 19 (the shared-field claim contradicted) | `[[observed_bucket]]` entries (inspected-corpus-invariant, attributable corpus) on nine render families; discovery candidates `measured` / `inspected-corpus-invariant`, **still candidates** (one root per target; each family spans two archives + an unobserved place); identity notes |
| D4 PS3 portrait census | 18 663 files hashed per id; 4 500 / 4 501 of 5 449 anchor ids have a loose current file, 949 / 948 do not — 350 in 12403..12814 (outside the observed regular ranges), 599 / 598 inside 1..12401; 4 728 files name no row | `[identity]` candidate on `nhllegacy-ps3` keeps `unknown` **because** the join is not total; the PS3 observed buckets now cite the committed per-id census |
| D3 PS3 `patch.big` | 459 members: 83 portrait `BIGF`s at the loose tree's paths (23 collide, all with differing content; 33 anchor ids exist only here, 11 member ids exceed the anchor's maximum), 8 localization DBs, 328 `.rpsgl` (72 `chunkzip` inflate, **257 fail** in the committed codec — all multi-chunk), no stick member | `[runtime_lookup]` / `[discovery]` notes on `nhllegacy-ps3`; no value (precedence is unobserved) |

**Coverage after C0-AS7 (unchanged in kind):** still 1 populated / 3 represented / 3 absent of 7;
still 7 operative blocks, all on `nhllegacy-ps3`; every operation on every target still blocked;
`target_required_constraints` 0. 552 canonical cases (539 + 13) and 28 archive cases (23 + 5 P2 error-path cases) on the isolated
candidate; clippy and fmt clean on `tdb-canonical`; the new `tdb-archive-big` example lint- and format-clean while
the crate's baseline files keep the recorded C0.2 defects. Open for the architect: per-family
discovery (schema), the goalie-stick field, the non-total portrait binding, patch precedence, the
multi-chunk `chunkzip` defect (C3) and the uncommitted POR1 records (the 6.4 MB of records were
accepted at review and committed in full).

Committed as `8d60a6d` (C0-AS7-CLOSE) after Codex review passed with P2 (loose-file absence =
`NotFound` only) and P3 (summaries reconciled to the records) closed in place, on the isolated
validation above — canonical 552 / 0 / 0, archive 28 / 0 / 1 baseline ignore, scoped clippy and
fmt clean with the documented baseline exceptions preserved; no rerun, the committed bytes are the
validated bytes. Index empty afterwards; nothing pushed.
