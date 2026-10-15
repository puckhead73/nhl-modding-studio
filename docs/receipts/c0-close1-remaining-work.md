# C0-CLOSE1 — reconciled C0 status and the actionable remainder

**Package:** C0-CLOSE1, revised in place by **C0-CLOSE1-R1** · **Dates:** 2026-09-08, revised
2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged)
**Status: neither version accepted.** C0-CLOSE1 was corrected by Codex review before acceptance.

> **Documentation only.** No feature implemented, no schema changed, no source or configuration
> touched, no generator run, no build run, no game data read or searched, no publication, no
> migration installed, no CI change, no fixture acquired, no product question resolved.
> **Nothing staged, committed or pushed.** No historical receipt was rewritten; superseded claims
> are labelled where they are restated, not edited away.
>
> This record **promotes nothing**. No target, family or operation changes status; **no cell
> anywhere is game-verified**; no cross-title asset compatibility is inferred; no unknown-only
> registry entry is counted as established coverage.

---

## 0. Revision R1 — conclusions withdrawn and corrected

C0-CLOSE1 reached three wrong conclusions and carried several inaccurate references. They are
**withdrawn here**, and the body below is corrected throughout. The withdrawn text is not preserved
verbatim in this file — it was one day old, unaccepted, and reproducing it would give it a currency
it never had — but every withdrawal is named:

| # | Withdrawn conclusion | Why it was wrong | Corrected in |
|---|---|---|---|
| **W1** | "Registry population is not a C0 exit criterion" | **C0 item 7 requires it in its own numbered text**: *"Expand the existing schema snapshots into the comprehensive semantic/physical registry defined by Q07a.1, and create the equivalent per-family asset output profiles."* The exit summary paragraph does not supersede a numbered deliverable. [c0-06 §5.2](../validation/c0-06-canonical-project-and-adapters.md) already said so plainly — *"the registry is therefore not comprehensive, and C0 item 7 is not complete"* — and C0-CLOSE1 contradicted the record it was reconciling | §3, §4.6 |
| **W2** | "Exactly one exit criterion is unmet (ownership)" | Follows from W1 and is false. Ownership is unmet; so is item 7's population, so is the storage policy item 5 names, and activation sequencing is undecided rather than out of scope | §3.3 |
| **W3** | Unconditional reassignment of population to C1/C4 | Population splits along a line C0-CLOSE1 did not draw: **registering evidence this repository already holds is C0 work**; **producing facts nobody has measured yet is later work**. Reassigning the whole of it was a scope reduction the developer had no standing to make | §3.2, §4.6 |
| **W4** | "Ownership assignment closes C0" | Ownership is one unmet criterion among several; assigning owners does not close the milestone | §3.3, §6 |
| **W5** | Activation described as "not a C0 criterion", i.e. effectively deferred | Reproducibility is demonstrated **on external evidence trees only**. Live migration, publication, consumer transition and CI are **inactive**, and their sequencing is an **architect decision**, not an approved scope reduction | §3.2, §4.1 |
| **W6** | "The C0-CAN1/AS1/AS2/AS3 chain is unreviewed" | The briefing **accepted C0-AS3-R2 for its scoped objective**. Acceptance status is now recorded in three distinct classes rather than one blanket claim | §4.5 |
| **W7** | "The four `QA_WRITE=1` emitters" | Wrong switch. **`QA_EVIDENCE_ROOT` enables observation capture; `QA_WRITE` controls legacy CSV regeneration**, and they are independent — observation capture does not require `QA_WRITE` | §2.2 |
| **W8** | P4 cited to C0-AS1-R1 | C0-AS1-R1's departure-policy design was **withdrawn**; the standing answer is the corrected Build-mapping override implementation | §2.2, §4.7 |
| **W9** | "18 blockers" used as the count still open | 18 is the count of register **identifiers**. FR18 is closed and FR16's schema half is closed; **17 remain open**, one of them partially | §2.2, §4.4 |
| **W10** | "No asset constraint reaches `supported-target-constraint`" restated as though it described both registries | It is a statement about the **asset** registry. The **database** adapters do carry established target-specific facts. The two must not be merged | §2.3, §4.6 |
| **W11** | Registry coverage quoted as grep counts and as "under 0.02 %" | A raw `grep` of `state = "established"` is not semantic coverage, and a semantic-mapping count over a physical field total mixes units. The population figures are **reported by `registry_population` and `asset_registry_population`** and are cited from [c0-06 §5](../validation/c0-06-canonical-project-and-adapters.md) | §4.6 |
| **W12** | "One decoded portrait read promotes most of one target profile" | Overstated. A source read establishes some **source-format** facts on the corpus read; it establishes **no install location, no runtime lookup and no target requirement**, and promotes nothing by itself | §4.6, §7 D9 |
| **W13** | FR16 framed as an architect vote on whether the 175 PS4 portraits are a distinct asset class | The subgroup is an **observed distinction**. Whether it constitutes another asset class is an **evidence** question. Only its interim representation is a decision | §7 D9 |
| **W14** | "Commit the C0 half in reviewed, named slices"; the working tree called "unbacked" | Separability is **not established** — the tree has mixed files and untracked cross-dependencies, and no commit order is approved. The accurate risk statement is that the work is **not recoverable from HEAD**; this record makes no claim about backups | §5, §6 Q0 |

---

## 1. Why this package exists

The C0 status block in [the completion roadmap](../plans/project-completion-roadmap.md) was written
by **C0-BULK1 checkpoint 7 (2026-09-07)**. Nine packages landed after it — C0-S2-INT1 and its five
revisions, C0-CAN1 + R1/R2, C0-POR1 + R1, C0-AS1 + R1, C0-AS2 + R1/R2, C0-AS3 + R1/R2 — and the
block drifted: it **understated** what is built (item 4's "emitters unwired"; item 7's "the one
profile that exists declares no established constraint", which its own preceding sentence
contradicts) while leaving the genuinely open deliverables imprecise.

The objective is an accurate, actionable remainder — not another schema round, and not a narrower
C0.

---

## 2. Reconciliation of the seven C0 items

Five states are separated throughout, because they are routinely conflated:

| State | Meaning |
|---|---|
| **implemented** | the code or record exists in the working tree |
| **tested** | a named automated case executed and asserted it (developer-run) |
| **architect-accepted** | Codex reviewed and accepted it — see §4.5 for the three acceptance classes |
| **committed** | the artefact exists at HEAD, not only in the working tree |
| **activated** | it is in the path something else depends on — published, installed, gated in CI, or consumed |

### 2.1 Item by item

| # | Item | Implemented | Tested | Accepted | Committed | Activated | Remainder |
|---|---|---|---|---|---|---|---|
| **1** | Inventory | yes | n/a | yes | **yes** (`16825ca`) | n/a | none |
| **2** | Baseline execution + isolated build | yes | yes | yes | **partly** — `c0-01/02/dt1/int1/overnight` are at HEAD; the **B1–B3 repairs are working-tree only** | n/a | B4–B7 and Windows CI (C0.2 R6) remain registered baseline defects; the commit boundary (§5) is undecided |
| **3** | Documentation reconciliation | yes | n/a | yes | **no** — `c0-03` is untracked | n/a | commit boundary only |
| **4** | Generated vs manual evidence, deterministic merge | **yes — S1, S2 and S3** | yes | S1/S3 accepted; the S2-INT1 R1–R5 chain is class **(c)** in §4.5 | **no** | **no** — reproducibility demonstrated on **external** trees only; live migration, publication, consumer transition and CI all **inactive** | §4.1 — sequencing is an architect decision, not a settled deferral |
| **5** | Fixture/runtime register | yes | measured counts | through C0-BULK1; the POR1 additions are class **(c)** | **no** | n/a | **storage policy (U3, FR2, FR3) is an item-5 deliverable and is outstanding** (§4.2) |
| **6** | Completion mapping | mapping yes (2,016-row mechanised universe) | regenerates | yes | **no** | n/a | **no item has an owner** (§4.3) |
| **7** | Canonical schema/adapter contract **and the comprehensive registry** | **format complete, both halves**; **registry population 2/28 units, 1/14 families — the numbered deliverable is not met** | yes | mixed; see §4.5 | **no** | n/a | §4.6 — the importable half is C0 work |

### 2.2 Statements corrected as stale or inaccurate

Corrected **here and in the roadmap block only**. The originating receipts stay as written; they were
accurate when written.

| Stale or inaccurate statement | Where | Correct as of 2026-09-09 | Record |
|---|---|---|---|
| "S2 partial — **emitters unwired**" | roadmap C0 block item 4; [c0-bulk1-07 §1, §4.2](c0-bulk1-07-closure.md) | **Wired.** All four matrix emitters — `studio_domain::{mutation,operation,save,surface}_matrix` — carry a parallel `observe` call through `evidence::emit::Emitter`, journalled before the first case runs. **The switch is `QA_EVIDENCE_ROOT`**, which enables observation capture and creates no file when unset. **`QA_WRITE` is a separate, independent switch controlling legacy CSV regeneration**; observation capture does not require it | [c0-s2-int1 §1](c0-s2-int1-emitter-integration.md) |
| "`check` never validated against non-empty observations" | same | **Validated**, against real preserved runs held **outside the repository** (`H:\c0-s2-int1\run1`), including an older-layout set that still loads, validates and republishes byte-identically. This is reproducibility on an **external** tree; nothing is published inside the repository | [c0-s2-int1-r5 §1, §5](c0-s2-int1-r5-file-read-integrity.md) |
| "215 `automated-qualified`" | [c0-s2-int1](c0-s2-int1-emitter-integration.md) (already withdrawn there) | **140** under corrected, versioned freshness scopes; **0 `game-verified`**; 159 `withheld` by the conservative variant rule | [c0-s2-int1-r1 §1.1](c0-s2-int1-r1-execution-truth.md) |
| "150 executing cases" (item 4) | roadmap C0 block | Superseded by the R1–R5 chain; current developer-run figures are `tdb-qa` **215** and `tdb-canonical` **191** (the latter independently verified by Codex) | [c0-as3-r2 §7](c0-as3-r2-observation-authority.md) |
| "12 registered blockers" (item 5) | roadmap C0 block | The register carries **18 identifiers**, FR1–FR18. **FR18 is closed** and **FR16's schema half is closed**, so **17 remain open**, one of them partially | [c0-05 §9](../validation/c0-05-fixture-runtime-register.md), [c0-as3](c0-as3-scoped-asset-variants.md) |
| "the one profile that exists declares **no** established constraint" | roadmap C0 block item 7 closing paragraph — contradicts the same block's preceding sentence | **7 established constraints on 1 of the profile's 3 targets** (3 at target level, 4 inside one variant), every one `observed-source-format` over an inspected corpus. **No *asset* constraint reaches `supported-target-constraint`** — a statement about the asset registry only (§2.3) | [c0-06 §5.2](../validation/c0-06-canonical-project-and-adapters.md), [c0-as1](c0-as1-asset-profile-constraints.md), [c0-as3](c0-as3-scoped-asset-variants.md) |
| "the adapter format **cannot** represent enum stored codes, null sentinels, pool behaviour, companion rules or capacity (U-A3/U-A4/U-A6)" | [c0-bulk1-07 §1 item 7, §4.5](c0-bulk1-07-closure.md) | **Closed as format gaps by C0-CAN1.** The evidence caveats under U-A3 stand and are now expressed rather than hidden | [c0-06 §5.3](../validation/c0-06-canonical-project-and-adapters.md) |
| "P4 remains undecided" | [c0-06 §7](../validation/c0-06-canonical-project-and-adapters.md), §9.3 row 2 | **Answered** — asset profiles mirror Q07a.3a. The standing implementation is the **corrected Build-mapping override**: exact approval binding ([c0-as2-r2](c0-as2-r2-exact-approval-binding.md)), source-backed variant selection ([c0-as3-r1](c0-as3-r1-source-backed-variants.md)) and the observation-authority gate ([c0-as3-r2](c0-as3-r2-observation-authority.md)), in `crates/tdb-canonical/src/build_override.rs`. **C0-AS1-R1's departure-policy design was withdrawn and must not be cited as the answer.** P1, P2 and P3 remain open | as cited |
| FR18 — `dimensions` cannot express co-existing per-target variants; `path_rule` cannot express multiple path families | [c0-05 §9](../validation/c0-05-fixture-runtime-register.md) | **Closed** by `asset-output-profile/3`: `AssetVariant` and `PathFamily`, enforced by `AssetPathFamilyUnresolved` | [c0-as3 §2](c0-as3-scoped-asset-variants.md) |
| FR16 — "`RasterValue` should stop bundling colour space" (schema half) | same | **Closed** — colour space is optional and independently validated. The **evidence half stays open** (§7 D9) | [c0-as3 §2](c0-as3-scoped-asset-variants.md) |

### 2.3 What the latest accepted package (C0-AS3-R2) settled, at its real strength

Accepted **for its scoped objective**, per the briefing:

- `operator-entered` observations are **representable and preserved**, and **cannot decide a
  variant**; no approval reaches past the gate. Tested.
- **Provenance bridges are declarations.** `validate_bridge` performs structural checks only: it
  opens nothing, verifies no citation, proves no byte identity, and establishes **no runtime lookup
  precedence**. Populating a real bridge requires separately reviewed evidence. **None exists in the
  repository.**
- **No real target-required asset constraint and no game compatibility was established.**
  `patch.big` is unopened; `[runtime_lookup]` is `unknown` on every target.
- Codex independently verified **191** canonical cases pass.

**This is an asset-side statement.** The **database** adapters do carry established target-specific
facts — 61 established blocks over two units, 30 measured and 31 derived, each linked to anchor or
audit evidence. "Zero demonstrated target-required *asset* constraints" says nothing about the
database registry, and the two must not be collapsed into one sentence.

---

## 3. C0's requirements, as written

C0's numbered items are the deliverables; the exit paragraph summarises them and does not replace
them.

### 3.1 Met

| Requirement | Met by | Note |
|---|---|---|
| Item 1 inventory; item 2 baseline and isolated build | items 1–2 | the only committed part of C0 |
| Item 3 documentation reconciliation | item 3 | record untracked |
| Item 5 register — **the register itself** | item 5 | 27 units, 18 identifiers, all enumerated. Its **storage-policy** half is not met (§4.2) |
| Item 6 — **milestone / evidence cell / test route or blocker** | item 6 | mechanised, 2,016 rows. Its **owner** half is not met (§4.3) |
| Item 7 — **the contract**: canonical schema, versioned adapter contract, expressive coverage | item 7 | `target-adapter/2` covers all 20 database constraints Q07a.1 names; `asset-output-profile/3` all 26 asset constraints, with evidence scope and force |
| Item 4 — **a defined, deterministic, reproducible merge** | item 4 | generation, merge, publication and recovery implemented, tested, and byte-identically reproducible **on external trees** |

### 3.2 Partly met — and the line that separates C0 from later work

Two items are met in form and open in substance. The distinction that matters, and that C0-CLOSE1
failed to draw:

| | **C0 work** | **Later measurement work** |
|---|---|---|
| **Item 7 registry** | Register the target-specific evidence **this repository already holds**: 27 physical schema snapshots (3,121 tables, 69,737 fields), 28 audit documents, the anchors manifest and its equivalence bridges. Represent per-target and per-family coverage, mark every fact's provenance and confidence, make unknowns **explicit**, and identify **unsupported target capabilities** | Produce facts nobody has measured: the position-code probe, an attributable shipped-asset read, byte-identity proofs, and any load/acceptance test |
| **Item 4 activation** | Nothing here is a fresh measurement — the merge is built and reproducible | Live migration, publication, consumer transition and CI are **inactive**; **sequencing is an architect decision** |

**Importing an already-supported fact must not wait on a fresh measurement.** An audit that records a
verified per-title behaviour is exactly the "fixture/audit evidence" item 7 asks each fact to link
to. **Equally, an entry consisting only of `unknown` blocks is not established semantic coverage**,
and creating profile documents full of unknowns satisfies the *representation* half of item 7 while
leaving the *population* half exactly where it was — both halves must be reported apart.

### 3.3 Unmet

| # | Unmet requirement | Class |
|---|---|---|
| **A** | Item 6 — **ownership**. Every row in both registers is `owner-unassigned` (FR12) | architect decision |
| **B** | Item 5 — **storage policy**: U3 (evidence tree location and commit policy), FR2 (mounted installs behind the fixture root), FR3 (outer-package acquisition and storage) | architect decision; FR3 also a user decision |
| **C** | Item 7 — **the comprehensive registry**: 26 of 28 units have no adapter; 13 of 14 families have no output profile; the one family that has a profile establishes 7 constraints on 1 of its 3 targets | implementation, from evidence already held |
| **D** | Item 4 — **activation sequencing**: whether migration installation, publication, consumer transition and the CI gate are inside C0's boundary, and in what order | architect decision |
| **E** | Acceptance of the packages in class (c) of §4.5 | architect review |

**None of A–E alone closes C0**, and ownership assignment in particular does not. Registered
fixture and runtime blockers (§4.4) are a different matter: an enumerated blocker is what the exit
criterion permits in place of a test route, and those do not block closure.

---

## 4. Unmet deliverables, one row each

Format: exact remaining deliverable · supporting requirement · current evidence · dependencies ·
acceptance check · what unblocks it.

### 4.1 QA migration, publication, consumer transition and CI

| | |
|---|---|
| **Deliverable** | Install `mig-c0-bulk1` (`sha256:39e56bb1…`); publish the first real layer-A merge to a decided location; transition `game-qa-matrix.csv`, `matrix_guard` and the emitter CSV paths (dual-run, then retire); add the `qa-evidence check` CI gate |
| **Requirement** | C0 item 4. The **definition** is met; **activation is inactive, and whether it sits inside C0 is an architect decision, not a settled deferral** |
| **Evidence today** | Merge, publication, journalled recovery and migration implemented and tested; `check` validated against a real non-empty tree **outside the repository**; first real merge = **140 `automated-qualified`, 0 `game-verified`** ([c0-s2-int1-r1 §1.1](c0-s2-int1-r1-execution-truth.md)). Nothing published inside the repository; `docs/audits/evidence/` does not exist |
| **Dependencies** | **U3** gates all four. **U2** (`conversion.class` vocabulary) makes pair keys semantically final. **M1/P3** leaves 2 migration claims unmapped. **R-NO-CI**: the gate needs a real root. **R-TRANSITION**: no consumer retires before the replacement is live |
| **Acceptance** | A published tree that re-merges byte-identically; `qa-evidence check` exit 0 in CI against that tree; every migrated claim mapped or explicitly refused; the transitioned consumer's last output reproduced from the new tree |
| **Unblocked by** | **Architect decisions** (U3, U2, M1/P3, and the sequencing question itself), then implementation |

### 4.2 Evidence and fixture storage policy — an item-5 deliverable

| | |
|---|---|
| **Deliverable** | Decide and record **U3**, **FR2** and **FR3** in the register |
| **Requirement** | **C0 item 5 names storage policy in its own text.** This is outstanding C0 work, not follow-on |
| **Evidence today** | The register enumerates all three and refers them to the architect unchanged. Q04's required outer-package cells have **no route at all**. 7 `tdb-rx2` cases plus ~20 elsewhere depend silently on mounted trees and are therefore machine-dependent |
| **Dependencies** | The EA data policy (CLAUDE.md rule 2) bounds what may enter git |
| **Acceptance** | Each of U3/FR2/FR3 carries a decision, a location and a next action; no register row leaves the policy question open |
| **Unblocked by** | **Architect decision**; FR3 additionally needs a **user** decision on acquisition |

### 4.3 Ownership assignment

| | |
|---|---|
| **Deliverable** | An accountable implementer or manual tester on each of the 10 milestone rows in [c0-06 §6](../validation/c0-06-canonical-project-and-adapters.md) and each of the 18 register identifiers in [c0-05 §9](../validation/c0-05-fixture-runtime-register.md) |
| **Requirement** | C0 item 6 and the exit text |
| **Evidence today** | Mapping mechanised (2,016 declared rows, regenerates); the owner column is uniformly `owner-unassigned` |
| **Dependencies** | none — no code, no fixture, no measurement |
| **Acceptance** | No `owner-unassigned` row remains in either record |
| **Unblocked by** | **Architect decision.** The cheapest remaining item — **and it does not by itself close C0** (§3.3) |

### 4.4 Fixture and runtime gaps

Registered, and per the exit text **explicitly permitted** in place of a test route. None blocks C0
closure; each blocks a later milestone. **17 of the 18 identifiers remain open**, one partially.

| Blocker | Blocks | Class |
|---|---|---|
| FR1 — 6 private fixtures absent, 26 `tdb-rx2` cases not run | C4 | fixture |
| FR4 — `legacyrecomp-recomp` has no anchor, so no physical snapshot exists to build an adapter over (U-A5) | C1, C9; **bounds item 7 to 27 of 28 units** | fixture |
| FR5 — `ps3/nhl07` has no save fixture (research-only; likely never needed) | C2 | fixture |
| FR6 — no runtime build identity or deployment path for any of the four runtimes | **C9 entirely** — R-IDENTITY withholds every game acceptance | runtime |
| FR7 — per-title save modes not inventoried against Q03 | C2 | evidence |
| FR8 — `texlib_67.rx2` byte identity unproven | C3 | provenance |
| FR9 — NHL 19 PS4 title id `CUSA11126` vs `CUSA11116` | C2/C4 | correctness |
| FR10 — audio: no fixtures, no implementation | C7 | scope |
| FR11 — Blender 4.5 presence unknown at both resolution locations | C5 | runtime |
| FR13 — `por1/ps3-nhllegacy` carries third-party writes | C4 | fixture provenance |
| FR14 — the Xbox 360 and PS4 portrait corpora are derived trees, not attributable to their titles | C4 | evidence |
| FR15 — no PS4 portrait pixel data decoded; BC7 and `tiling_index 13` unexercised | C4 | evidence |
| FR16 (evidence half) — colour space unmeasured on Gen7; an observed subgroup on PS4 (10 246 vs 175 in the inspected corpus) whose class status is unestablished | C4 | evidence |
| FR17 — `patch.big` never opened; it may shadow the loose PS3 portraits | C4 | evidence |
| FR2, FR3, FR12 | — | **policy/ownership — §4.2, §4.3, not later-phase** |

**Closed:** FR18 entirely; FR16's schema half.

### 4.5 Acceptance bookkeeping

Three classes, kept apart. **A missing on-disk acceptance record is not proof that no review
occurred**, and this record asserts no blanket acceptance in either direction.

| Class | Packages | Status |
|---|---|---|
| **(a) Accepted** | C0 items 1–3, 5 and 6 through C0-BULK1 and its R1/R2; **C0-AS3-R2, accepted for its scoped objective** per the C0-CLOSE1 briefing | accepted as scoped |
| **(b) Superseded in part by later accepted work** | C0-S2-INT1 (two claims withdrawn by R1); C0-S2-INT1-R1 (three integrity statements corrected by R2); C0-POR1 (fourteen promotions cut to three by R1); **C0-AS1-R1's departure-policy design (withdrawn; the corrected Build-mapping override stands in its place)**; C0-BULK1 §2.5's isolation claim | the superseding text governs |
| **(c) Acceptance not located in repository records** | C0-S2-INT1-R2/R3/R4/R5; C0-CAN1 + R1/R2; C0-POR1-R1; C0-AS1 + R1; C0-AS2 + R1/R2; C0-AS3 + R1; contract **R2**'s nine resolutions (X8–X16, R-CONFLICT-CLASS, U1's closure) | **no acceptance record was found in this repository.** Whether review occurred outside it is not something this record can establish — Codex to confirm or direct |

### 4.6 The comprehensive registry — C0 item 7's population half

**This is outstanding C0 work.** It splits into a part that needs no new measurement and a part that
does.

**Population as reported by the counters, not by grep** — `registry_population` and
`asset_registry_population`, cited from [c0-06 §5.2](../validation/c0-06-canonical-project-and-adapters.md):

| Layer | Reported coverage |
|---|---|
| Physical facts | **27 / 27 targets**, 3,121 tables, 69,737 fields — already generated and committed |
| Semantic adapters | **2 / 28 catalog units** |
| Semantic mappings | **13 mappings + 1 omission**; **2** reach `automated-tested` |
| Established constraint blocks | **61** (30 measured, 31 derived) against **63** unknown, 20 of those carrying a non-operative candidate |
| Asset output profiles | **1 / 14 families**; **3 / 28 target profiles** within it; **7 established constraints**, all on `nhllegacy-ps3`; 45 unknown target-level blocks, 22 with candidates; `target_required_constraints` pinned at **0** by test |

No percentage is quoted here: a semantic-mapping count and a physical-field total are different
units, and dividing one by the other produced a number C0-CLOSE1 should not have printed.

**(i) Registration from evidence already held — C0.** Sources: the 27 committed physical snapshots,
the 28 documents in [docs/audits/](../audits/) (schema drift, roster capacity, ratings write codec,
scalar mutation, cross-platform format, save container, relational operations and the rest), the
anchors manifest with its per-game cross-platform equivalence bridges, and `defs/enums/`. Each
imported fact carries its source, its confidence and its applicability; a fact whose stated anchor
range does not cover the target is a **candidate**, never an established block. The deliverables are
per-target and per-family **coverage representation**, **explicit unknowns**, and identified
**unsupported target capabilities**. **FR4 bounds this to 27 of 28 units** — `legacyrecomp-recomp`
has no snapshot to build over.

**(ii) Genuinely missing facts — later.** The position-code probe (`ubPc.vjla`) promotes, **on the
probed target only**, that target's discriminator and its ratings applicability predicates, and — for
NHL 15 only — its companion rule; it does **not** establish handedness (`ubPc.GRkp` is a different
field with a different vocabulary), and one anchor does not carry across titles. On the asset side, a
decoded read of an **attributable** shipped asset would establish some **source-format** facts about
the corpus read — and **no install location, no runtime lookup and no target requirement**; FR14 and
FR17 must be settled first or the read is of a derived tree again. Proving what a target *requires*
needs a load, acceptance or shipping-manifest test (U-A13), which no package has been authorized to
perform.

**Reading rules that survive from C0-CLOSE1 unchanged:** an `unknown` block — including one carrying
a candidate — is **not** established coverage; nothing in either registry states a demonstrated
target requirement it has not measured; and no cross-title asset compatibility is implied anywhere.

### 4.7 Canonical and Build questions still outstanding

| # | Open | Effect while open |
|---|---|---|
| **P1** | Does the canonical model carry a stored overall rating, a computed one, or both? | decides what a Project holds and what a Build writes; C1 item 2 depends on the distinction |
| **P2** | Does a Project retain target-native fields as provenance, round-tripped on export? `target-adapter` has **no syntax** for it | decides whether Standard→Project promotion is lossless |
| **P3 / M1** | Is `assets-jerseys` the family the legacy `uniforms` rows belong to? | 2 migration claims stay unmapped |
| — | Project UI, converter and Build engine | **out of C0 by instruction**; C5/C8 own them. `resolve_build_override` is a resolver, not a Build |

**P4 is answered** — by the corrected Build-mapping override, not by C0-AS1-R1's withdrawn departure
model (§2.2).

---

## 5. The commit boundary

The working tree carries **242 entries** with an **empty index**.

- **C0's own records are almost entirely uncommitted.** At HEAD: `c0-01`, `c0-02`, `c0-dt1`,
  `c0-integration-01`, `c0-overnight`. Untracked: `docs/receipts/` in its entirety, `c0-03`,
  `c0-04a`, `c0-05`, `c0-06`, `c0-fx1`, `docs/plans/completion-design-questions.md`, and
  **`docs/plans/project-completion-roadmap.md` itself**.
- **C0's source is uncommitted**: `crates/tdb-canonical/**`, `crates/tdb-qa/src/evidence/**`,
  `defs/canonical/`, `defs/adapters/`, `defs/qa/scopes.toml`.
- **The tree is mixed, and separability is not established.** It also holds the unrelated
  `feat/phase5-asset-tooling` work — `app/**`, `studio-api`, `studio-domain`, the mesh and asset
  crates. Several files are modified for both reasons, and untracked C0 paths depend on tracked
  files that carry feature changes. **Whether a clean C0-only slice exists is an open question, not
  a plan**, and no commit order is approved.
- **The baseline is split.** C0-INT1's B1–B3 repairs, which make repo-wide `fmt` and `clippy` pass,
  live only in the working tree; **at HEAD both still fail**, and C0-INT1 §7.2 records that they
  cannot be committed standalone. Every "clean" check in the recent receipts is a working-tree
  statement, correctly labelled as such in each.

C0's exit text does not require a commit. The accurate statement of consequence is narrow: **none of
this work is recoverable from HEAD.** This record makes no claim about whether copies exist
elsewhere.

---

## 6. Proposed remaining queue

Ordered, with explicit dependencies. **Proposals for Codex to sequence; no order is approved, and
none of these packages is authorized by this record.**

| # | Package | Contents | Depends on | Decisions it needs first |
|---|---|---|---|---|
| **Q0** | **C0-SLICE1 — read-only commit-boundary analysis** | Enumerate, without staging anything: which paths are C0-only, which are feature-only, which files are modified for both reasons, and which untracked C0 paths depend on tracked feature-modified files. Report whether a separable C0 slice exists at all, and what the C0-INT1 §7.2 constraint forces. **Produces a proposal, not commits** | — | D10 (whether to analyse now) |
| **Q1** | **C0-OWN1 — ownership and storage policy** | Record the architect's owner assignments across `c0-05 §9` and `c0-06 §6`; record U3, FR2 and FR3 with decisions, locations and next actions. Closes §3.3 A and B — **not C0** | — | **D1, D2, D3, D4** |
| **Q2** | **C0-REG1 — register the database evidence already held** | Expand adapters across the 27 units that have a snapshot, importing from the physical snapshots, the 28 audits, the anchors manifest and `defs/enums/`. Every fact linked and confidence-marked; unknowns explicit; unsupported target capabilities identified. Report established and unknown coverage apart, via `registry_population` | acceptance of the C0-CAN1 chain (§4.5 class c) | none blocking; FR4 bounds it to 27/28 |
| **Q3** | **C0-ASP1 — create the missing per-family asset profiles** | Create output profiles for the 13 families that have none, populated **only** from evidence already held, with unknowns explicit and unsupported capabilities identified. **A profile of unknowns is representation, not population** — Q3 must report both counts | acceptance of the C0-AS chain; Q2's import conventions | D9's interim representation |
| **Q4** | **C0-ACT1 — activate the evidence system** | Install `mig-c0-bulk1`; first real publication to the U3 location; map or refuse the 2 unmapped claims; dual-run then transition `game-qa-matrix.csv`, `matrix_guard` and the emitter CSVs under R-TRANSITION; add the `qa-evidence check` CI gate | Q1 (U3, P3) | **D2, D7, D8, and the sequencing decision D11** |
| **Q5** | **C0-CI1 — Windows CI (C0.2 R6)** | Add the Windows job once repo-wide `fmt`/`clippy` pass **at HEAD** | Q0 and whatever commit decision follows it | D10 |
| **Q6** | **MEAS-DB1 — first authorized database measurement** | The `ubPc.vjla` probe as a named case on one target; promote only what that target's evidence carries | Q2 | fixture authorization |
| **Q7** | **MEAS-AS1 — first attributable asset read** | Settle FR17 and FR14; then one decoded shipped-asset read; promote only at the scope reached. Install and runtime-lookup facts are **not** products of this package | Q3; FR14/FR17 | D3 storage policy |
| **Q8** | **CAN-MODEL1 — P1 and P2** | Stored vs computed overall; target-native extension syntax and export round-trip | — | **D5, D6** |

Q1 through Q4 are C0's remainder. Q5 is C0.2's. Q6 and Q7 are the later measurement work §3.2
separates out, and neither is a prerequisite for Q2 or Q3.

---

## 7. Decisions requiring the user or the architect

| # | Decision | Who | Recommendation | Consequence of leaving it open |
|---|---|---|---|---|
| **D1** | Owner assignment across both registers (FR12) | **architect** | assign early; it costs nothing and it is a hard criterion — while noting it does not close C0 alone | §3.3 A stays unmet |
| **D2** | **U3** — evidence tree location; are layer-A runs committed? | **architect** | commit **layer B (manual) only**; keep layer-A runs out of git and republish them, so no generator run can produce diff churn or overwrite manual sign-off | blocks Q4 entirely |
| **D3** | **FR3** — outer-package fixture acquisition and storage | architect + **user** | acquire one real outer package per console family behind `NHL_STUDIO_FIXTURES`, never into git | Q04's required cells stay unroutable |
| **D4** | **FR2** — do mounted game installs belong behind the fixture root? | **architect** | **yes** — same explicit root, and let a selected case **fail** on absence as C0-FX1 did for `tdb-rx2`; silent machine-dependence is the worse failure | ~27 cases stay silently machine-dependent |
| **D5** | **P1** — stored vs computed overall in the canonical model | **user** (product) | carry **both**, explicitly labelled | C1 item 2 and Build semantics stay unspecified |
| **D6** | **P2** — retain target-native fields as provenance? | architect | **yes**, retain and round-trip; a lossy promotion is not reversible | Standard→Project loss is unbounded |
| **D7** | **P3/M1** — does `assets-jerseys` own the legacy `uniforms` rows? | architect | do **not** map on likelihood; refuse the 2 claims until decided | 2 migration claims unmapped |
| **D8** | **U2** — `conversion.class` / recipe-id vocabulary | architect | freeze before publication so pair keys become semantically final | pair keys stable but not final |
| **D9** | **FR16 interim representation** — how to record the observed PS4 portrait subgroup (10 246 vs 175) pending evidence | architect (representation only) | record it as an **observed distinction with its class status unknown**; do not average the two groups into one constraint. **Whether it is a distinct asset class is an evidence question and cannot be settled by decision** | Q3 cannot represent the family honestly |
| **D10** | Whether to run Q0's read-only slice analysis, and what to do with its finding | architect + **user** | run the analysis before proposing any commit; none of this work is recoverable from HEAD | the boundary stays unexamined |
| **D11** | **Activation sequencing** — are migration, publication, consumer transition and the CI gate inside C0's boundary, and in what order? | **architect** | — no developer recommendation; this is a scope call | §3.3 D stays open and Q4 stays unsequenced |

**None of these is decided here, and none is silently deferred.**

---

## 8. Checks performed

Static reference and consistency checks only. **No tests, generators, builds, game-data access,
migration, publication, staging, commit or push.** No figure below is a fresh measurement by this
package; population figures are cited from the counters the canonical crate reports, never from a
text search.

| Check | Method | Result |
|---|---|---|
| Emitter wiring and its switch | source inspection of `crates/tdb-qa/src/evidence/emit.rs` and the four matrix test binaries | wired via `Emitter`; **`QA_EVIDENCE_ROOT` enables capture, `QA_WRITE` is the independent legacy-CSV switch** — W7 corrected |
| Evidence module inventory | `crates/tdb-qa/src/evidence/` | 15 modules including `emit`, `journal`, `merge`, `migrate`, `publish`, `paths`, `validate` |
| Registry population | **cited** from [c0-06 §5.2](../validation/c0-06-canonical-project-and-adapters.md), as reported by `registry_population` / `asset_registry_population` | 2/28 units; 13 mappings + 1 omission; 61 established / 63 unknown; 1/14 families, 3/28 target profiles, 7 established, `target_required_constraints` = 0 |
| Registry membership | `defs/canonical/`, `defs/adapters/` | 2 adapters, 1 asset profile, 1 project schema |
| Item 7's own text | [roadmap C0 item 7](../plans/project-completion-roadmap.md) vs [c0-06 §5.2](../validation/c0-06-canonical-project-and-adapters.md) | population is a **numbered C0 deliverable**; c0-06 already said item 7 is not complete — W1/W2/W3 corrected |
| P4's standing answer | `crates/tdb-canonical/src/build_override.rs`; c0-as2-r2, c0-as3-r1, c0-as3-r2 | the corrected override implementation; C0-AS1-R1's departure model was withdrawn — W8 corrected |
| Register identifiers vs open count | [c0-05 §9](../validation/c0-05-fixture-runtime-register.md) + C0-AS3 closures | 18 identifiers, **17 open**, one partially — W9 corrected |
| Acceptance records | search of `docs/receipts/` and `docs/validation/` for acceptance statements | class (a)/(b)/(c) in §4.5; class (c) records were **not found in this repository**, which is not evidence they were not reviewed |
| Commit boundary | `git status --porcelain`, `git log -- docs/` | 242 entries, **empty index**, HEAD `a2bbdc4`; separability **not established** — W14 corrected |
| Relative links | scripted check of both edited files | 0 broken |

---

## 9. Preservation

| | |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` — unchanged |
| Index | **empty** at every boundary |
| Reset / restore / clean / stash / checkout / branch change / staging | **none** |
| Working-tree entries removed | **0** |
| Files changed | **2** — this receipt, and the C0 status block in the roadmap |
| Source, configuration, `defs/`, `anchors/`, `app/`, `crates/` | **not touched** |
| Historical receipts and validation records | **not rewritten**; superseded and withdrawn claims are labelled in §0 and §2.2 and left standing where they were written |
| Generators, `capability.csv`, evidence publication, migration, fixtures | **not run, not installed, not acquired** |
| Commits / pushes | **none** |

---

## 10. Files changed

| File | Change |
|---|---|
| `docs/receipts/c0-close1-remaining-work.md` | This record; revised in place by C0-CLOSE1-R1 with §0 recording every withdrawn conclusion |
| `docs/plans/project-completion-roadmap.md` | **C0 status block only** — no other section touched |

**Nothing staged, committed or pushed. Stopping for Codex review.**
