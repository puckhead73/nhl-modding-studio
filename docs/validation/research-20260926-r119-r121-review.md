# Coordinator review — research receipt 3 / R119–R121

26 September 2026. Continue the same research session. Publish the corrections below, then continue the authorized queue; no routine re-review stop is needed.

## Disposition

The correction checkpoint addresses the earlier median/spread, consistency-routing, schedule-total and archived-bank wrapper wording findings. R119's record reconciliation is accepted as a derivation from the published censuses. R120's measured nationality code shifts are supported. Its claim about **current Studio behavior on NHL 08/09 is incorrect**, because the comparison ignores an existing runtime vocabulary override. Correct that developer-facing scope before using C16 as an application defect inventory.

R120/R120b/R120c were independently rerun against their registered sources and reproduce their published JSON exactly. R121's substantive counts reproduce, but tied rankings are not deterministic. These are offline research checks, not new game, desktop or listening acceptance.

## 1. The enum-file comparison is not the actual application resolver

`r120b_nationality_label_table.py` reads only `defs/enums/nationality.toml`. It calls the result `studio_label`, although `view.rs::read_field` subsequently applies `attributes.rs::apply_gen6_geo_vocab` to both nationality (`Lelh`) and birth state (`QvxI`). Its gate is `is_gen6_style_schema`: `dSvy.mQoE` with bit width 3, not a title-id check restricted to NHL 07.

Independent descriptor reads confirm NHL 07, 08 and 09 use that 3-bit field; NHL 10 uses 4 bits. The override includes 79=Czech Republic, 82=Germany, 89=Switzerland and 96=Japan. Therefore the shared TOML's missing/wrong entries do **not** establish wrong field-view labels on NHL 08/09. The matching nationality/state field IDs were also inspected. This conclusion is from the measured descriptors and current source path, not a desktop observation.

The reported NHL 10 cases remain actionable: 81=Croatia, 84=France and 95=Slovakia in the shared vocabulary, against the Czech, German and Swiss roster associations. NHL 11/12 code 108 has no shared Japan label. Keep these source-specific cases. Audit other consumers independently: imports resolving the enum registry directly may differ from the field view. Do not claim every display/edit/import path is correct simply because this override exists.

Existing NHL 07 code comments record user-confirmed countries, player-based birthplace evidence, an explicitly inferred Belgium entry and the DC/Delaware exception. They also state that the Origin dropdown's alphabetical ordering does not follow stored numeric codes. These are existing provenance to reconcile, not grounds to reopen all NHL 07 labels or to infer numeric codes from option order. Preserve accepted user observations. Style labels also have synthetic vocabulary and filtering paths; a TOML-only comparison cannot establish their current application behavior.

## 2. Narrow the summaries and recommendations

- The Croatian/French/Slovak examples are specifically NHL 10. Even the raw TOML comparison gives NHL 08/09 Czech=unlabelled, German=Czech Republic, Swiss=Netherlands; the effective field view then overrides those entries. Update handoff, receipt, queue and status with a dated correction rather than generalizing the NHL 10 examples.
- “Matches all 21 national teams from NHL 13” overstates exact label agreement. R120b itself reports code 83 as **England**, whereas its abbreviation map calls the team **Great Britain**. National-team membership does not by itself settle that distinction. Separate stable code association from exact label semantics; do not automatically rename England to Great Britain. Preserve ambiguous abbreviation cases and mixed-nationality rosters.
- The wider field is a measured schema fact, not a demonstrated 7-bit editor limitation. D2 must identify a failing consumer or remain an audit question. Code 127's observed presence/absence does not by itself establish a sentinel contract on every title.
- Zero-height rows are measured; a global `raw 0 = missing` interpretation is not established by their frequency. D3 should request a bounded presentation investigation, with explicit placeholder criteria and the named rostered exception, not authorize hiding rows or changing every minimum encoded height.
- Birth-state country continuity supports investigation but is not an independently recovered title-owned label table. Weight/height/handedness continuity supports the existing contract; it does not prove unchanged meanings in every possible field/value/title. Preserve the stronger, separately measured roster-position result and the limited source coverage.

These corrections narrow conclusions, not the raw code-transition evidence. The developer already has a prompt to reproduce NHL 10's wrong label and avoid speculative width/sentinel changes. A dated addendum to that prompt records the NHL 08/09 override finding; no separate chat was messaged.

## 3. Reproduction and inspection

Executed with bundled Python, `-B` to avoid writing bytecode into research-owned directories, outputs isolated under reviewer-owned `local/runs/coordinator-r119-r121-review`:

| Probe | Result |
|---|---|
| `r120_identity_audit.py` | exit 0, 14.33 s; 19 distinct sources; exact parsed-JSON equality |
| `r120b_nationality_label_table.py` | exit 0, 0.06 s; 18 national-roster sources; exact parsed-JSON equality (confirms the table calculation, not its incomplete model of Studio behavior) |
| `r120c_position_line_slots.py` | exit 0, 13.19 s; 19 sources; exact parsed-JSON equality |
| `r121_unscoped_enum_continuity.py` | exit 0, 24.59 s; 19 sources; source metadata/distributions, pair populations, unchanged shares, flagged source codes and count profiles match |

R121 iterates set intersections, then uses `Counter.most_common()` with truncation. Ties therefore change list order and sometimes which equal-count transition enters the last retained slot. Comparison confirmed equal counts for common transitions and that differing retained transitions occur only at equal-count cutoffs. Sort match keys and break ranking ties by source/destination code when maintaining this probe; this is a reproducibility correction, not evidence of different populations or remapping.

Independently recomputed R119 from R82b and R98 JSON:

| Source | Present + absent | R98 slots |
|---|---|---|
| PS3 NHL 09 | 2594 + 25 = 2619 | 2619 |
| PS3 NHL 10 | 2748 + 24 = 2772 | 2772 |
| PS3 NHL 12 | 4697 + 37 = 4734 | 4734 |
| PS3 NHL 13 | 5117 + 43 = 5160 | 5160 |
| Xbox NHL 09 | 2426 + 25 = 2451 per copy | 4902 across 372 members, counting archive and loose copies |

The “only empty slots” explanation needs the Xbox duplicate-copy qualifier already present in the detailed handoff. Archive members were not re-read for this reconciliation.

Inspected source matching/join code, the slot-position and zero-row probe, R121's companion joins, the safe writer, relevant resolver code and definitions, current claims, receipt/status/queue and corrected handoffs. The four reported text files now have uniform line endings; `safe_patch.py` parses successfully. No fresh fault-injection/atomicity claim is made for that helper.

A small direct-rustc field-view harness was attempted against the existing domain rlib. Three link attempts failed on MSVC CRT symbols; it never executed, and no public-API result is claimed from it. No application files were changed. Compact totals, replay dispositions, descriptor widths and source hashes are in `research-20260926-r119-r121-evidence.json`.

## Cleanup and next work

Reviewer scratch/output/build files under `E:\Repositories\nhl-modding-studio\local\runs\coordinator-r119-r121-review` were removed after saving the compact record. The resolved root was verified within the named runs directory, all descendants checked for reparse points, and removal verified. Shared caches and research originals were untouched. Next research: correct C16's effective-resolver assumptions, then seek genuinely missing title-owned style/country label evidence, preserving existing NHL 07 acceptance. If bounded static searches yield no new discriminator, proceed to Players 3.4 portrait/head reassignment inputs and independent League/Team gaps. Do not implement the separately owned head-conversion workstream. Projects/Builds stays deferred.
