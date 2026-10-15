# Checkpoint 09 — corrections after receipt 5 and developer receipts 55–62

Research lane, 28 September 2026, published 13:20 MDT (system clock). Prompt:
[`research-20260928-after-receipt5.md`](../../plans/research-20260928-after-receipt5.md) §1. Claim `research-q7-after-receipt5-corrections`.
Inputs read:
- the coordinator's [receipt-5 review](../../validation/research-20260928-receipt5-review.md) and [receipt-60 review](../../validation/development-20260928-receipt60-review.md);
- developer receipts 55–62 (60 is the consolidated receipt; 61 Hockey Shop rollback F1; 62 Recompiled public edits);
- the R137 / R137-ext / R137c / R138 / R128 data files.

Nothing was rerun for this checkpoint. Every number below is read back from the published JSON.

## Plain-language summary

Four statements in receipt 5 said more than was measured:
- The Recompiled result counted matching **paths**. It compared file **bytes** only for a 52-file sample, plus whole archives.
- "Every arena file has an identical archive copy" does not hold as written. PS3 NHL 11–13 have no loose copy at all, and on Xbox 360
  NHL 10 the archives were never compared.
- The board-ad contract both required writing the archive copy and allowed writing only the loose one. The loose-only allowance is withdrawn.
- The NHL 11–13 style labels are now a closed **coordinator** decision. They stay numbers.

The developer has also delivered several items the handoff still listed as absent: PS3 venue previews, Creation Zone names,
archive-held reassignment and Recompiled sound banks. None of that is reviewed or runtime-accepted.

## K9 — R138 Recompiled identity, narrowed

| Claim in receipt 5 / handoff 04 | What was measured | Corrected wording |
|---|---|---|
| "44 602 / 44 606 X360 Legacy loose files = one archive member each" | lower-case **path** match of every X360 Legacy loose file against the Recompiled EB member lists: 44 602 unique matches; 0 paths in two archives; 4 loose-only = `gamedata\{aidata, gps, nis}.big`, `gamedata\anim\faceposer.big`, which Recompiled ships as the same top-level files | **44 602 unique path matches, 4 shared-file exceptions.** A path match is not a byte comparison. |
| "52 / 52 samples identical" | 52 members (the smallest and a middle-sized member per class, 13 classes) unwrapped and compared with the loose file | **52 sampled byte comparisons**, all equal. The other 44 550 path matches were **not** byte-compared member by member. |
| receipt 5 summary: "where Legacy Recompiled keeps every file Studio needs"; handoff 04: "The **bytes** are identical" | R128: the 13 top-level `.big` archives and 7 other shared paths are whole-file SHA-256 equal between the Recompiled `game\` and the X360 Legacy tree (20 / 20) | **Whole-archive identity** is separate, source-scoped evidence. It shows that Recompiled's archive members are the X360 Legacy **archive** members. It does not compare the X360 **loose** override files with those members. |

Where two measurements chain, the scope is exact. Examples:
- **Venue packages.** R137 compared all 67 X360 Legacy `cacherender.big` stadium members with the loose files in full. With the
  archive-level identity, the 67 Recompiled venue members equal the X360 loose packages.
- **Main database.** R128 read one member by offset: `db/nhlng.db` equals the anchor.

For any other class, "bytes identical to the X360 Legacy loose file" is measured only for the sampled member. Otherwise it holds
only as far as the loose file equals its own archive member, which has not been measured per class.

## K10 — R137 copy topology, with its qualifier

Read back from `r137*` `aliases`. "Pair" means loose file and archive member compared in full after unwrapping.

| Tree | Topology | Measured |
|---|---|---|
| X360 NHL 11 | loose + archive pair | 53 / 53 equal (`stadium.big`, chunklzx) |
| X360 NHL 12 / 13 / 14 / 15 / Legacy | loose + archive pair | 59 / 64 / 67 / 67 / 67 equal (`cacherender.big`, chunklzx) |
| PS3 NHL 14 / 15 / Legacy | loose + archive pair | 67 / 67 / 67 equal (`cacherender.big`, chunkzip) |
| PS3 NHL 11 / 12 / 13 | **archive-only** (R137c, handoff 03 §9) | no loose venue package exists; the member is the only copy |
| Legacy Recompiled | **archive-only** (no loose tree, R138) | the members equal the X360 Legacy pair by archive identity |
| **X360 NHL 10** | loose files (49); archives are BIG4 | **not measured**: the probe skipped BIG4 archives ("names not compared"). Whether an archive copy exists is unknown. |

**Correction to handoff 03 §2 and receipt 5.** "Every loose venue package has exactly one archive twin with the same bytes" and "loose ↔
archive twins identical everywhere" are replaced by the table above. X360 NHL 10 is **unmeasured**, not paired. Until its BIG4
members are listed, a writer must refuse NHL 10 or treat the topology as unknown. Handoff 10 §2 carries this as a gate.

## K11 — handoff 03 §5 copies rule: the loose-only allowance is withdrawn

§5 said "Plan both the loose file and its archive twin, or refuse" and then "the loose-only write is allowed only if the operation
states that the twin stays stale". These defaults contradict each other. The second sentence is **withdrawn**. The replacement rule
is in [handoff 10](10-ps3-board-raster-writer-contract.md) §2:
- **measured pair:** both copies share one review and recovery scope, or the operation refuses;
- **archive-only (PS3 NHL 11–13):** use the member route alone, and never invent a loose counterpart;
- **unmeasured (X360 NHL 10):** refuse;
- **unknown precedence** stays unknown. It is never a reason to update only one copy.

## K12 — S3 closed as a coordinator decision

**Coordinator decision (receipt-5 review):** keep NHL 11–13 companion style values numeric until there is an independently supported
mapping.
- This is an intentional presentation policy. It is **not** a user decision, not a defect only because labels are absent, and not
  an automatic beta blocker.
- Accepted title-specific mappings keep their scope, and stored codes are never remapped.
- Research makes no code or definitions change.
- New independent evidence may reopen the question. The same continuity test is **not** repeated.

BETA-HANDOFF row B2 is updated accordingly.

## K13 — R123 revision 2, replayed

The coordinator replayed 27 old vectors, 29 new vectors and 144 stock tables and got an exact parsed-JSON match. The correction is
accepted as a research reference for the bounded single-phase scope. It is not rerun here. Austrian phased writing stays withheld,
and its runtime meaning stays card P1-A.

## K14 — consumption after developer receipts 55–62 (delivery ≠ review ≠ runtime acceptance)

| Research input | Developer delivery | Still open |
|---|---|---|
| handoff 17 §9 + 04 (archive-held reassignment) | **55**: PS3 NHL 09 → 14 and Recompiled portrait / head reassignment through listed archive members | review; in-game selection (P34-1); PS4 heads (developer's active `dev-ps4-head-reassign`) |
| 06 §4 (`.ast` NHL 10 / 15 / Legacy) | **56**: stream shapes admitted | listening; surround speaker order |
| 08 (Creation Zone names) | **57**: PS4 17–21 gear and mask names via `loc_hash`, hash-gated dictionary | review; G-MASK-1 (slot vs personal mask) |
| 03 §1 / §8–9 (venue previews) | **58**: PS3 NHL 14 / 15 / Legacy pictures; X360 NHL 11 / 12 / 14 and PS3 NHL 14 in the gallery; PS3 NHL 11–13 not listed (archive-only) | **no writer**; archive-only listing; desktop observation |
| 04 (Recompiled routes) | **59**: title-id spelling defect fixed; Recompiled sound banks from its own archives (5 PCM-equal previews). **62**: public-path edit / import / save / reopen regression under `legacyrecomp` | other Recompiled read routes (kits, logos, venue, loc) not claimed; loader precedence; saves; roster ceilings **not** inferred from identical DB bytes |
| — | **61**: Hockey Shop rollback keeps the original on failed restore (F1) | coordinator review |

## Next

Package 2 is the PS3 board-raster writer contract, [handoff 10](10-ps3-board-raster-writer-contract.md).
