# C0-AS1-R1 — The Q07a.3a override policy in `asset-output-profile/2`

> **Package:** C0-AS1-R1, resolving question **P4** from
> [c0-as1 §8](c0-as1-asset-profile-constraints.md#8-questions-for-the-architect). **Date:**
> 2026-09-08. **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No asset editor, no conversion engine, no archive write,
> no UI, no Build engine. **No fixture measured or acquired**, no evidence publication changed, no
> additional target or family populated, and **no constraint promoted anywhere**. Every fact in the
> committed portrait profile is exactly as C0-AS1 left it.

---

## 1. The decision

P4 asked whether asset profiles need per-target overrides parallel to **Q07a.3a** (*invalid-value
corrections apply only to the selected target's Build mapping; canonical values stay unchanged;
store the prior value, the resolution method and the schema/adapter provenance; revalidate or
invalidate when inputs change*).

**Answered: yes — mirror Q07a.3a.** This record implements that answer.

## 2. What was implemented, and the distinction it rests on

A Q07a.3a override has two separable halves, and only one of them is a specification:

| Half | Where it belongs | Implemented here |
|---|---|---|
| The **permission** — may a Build depart from this target's constraints at all, from which of them, by which methods, and what makes a departure stale | the profile: it is a per-target fact about the format's tolerances | **yes** |
| The **record** — this instance's corrected value, its prior value, its provenance, its validity | Build data (C8) | **no**, and it has no spelling here |

Keeping them apart is what stops a specification file from becoming a place corrections
accumulate. `OverridePolicy` carries **no key in which a corrected value could be written**, and
`deny_unknown_fields` makes an attempt to add one a **parse error** — the same mechanism that stops
a v2 profile recording converter support. A test asserts it for `value`, `prior_value`,
`corrections` and `instances`.

## 3. The block

`[target_profile.override_policy]`, one per target, on the same `deny_unknown_fields` /
closed-vocabulary discipline as the rest of the file. Four new closed vocabularies:

| Key | Vocabulary | Terms |
|---|---|---|
| `disposition` | `OVERRIDE_DISPOSITION` | `unspecified` (default), `recompute-only`, `overrides-allowed` |
| `scope` | `OVERRIDE_SCOPE` | `per-instance`, `per-target` |
| `resolutions` | `OVERRIDE_RESOLUTION` | `resample`, `recode`, `requantize`, `retopologize`, `truncate`, `substitute`, `manual-authored` |
| `invalidated_by` | `OVERRIDE_INVALIDATOR` | `canonical-source-changed`, `constraint-changed`, `profile-version-changed`, `converter-changed` |

plus `blocks` (ids from `ASSET_BLOCK`), `reason`, `requires_prior_value` and `note`.

### 3.1 Silence is not permission

The default is `unspecified` — no decision, and therefore **no override**, exactly as an absent
constraint block is neither a fact nor an absence. A target refuses overrides by recording
`recompute-only` **with a reason**; an empty reason is silence, which the default already expresses,
and is refused. A target permits them by recording `overrides-allowed` with its blocks, resolutions
and invalidators.

### 3.2 Three new rules

| Rule | Refuses |
|---|---|
| `asset-override-policy-incomplete` | `overrides-allowed` with no `blocks`, no `resolutions`, or an `invalidated_by` missing `canonical-source-changed` or `constraint-changed`; `recompute-only` with no reason |
| `asset-override-policy-conflict` | a disposition that permits nothing while still configuring overriding; `requires_prior_value = false` under a permissive policy; a `reason` on a permissive one; a resolution whose class this profile declares no source for |
| `asset-override-on-undecided-constraint` | a `blocks` entry naming a constraint that is not **established** |

The last is the substantive one. **An override is a departure from a known requirement.** There is
nothing to depart from in an `unknown` constraint and nothing to depart from in one the target has
decided it does not have, so naming either is refused rather than warned about. `requires_prior_value`
is checked rather than trusted for the same reason: Q07a.3a stores the prior value with every
correction, so a policy may not authorise a Build to discard it.

Existing rules do the rest of the work unchanged: `unknown-vocabulary-term` closes all four
vocabularies, `asset-bounds-inconsistent` catches repeated list entries, and
`constraint-kind-mismatch` catches a block belonging to a class the profile declares no source for.

## 4. What the block deliberately cannot do

| It cannot | Enforced by |
|---|---|
| carry a corrected value | no key exists; `deny_unknown_fields` makes one a parse error |
| unblock an operation | `operation_constraint_readiness` reads block states and cannot see the policy — a test asserts the most permissive policy expressible leaves a blocked export blocked |
| count as coverage | it carries no `ConstraintState`; `AssetPopulation` reports `override_disposition` and `overridable_blocks` **apart**, and the three constraint counts still sum to `ASSET_BLOCK.len()` |
| promote support | unchanged: there is still no key for a converter, a support level or an operation result |
| exist without an established constraint | `asset-override-on-undecided-constraint` |

The Q07a.1 representation table is **untouched at 26/26**: an override policy is Q07a.3a, not one
of the 26 asset constraints Q07a.1 names, and adding it there would have inflated a count that
measures something else. The profile still carries **fifteen** `Constraint<T>` blocks.

## 5. Population is unchanged: still zero

The committed `assets-portraits` profile establishes nothing, so **neither of its targets can name
a single overridable block** — the honest outcome, and the one the new rule forces. Both record
`disposition = "unspecified"` with a note saying to revisit once any block is established. A test
asserts exactly that, and asserts it *for the stated reason*, so a future promotion that leaves the
policy behind will fail rather than pass quietly.

The mechanics live where the rest of the unproven combinations live: the labelled synthetic fixture,
excluded from population. Its fully-populated target carries a permissive policy over three
established blocks; its second target carries a stated `recompute-only`.

| Metric | Before | After |
|---|---|---|
| Q07a.1 asset representation | 26/26 | 26/26 |
| `Constraint<T>` blocks per target | 15 | 15 |
| Established constraints, real profiles | 0 | **0** |
| Overridable blocks, real profiles | — | **0** |
| Rules | 53 | **56** |
| `tdb-canonical` tests | 126 | **136** |

## 6. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/asset.rs` | four vocabularies, `RESOLUTION_CLASS`, `REQUIRED_INVALIDATORS`, `OverridePolicy`, the `TargetProfile` field, `validate_override_policy`, two `AssetPopulation` fields, module doc |
| `crates/tdb-canonical/src/lib.rs` | three rules with their diagnosable names; re-exports |
| `crates/tdb-canonical/src/asset_tests.rs` | ten cases; the three rules added to the name-stability list |
| `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` | a permissive policy and a refusing one |
| `defs/adapters/asset-profiles/assets-portraits.toml` | `unspecified` on both targets, with the reason |

## 7. Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **136/136**, 0 failed |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean (repo-wide) |
| HEAD | `a2bbdc4`, unchanged |
| Index | **empty**; nothing staged or committed |
| `anchors/`, `docs/audits/**` | not read, not written |
| QA generators | not run |

`tdb-canonical` is referenced by no other crate, so nothing else compiles against the changed
`AssetPopulation`.

## 8. Questions for the architect

1. **The shipped-portrait read is authorized but has no path.** Phase 5's path-intake gate forbids
   searching for game data, `NHL_STUDIO_FIXTURES` is unset in this environment, and `anchors/`
   holds databases, not assets. The measurement needs one explicit path to an NHL Legacy Xbox 360
   or NHL 21 PS4 install (or an extracted tree). Until then every portrait constraint stays
   `unknown` and every portrait operation stays blocked.
2. **Does the database half now need the same block?** `target-adapter/2` has no override policy
   either, and Q07a.3a was written about invalid *database* values in the first place. This package
   deliberately did not widen into it.
3. Carried forward from C0-AS1: P1–P3 undecided, 26 units have no adapter, 13 families have no
   profile, ownership unassigned, population still a C0 obligation.

**Stop for Codex review.**

---

## Withdrawn — appended 2026-09-08 by package C0-AS2-R1

**This package is not accepted, and everything it implemented has been removed from the tree.** The
record is kept unaltered above; treat it as history, not as guidance. It diverged from the approved
C0-AS2 instruction in one decisive way.

### The error

It read a Q07a.3a override as a **departure from** a target's constraints, and therefore built a
per-target *permission* to depart. The authoritative reading is the opposite:

> An override changes a target Build's requested derivation settings **within** proven target
> constraints. It never permits departure from those constraints. Canonical assets and target
> profiles remain unchanged. Override records, provenance and approval belong in Build mappings.

So the whole `override_policy` construction was answering a question nobody asked. Its central
rule — `asset-override-on-undecided-constraint`, "an override is a departure from a known
requirement" — encodes the misreading directly in the rule text. Its second-order consequences were
wrong too: it put a Q07a.3a concern **in the profile**, where the approved contract puts it in the
Build mapping, and it had no notion of an approval, an approved input binding, or invalidation.

Two of its claims survive as sound instincts and are carried into the replacement: that a
specification must not become a place corrections accumulate, and that permitting a choice must
never unblock an operation or count as coverage.

### What was removed

`OverridePolicy`, `[target_profile.override_policy]`, `validate_override_policy`, the four override
vocabularies, `RESOLUTION_CLASS`, `REQUIRED_INVALIDATORS`, the `AssetPopulation` fields
`override_disposition` and `overridable_blocks`, the three `asset-override-*` rules, the ten tests,
and the policy tables added to the synthetic fixture and to `assets-portraits.toml`. The crate is
back to the C0-AS1 baseline it started from — **53 rules, 126 tests** — with the accepted AS1
schema work untouched.

### What replaces it

[c0-as2-r1-build-mapping-overrides.md](c0-as2-r1-build-mapping-overrides.md) —
`build-mapping-override/1`, a Build-mapping record with typed settings, prior values, bound
approvals and pure resolution.
