# C0-COMMIT-REG2 + C0-REG3 — receipt

**Package:** commit the accepted C0-REG2 batch in two isolated slices; register `nhl10-ps3`,
`nhl11-ps3`, `nhl12-ps3`, `nhl13-ps3` on their own committed evidence (C0-REG3, uncommitted for
review); write the asset-contract design for the two recorded gaps and the external-project
ingestion record.
**Baseline:** `52a12fd1c0d1e4169e62951382dea6c24e17cd5f`, branch `feat/phase5-asset-tooling`.
**Date:** 2026-09-15.

**Result.** Both REG2 commits landed on trees byte-identical to the trees tested in isolation
(`2ec9fde` → 290 cases, `0251b57` → 291 cases); index empty after each; nothing pushed. REG3 adds
four measured PS3 adapters and 50 named cases (291 → **341**, clippy `-D warnings` and rustfmt clean
on an isolated candidate built from `0251b57` + a 10-file overlay). NHL 10 diverges from NHL 11
onwards in three measured ways and one of them is a **conflict between two committed sources** (the
ratings audit names NHL 10 under offset 36; the production probe resolves 50 on the anchor) — the
adapter records it as a blocked mapping with the probe's result as a candidate and does not pick a
side. The asset design (§C) proposes representations for composite identity/path construction,
ordered fallback resolution with shared-file write exclusion, and a separate Project import record —
against the existing `asset-output-profile/3` fields, with no schema change implemented.

---

## A. REG2 committed in two slices

Source hashes of all 16 overlay files (and the inventory CSV) were verified against the REG2
receipt's §1 table before staging: every SHA-256 matched. The index was empty (`git diff --cached
--stat` = 0 lines) before each `git add`; only the named paths were staged; `git diff --cached
--check` passed; each commit used a message file.

| Slice | Commit | Tree (= `git write-tree` of the staged index, exported with `git archive` and tested) | Files |
|---|---|---|---|
| 1 `feat(canonical): register six measured gen7 adapters` | **`2ec9fdef45be51b363346d6538bbe4e1534c08e5`** | `09ebfd316762ba973514ae3009b05d7f5634e08d` | 13: the six adapter TOMLs, `target_batch_tests.rs` (A), `target_evidence.rs`, `legacy_xbox360_tests.rs`, `nhl15_ps3_tests.rs`, `snapshot_tests.rs`, `tests.rs`, `lib.rs` (M) — 7 187 insertions, 28 deletions |
| 2 `feat(canonical): represent logo and jersey asset families` | **`0251b57cf7311098f372534b755ef23f6268c061`** | `c9458ea3b56117ca997e8fb35091b831f90134a4` | 4: `assets-logos.toml`, `assets-jerseys.toml` (A), `asset_tests.rs` (M), `docs/receipts/data/c0-reg2-asset-mapping-inventory.csv` (A) — 961 insertions, 1 deletion |

Verification, each on the exported staged tree (external `CARGO_TARGET_DIR`; `QA_WRITE` /
`QA_EVIDENCE_ROOT` unset; `--locked --offline`):

| Check | Tree 1 | Tree 2 |
|---|---|---|
| `cargo test -p tdb-canonical` | **290 passed / 0 failed** (`log-c1-test.txt`) | **291 passed / 0 failed** (`log-c2-test.txt`) |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | exit 0 | exit 0 |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | exit 0, empty |
| `git rev-parse <commit>^{tree}` equals the tested tree | yes | yes |

Totals matched the expected 290 / 291 without any assertion change. After each commit the index was
empty and the 307 unrelated modified/untracked entries of the live tree (frontend, studio-domain,
studio-api, docs, …) were untouched; no path under `crates/tdb-canonical` or `defs/adapters` was
left staged or modified after slice 2. No manifest, lock file, roadmap, STATE document or receipt
was included; those remain outside the commits as instructed.

**REG2 receipt wording clarified** (`docs/receipts/c0-reg2-database-asset-registry.md` §6): the
Part C table now opens with the statement that the listed corpus/bridge/placement fields are
*reusable provenance primitives*, not an implemented baseline-diff/import workflow and not an
opaque-file retention mechanism — no importer, per-file comparison, Project import record or
retention store exists. §1 notes the two commit identities. (That receipt is untracked, as before.)

---

## B. C0-REG3 — `nhl10-ps3`, `nhl11-ps3`, `nhl12-ps3`, `nhl13-ps3`

### B.1 Fixture identities (measured through `tdb_savedata` / `tdb_core`; pinned by SHA-256 in the test modules)

| Target | Main DB / metadata sha256 | ROSTER fixture (`SYS-DATA`) | Container as decoded | Inner TDB |
|---|---|---|---|---|
| `nhl10-ps3` | `e87b8c80…` / `6c335a62…` — byte-identical to Xbox 360 NHL 10 (asserted on the bytes) | `anchors/ps3/nhl10/savefiles/BLUS303450200 - ROSTER/SYS-DATA`, 1 610 252 B, `ceba4776…` | `PS3RosterFile`, **word 1 → `RawV1`** (32-byte header, one checksum, no size words; write path experimental) | 1 610 220 B, `d2f68d6b…`, **25 tables** |
| `nhl11-ps3` | `461f8d64…` / `a83aae92…` — byte-identical to Xbox 360 NHL 11 | `…/nhl11/savefiles/BLUS305680200 - ROSTER/SYS-DATA`, 1 935 216 B, `c18a39f6…` | `PS3RosterFile`, **word 1 → `RawV1`** | 1 935 184 B, `77d2dcb3…`, 32 |
| `nhl12-ps3` | `94b31ff2…` / `ddaaff54…` — byte-identical to Xbox 360 NHL 12 | `…/nhl12/savefiles/BLUS307910200 - ROSTER/SYS-DATA`, 2 350 856 B, `28132930…` | `PS3RosterFile`, **word 2 + `DB` at 0x2C → `RawV2`**, unc 2 350 812 | 2 350 812 B, `70740440…`, 36 |
| `nhl13-ps3` | `64460a15…` / `c5b9994e…` — byte-identical to Xbox 360 NHL 13 | `…/nhl13/savefiles/BLUS310030200 - ROSTER/SYS-DATA`, 2 180 292 B, `afa16f8f…` | `PS3RosterFile`, **word 0 → `RawV2`**, unc 2 180 248 | 2 180 248 B, `c2cf144d…`, 36 |

The PS3 fixtures are different files from the Xbox 360 ones (other outer and payload hashes) with
the same directory figures on NHL 11–13; the twin-capacity case shows that provenance, not the
numbers, tells them apart. Every save table's population equals the main anchor's on all four
fixtures (asserted per fixture, never assumed). The main DB is packed on all four (no table with
`max_records ≠ current_records`).

### B.2 Capacities — each fixture's own directory (`max_records` / `current_records`)

| Target | `ubPc` | `ZBac` | `eGlu` | `dSvy` | `xmja` | `mHuy` | `fMVl` | `kOtt` |
|---|---|---|---|---|---|---|---|---|
| `nhl10-ps3` | 4533/3933 | 5688/4188 | 6219/3719 | 3999/3499 | 3999/3499 | 534/434 | 534/434 | 182/182 |
| `nhl11-ps3` | 5910/5310 | 7093/5593 | 7728/5228 | 5260/4760 | 5260/4760 | 650/550 | 650/550 | 242/242 |
| `nhl12-ps3` | 6898/5395 | 8283/5779 | 7853/5349 | 6324/4821 | 6324/4821 | 874/574 | 874/574 | 243/243 |
| `nhl13-ps3` | 6946/5412 | 8907/5874 | 10457/5446 | 6407/4875 | 6407/4875 | 839/537 | 839/537 | 248/248 |

All 32 blocks: `source = "roster-save-max-records"`, `measured`, provenance = fixture path + the
`capacities_are_the_observed_directory_of_the_committed_roster_fixture` case of the target's module
+ its `save-matrix.csv` row; every note states *not a demonstrated runtime limit*. The NHL 10 Xbox
360 adapter's audit-derived figures (7 tables) coincide with the PS3 fixture's on those 7 tables;
that coincidence is observed, not relied on, and the PS3 adapter cites only PS3 files.

### B.3 Semantics measured on each anchor

| Fact | `nhl10-ps3` | `nhl11-ps3` | `nhl12-ps3` | `nhl13-ps3` |
|---|---|---|---|---|
| Birth month/day stored ranges → origin | **1..=12 / 1..=31 → 1-origin; transform `identity`** (corroborated by `birthdate_encoding::nhl10_probes_one_indexed_on_both_platforms`, which lists this anchor) | 0..=11 / 0..=30 → `index-origin-0` | same | same |
| Component offsets day/month/year | 1400/1408/1304 | 1126/1134/984 | 1128/1136/984 | 938/954/768 |
| Companion partition (skaters/goalies), codes | 3499/434, {0,1,2,3}/{4} | 4760/550 | 4821/574 | 4875/537 |
| `ubPc.dXSB` declared range | **0..255 unsigned** | -1..254 offset-binary | same | same |
| Main-team encoding vs roster slot | **slot team AS-IS** (3439 rostered, all `same`, none `+1`) | slot team + 1 (4923; `same` = 1 adjacent-code case) | + 1 (4995; `same` = 2) | + 1 (5008; `same` = 0) |
| Un-rostered store 0 | 494 — **and so do the 28 rostered players on team code 0 (`kOtt` record 0, "Anaheim Ducks™")** → no absence code established | 387 (0 is absence) | 400 | 404 |
| Largest stored pattern / team keys | 151 / 182 | 210 / 242 | 211 / 243 | 214 / 248 |
| Handedness `GRkp` offset, histogram | 1412, {0: 2771, 1: 1162} | 1138, {0: 3580, 1: 1730} | 1140, {0: 3613, 1: 1782} | 958, {0: 3621, 1: 1791} |
| `team.name` `kOtt.YmkJ` offset; records 0 / 1 | **680**; "Anaheim Ducks™" / "Atlanta Thrashers®" | 360; same | 336; "Anaheim Ducks™" / "Winnipeg Jets™" | 336; same |
| Name fields `HdeP`/`QbMR` | 320 bits @ 336 / 656 | 320 @ 328 / 648 | 320 @ 328 / 648 | 200 @ 328 / 528 |
| Ratings on `dSvy` | `qPWY` `p_speed` @72 only (34 fields); **6-bit regime unresolved** | `qPWY` `p_speed` @84 (36 fields) | `qPWY` @340 (68 fields) | `bPyZ` `p_overall` @190, `qPWY` `p_speed` @172 (70 fields) |
| `overall` name search | **0 hits** across 86 tables / 1769 fields | 2 (`pickoverall` ×2) / 131 / 2514 | 2 / 133 / 2593 | 5 incl. `p_overall`, `g_overall` / 136 / 2843 |
| `team.name` read/edit/reopen | passed at 680 | passed at 360 | passed at 336 | passed at 336 |

**NHL 10's rating regime — a conflict between two committed sources, recorded, not resolved.**
`docs/audits/ratings-write-codec.md` scopes the 6-bit/offset-36 regime to "NHL 10 → Legacy / 11–21
(PS3/X360)" by name, and designates `studio_domain::view::sixbit_rating_offset` as the seam that
resolves the offset per database. That probe returns 36 only if some 6-bit field on `dSvy`/`mHuy`
stores a value > 49, else 50. On the NHL 10 PS3 anchor the maximum over all 24 `dSvy` and 21 `mHuy`
six-bit fields is **49** (`p_speed` spans 2..47), so the production probe resolves **50** there; on
the NHL 11/12/13 anchors it resolves 36 (values up to 63 stored). The `nhl10-ps3` adapter therefore
maps `player.rating_speed` as `blocked` with an unknown transform, a `derived` candidate of
`linear-offset 50` (premises: the measured maximum + the probe's rule), and a note that names both
sources and the disagreement. The named case
`the_six_bit_rating_regime_is_unresolved_on_this_anchor_because_the_audit_and_the_probe_disagree`
measures the maximum and asserts that both sources say what they are cited for.
`tests::a_target_supported_derivation_stays_usable` states the exemption explicitly for this one
adapter (blocked, candidate 50, no human range) instead of asserting offset 36 — that is the only
existing assertion touched, and it now asserts the measured conflict rather than a count.

**Finding (not repaired, per the package):** `defs/adapters/nhl10-xbox360.toml` (byte-identical
main DB) declares `player.rating_speed` offset 36 as `derived`, resting on the audit's by-name row.
The same measurement applies to it. Left as it is; recommended for a named correction package.

### B.4 Blocked and unknown, per target

| Item | Targets | State |
|---|---|---|
| `player.shoots` codes | all four | `blocked`; enum anchored on an uncommitted `NHL Legacy default.db`; histogram recorded as counts only |
| `player.team` parent (`kOtt.vfEq`) | all four | candidate (containment + name `team`); mapping `blocked` |
| `player.team` absence code | `nhl10-ps3` | **unknown** (pattern 0 = team code 0 = un-rostered); candidate `encoding = "none"` `asserted`, citing the ambiguity case; the other three establish `literal-code 0 = absent` as measured |
| `player.rating_speed` transform | `nhl10-ps3` | **unknown** (audit 36 vs probe 50); candidate `linear-offset 50` `derived` |
| `player.rating_overall` identity | `nhl10-ps3`, `nhl11-ps3`, `nhl12-ps3` | `[[omission]] unknown-field-identity` with the search recorded (NHL 10: zero hits anywhere) |
| RawV1 write path | `nhl10-ps3`, `nhl11-ps3` | read only here; adapters say "WRITE path for this container is experimental"; `save-matrix.csv` rows `experimental-roundtrip` asserted |
| Twin capacity relabel case | `nhl10-ps3` | not declared: the Xbox 360 NHL 10 adapter carries seven audit-derived capacities, so there is no fixture-measured counterpart to relabel from; the identity case is declared |

### B.5 Mechanics (reused, generalised — no module copied)

* `target_evidence::MainTeamEncoding { SlotTeamPlusOne, SlotTeamAsIs }`; `main_team_measurement`
  and `assert_main_team_declared` take it. Under `SlotTeamAsIs` the assertion set is the measured
  one: unsigned 0..255 storage, `index-origin-0` transform, sentinel **unknown** with an `asserted`
  candidate citing the case, parent candidate, blocked. The Legacy and NHL 15 modules pass
  `SlotTeamPlusOne`; their expectations are unchanged.
* `target_batch_tests::Expect` gains `birth_origin` (0 / 1, checked against the stored ranges
  `(o, o+11) / (o, o+30)` and the transform kind `index-origin` / `identity`) and
  `main_team_encoding`. `codec_order` asserts it only runs under the plus-one regime (it orders an
  established sentinel; NHL 10 has none) and NHL 10 declares an ambiguity case instead.
* `rawv1_write_is_experimental(e, save_matrix_row)` shared by NHL 11 Xbox 360, NHL 11 PS3 and
  NHL 10 PS3; `overall_name_search` accepts zero hits.
* Adapters generated by a throwaway generator (scratch only) reading every physical number from the
  target's own `docs/audits/data/schema/ps3-nhl1x.json` and asserting the probe's measurements
  against it (partition sizes, populations, key flags, metadata names, ranges); twin adapters
  assert no `anchors/xbox360/` path or `xbox360-*.json` rendering in their text.

### B.6 Coverage and counts

| Target | Mapped / total fields | Mappings / omissions | Established / unknown / candidate |
|---|---|---|---|
| `nhl13-ps3` | 10 / 2843 | 8 / 0 | 44 / 30 / 6 |
| `nhl12-ps3` | 9 / 2593 | 7 / 1 | 40 / 26 / 5 |
| `nhl11-ps3` | 9 / 2514 | 7 / 1 | 40 / 26 / 5 |
| `nhl10-ps3` | 9 / 1769 | 7 / 1 | **38 / 28 / 7** (team sentinel and speed transform each unknown-with-candidate) |

`tdb-canonical` cases 291 → **341**: `nhl13_ps3` 12, `nhl12_ps3` 12, `nhl11_ps3` 13, `nhl10_ps3` 13
(macro-declared runners + per-target cases). Fourteen adapters are now in `COMMITTED_ADAPTERS`
(both files); `MEASURED_ON_OWN_ANCHOR` = 12; sentinel-candidate total 27 → 37; every
target-bound snapshot check (`every_committed_adapter_agrees_with_its_anchor_snapshot`,
`registry_population_counts_the_legacy_adapter_apart`, contract/derivation/discriminator/omission
sweeps) iterates the 14.

### B.7 Exact uncommitted files (SHA-256 of working-tree bytes)

```
d4ef3e0f3e754f5ab051e4f2e6b8ab4837c14d2b2a6d8a9f8752d97049d91bbb  defs/adapters/nhl10-ps3.toml                        A   970 lines
67f6eaed375b78c7fbd29bb5b9b5cf71af788d279731dc1292f398e85d3d5daa  defs/adapters/nhl11-ps3.toml                        A   950
f8e741df504d8adc97fe24f46320786eaf27afe58358d961251d3d74ed9a1304  defs/adapters/nhl12-ps3.toml                        A   950
17a96a9ff42e6acf45d8b6e2aded64722eb456daa66f9b98b356f2c7e62b1c5d  defs/adapters/nhl13-ps3.toml                        A  1014
225e606e6a8460a535b577ef702572456cd26a85be246b697894817d3f4722a2  crates/tdb-canonical/src/target_batch_tests.rs      M  +704/−40 (four modules; Expect fields; runners)
8b8e2ee3618fc15d5e09b37a5c014fa6120f859fbfd3f787fd285fb27cf8238d  crates/tdb-canonical/src/target_evidence.rs         M  +91/−12 (MainTeamEncoding)
fe599cd5ca495e588a8d3c25c63043cf23d57c1b7f48073e7c330b3a8ed7687d  crates/tdb-canonical/src/legacy_xbox360_tests.rs    M  +2/−1 (encoding argument)
4c7c5deb8b295c6fb285c89493e14ecb6c3daf7eabea46ebec2985631dac57e1  crates/tdb-canonical/src/nhl15_ps3_tests.rs         M  +2/−1
716af9fbe9a8b393a2cafa3aa6a7d11a1ebe9f6ab4293891395b3c71cbf973f6  crates/tdb-canonical/src/snapshot_tests.rs          M  +5/−1  (14 adapters)
94046001f68b0cb0e3b26059a5624e48dfae886fe15a4b6fd318c63aa860e8cf  crates/tdb-canonical/src/tests.rs                   M  +50/−14 (14 adapters; NHL 10 regime exemption; 37 candidates)
```

Total: 6 files changed / 854 insertions / 69 deletions plus four new adapters. No manifest, lock,
`lib.rs`, asset profile or unrelated file. Live tree: HEAD `0251b57`, index empty, nothing staged.
Proposed boundary: one commit, `feat(canonical): register four measured PS3 adapters` (10 files).

### B.8 Verification in isolation

Candidate = `git archive 0251b57` + the 10-file overlay (manifest 1204 → 1208 files, the four
adapters the only additions); external `CARGO_TARGET_DIR`; `QA_WRITE` / `QA_EVIDENCE_ROOT` unset.

| Check | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **341 passed · 0 failed · 0 ignored** | `reg3/log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `reg3/log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `reg3/log-fmt.txt` |

Failures met and fixed on the way (none by weakening): a `derivation` key on a candidate (the
`Candidate` type carries `note`, not `derivation` — the premises moved into the note); the NHL 10
sentinel candidate cited only the ambiguity case where `assert_main_team_declared` requires the
main-team case (now cites both); `a_target_supported_derivation_stays_usable` asserted offset 36 for
every adapter (now states NHL 10's measured conflict explicitly, §B.3).

---

## C. Asset design proposals (design only — nothing implemented, no schema extension written)

Read first: `crates/tdb-canonical/src/asset.rs` (`asset-output-profile/3`) — `IdentityValue { key,
source, ambiguity }`, `PathFamily { id, template, tokens ⊂ PATH_TOKEN, case, bucket_resolution,
observed_buckets }`, `PathRuleValue { families, family_selection }`, `RuntimeLookupValue { source,
patch_precedence, evidence }`, `InstallValue { kind, size_policy, repack, backup,
updates_references }`, `Corpus`, `ProvenanceBridge`, `AssetVariant.selectors`,
`AssetCandidate` (must carry a value). Committed examples used below: `assets-logos.toml`
(`fe-crest` = `fe/ion/artassets/teamlogos/t{team-id}.big`, `rink-texlib` =
`rendering/logo/texlib_{team-id}_0_0.rx2`), `assets-jerseys.toml` (`outfit-texlib` =
`rendering/jersey/texlib_{variant}.rx2` with `{variant}` standing in for the composite kit id;
`sock-loose`; `fe-thumbnail` = `fe/ion/artassets/jerseys/jersey_{variant}.big`), the xjgw row
(`mCXg` team, `cVLR` style, `GDQL` teamartid, `FccT` variant) and `kOtt.lOnr` (art id ≠ team key)
from the schema snapshot and `identities.csv`. Working-tree-only code (NOT at HEAD, establishes
nothing): `studio_domain::assets::jersey_thumbnail` (chain `jersey_{style}_{art}_{variant}.big` →
`custom.big` → `customhome.big` → `customaway.big`, with a `stock: bool` flag), `clone_kit_textures`.

### C.1 Composite identity / path construction from named database fields

**Gap restated.** `IDENTITY_KEY` already has `composite` and `IdentityValue.source` is free text, so
an identity can *say* it is composite; but `PATH_TOKEN` is closed
(`player-slug | numeric-id | team-id | variant`) and a `PathFamily.template` can only substitute one
of those, so no template can spell `texlib_{style}_{art}_{variant}` — the jerseys profile stands the
whole composite in as `{variant}`, which is representation, not a rule anything could execute.

**Proposal — a declared composite id, then templates reference it as one token.** Two additions,
both inside existing structures where possible:

1. *Where the composite is built:* on `[identity]` (the block that already owns `key = "composite"`
   and `source`). Add to `IdentityValue` an optional `[identity.value.composite]` table:

   ```toml
   [target_profile.identity.value]
   key = "composite"
   source = "xjgw"                       # the table; parts name the fields
   ambiguity = "reject"
   [target_profile.identity.value.composite]
   format = "{style}_{art}_{variant}"     # the id text, parts in braces
   [[target_profile.identity.value.composite.part]]
   name = "style";   table = "xjgw"; field = "cVLR"; render = "decimal"
   [[target_profile.identity.value.composite.part]]
   name = "art";     table = "xjgw"; field = "GDQL"; render = "decimal"
   [[target_profile.identity.value.composite.part]]
   name = "variant"; table = "xjgw"; field = "FccT"; render = "decimal"
   ```
   `render` closed vocabulary: `decimal` (no padding), `zero-padded:<n>`, `lower-hex`, `slug`
   (the existing `tdb-core::dev_key` slug for names — the portrait families' `player-slug` is the
   one case already in the contract). Fields resolve through the target adapter (`target-adapter/2`
   `[[table]]` ids), so a part on a table the adapter does not declare is a validation finding
   (extend `Rule::AssetOperandsIncomplete`, no new rule).

2. *Where the composite is used:* keep `PATH_TOKEN` closed but let a template's token set include
   `composite-id` — the rendered `[identity.value.composite].format` — so `outfit-texlib` becomes
   `template = "rendering/jersey/texlib_{composite-id}.rx2"`, `tokens = ["composite-id"]`. One new
   vocabulary entry, no new structure; a template using `{composite-id}` while `[identity]` is not
   established with a composite is a finding.

**Missing-value behaviour.** A part whose stored value is the field's established `null_sentinel`
(adapter fact) or whose row is absent makes the id **unconstructible**: the operation reports
`BlockedByUnknownConstraints` for that instance with the part named; nothing is rendered as `0` or
empty. A part on a mapping whose sentinel is unknown (as every rating sentinel is today) cannot be
used in a composite at all — the profile validator refuses it, because "absent" would be
indistinguishable from a real 0. (This is exactly the NHL 10 `dXSB` situation of §B.3, and it is why
the rule is stated here.)

**Ambiguity / collision.** Two different rows rendering the same id (e.g. two xjgw rows with equal
`cVLR`/`GDQL`/`FccT`) is a *collision*; `ambiguity = "reject"` (existing vocabulary) refuses to bind
either; `manual-resolution` surfaces both with their row keys. A target whose committed snapshot
shows collisions cannot establish `[identity]` at `supported-target-constraint`; it may be
established at `inspected-corpus-invariant` for a corpus in which the rendered ids were verified
distinct, citing the counting case. For the Legacy Xbox 360 xjgw table (703 rows) distinctness of
`{style}_{art}_{variant}` is **not yet measured**; measuring it on the anchor is one named case.

**Evidence requirements to establish.** (a) the part fields' identity and layout from the snapshot
(available); (b) a named case that renders the composite for every row of the anchor and asserts
distinctness (not yet written); (c) a corpus in which at least one rendered id is matched to an
actual member path by the bridge — for the jersey family that is the shipping-archive member
location, which is **unknown** (FR14; `chunklzx` undecoded), so `path_rule` stays unknown even after
(a)+(b): the composite can be *established* as an identity while the *path* is not.

**Backward compatibility.** `composite` is optional under `[identity.value]`; existing profiles
(portraits with `key = "player-slug"`) are unchanged; `{variant}` stays legal as a plain token, so
the current jerseys profile keeps parsing and its stand-in note stays honest until the family is
re-declared with `{composite-id}`. `profile_version` bumps on the file that adopts it; the schema
string stays `asset-output-profile/3` if the additions are optional, `/4` if `PATH_TOKEN` growth
is treated as a vocabulary change (recommended: `/4`, because a `/3` consumer would not know
`composite-id`).

### C.2 Ordered fallback resolution with shared-file write exclusion

**Gap restated.** `PathRuleValue.family_selection` picks *one* family per asset and
`RuntimeLookupValue.patch_precedence` orders *layers* (base vs patch) — neither can say "try the
kit's own file, then a shared default", nor mark the shared default as something a replace must
never write. The working-tree `jersey_thumbnail` chain is the concrete example: `jersey_{id}.big`
(owned), then `custom.big` → `customhome.big` → `customaway.big` (shared by every kit without stock
art). That code's comment claims the game's menu does the same — a claim **not established** by any
committed record; the design below must not inherit it.

**Proposal — a `[[resolution_chain]]` on the target profile, referencing existing families.**

```toml
[[target_profile.resolution_chain]]
id = "kit-thumbnail"
purpose = "read"                              # read | write | read-write
[[target_profile.resolution_chain.step]]
family = "fe-thumbnail"                       # existing [[path_family]] id, tokens substituted
ownership = "owned"                           # owned | shared
[[target_profile.resolution_chain.step]]
family = "fe-thumbnail-custom"                # a family with a literal template (no tokens)
ownership = "shared"
[[target_profile.resolution_chain.step]]
family = "fe-thumbnail-customhome"; ownership = "shared"
[[target_profile.resolution_chain.step]]
family = "fe-thumbnail-customaway"; ownership = "shared"
```

* *Fallback order* is the step order; the first step whose resolved path exists wins for a `read`
  purpose. Steps are families, so each step's path is subject to the same `bucket_resolution`
  discipline as today (a `template-only` family with no tokens resolves by itself).
* *Shared versus owned* is the per-step `ownership`. `shared` means the file is referenced by more
  than one asset instance; it is a declared fact with evidence like any other (a corpus count of
  distinct ids resolving to the same path is the measurement — the bridge method
  `archive-member-manifest` already carries where a member sits).
* *Write exclusion:* an `[[operation]]` of kind `replace` / `import` / `install` resolves through a
  chain with `purpose = "write"` or `read-write`, and the validator refuses (`Rule::…`, new finding
  text under the existing `AssetOperandsIncomplete` rule family) any chain whose write path could
  land on a `shared` step: a write resolves **only** the first `owned` step, creating it if absent
  (the family's `install.kind` says how — `loose-file` in the extracted tree today, an
  `archive-*` kind once the member location is known), and never falls through. So replacing a
  kit's thumbnail for a kit that today displays the shared `custom.big` writes
  `jersey_{composite-id}.big` and leaves `custom.big` byte-identical; `InstallValue.backup` still
  applies to the owned file. This is the `stock: bool` of the working-tree code lifted into a
  declared, validated fact rather than a runtime flag.
* *Runtime lookup* stays what it is: `[runtime_lookup]` says where the runtime reads
  (`loose-file | archive-member | patch-member`) and with what precedence between layers; the chain
  says which *file* within a layer. They compose; a chain does not establish runtime behaviour, and
  a chain step is not evidence that the game consults that step — `RuntimeLookupValue.evidence`
  ("a trace, a disassembly, a load test — never a folder listing") remains the only thing that does.

**Evidence requirements.** To establish a chain at `inspected-corpus-invariant`: the families exist
with their templates (snapshot/corpus), and for each `shared` step a named case counts ≥ 2 distinct
ids resolving to it on the corpus; for an `owned` step, that at most one id resolves to each path.
To establish `supported-target-constraint` (i.e. "the game resolves it this way"): runtime evidence
per `RuntimeLookupValue.evidence` — none exists for any family today, and the working-tree comment
does not count. Until then, chains can be *represented* with candidates (each step carrying a
family that exists) and every dependent operation stays `BlockedByUnknownConstraints`, which is the
`/3` behaviour already.

**Backward compatibility.** `resolution_chain` is a new optional array; profiles without it behave
exactly as now (`family_selection` alone). A `replace` operation on a family that appears as a
`shared` step in any chain and in no `owned` position anywhere is a finding — that catches the
jersey-thumbnail case the moment a chain is declared, and nothing before. Same version note as C.1.

### C.3 External completed-project ingestion — the separate Project import record

The asset profiles describe *targets*; Project history does not belong in them. The import record is
a Developer-Mode Project artefact (`defs/canonical/project-schema.toml` territory, not
`asset-output-profile`). Outline of its content, each field with the existing primitive it reuses:

| Section | Content | Reuses |
|---|---|---|
| `[source]` | identity of the ingested material: kind (`complete-installation` / `changed-files-package` — a new closed vocabulary **on the record**, not on `CORPUS_KIND`), root hash manifest (path → sha256, size), build identification read from the copy's own headers (SFO / XEX / EB header), declared origin (free text, "as provided", never verified) | `Corpus { identity_source, build_identification, inspected_files, limitations }` |
| `[baseline]` | the matching vanilla baseline if one exists: target id, the attributable corpus id, its manifest; `none` allowed with the reason | `Corpus.attribution = attributable-to-target`, `ProvenanceBridge.method = byte-identical-to-archive-member` |
| `[[file]]` | one row per input file: path, sha256, size, **comparison** = `identical-to-baseline | modified | added | no-baseline`, **mapping** = `mapped(family, target, canonical id, variant) | unresolved`, **retention** = `retained-opaque | lifted(canonical-source role, retention)`, **reason** for unresolved (family unknown / identity unconstructible per C.1 / container undecoded / outside any declared discovery root) | `SOURCE_RETENTION`, `CanonicalSource.role`, `IdentityValue`, `DISCOVERY_SCOPE` |
| `[canonical]` | the Project entities created: one per mapped file/DB row with its canonical id and the source `[[file]]` rows it came from; DB rows through the target adapter's mappings with their qualification recorded per property (blocked properties are imported as *retained raw*, not as canonical values) | `target-adapter/2` mappings, `qualification` |
| `[unresolved]` | the retained-opaque set as a sub-manifest with the same per-file reason, kept byte-for-byte inside the Project so a later re-target can re-run mapping once the gap closes | — (new) |
| `[[portability]]` | per candidate target: for each mapped canonical entity whether the destination representation exists (`OperationReadiness` of the family on that target), which blockers apply (`BlockedByUnknownConstraints` reasons), and which unresolved files would carry over as opaque (never installed) | `OperationReadiness`, `[[operation]].requires` |
| `[reproduction]` | what would regenerate the source installation from the Project: the target, the baseline, the ordered operations, and the unresolved files copied back to their original relative paths; explicitly `not-reproducible` when any mapped file's install kind is unknown | `InstallValue`, `RuntimeLookupValue` |

Distinguishing *complete-folder* from *changed-files* input: with a baseline, a complete folder
yields `identical-to-baseline` rows for the untouched majority and the diff is derived; a
changed-files package has no `identical` rows and every file is `modified` or `added` *relative to
the declared baseline* — but `modified` needs the baseline member to compare against, so a
changed-files package without a resolvable baseline degrades every row to `no-baseline`, which the
record must show rather than infer "modified". Files under a discovery root the profile declares
but in a container Studio cannot open (Legacy Xbox 360 `chunklzx`) are `unresolved` with reason
`container-undecoded` and retained; they are the majority case for archive-shipped families today.

Model gaps this record closes (from REG2 §6): the two new vocabularies live on the record; the
per-file comparison outcome lives on `[[file]]`; Project-level provenance lives in the Project. Gap
that remains: **no importer** — this is a record design, and REG2/REG3 wrote no code toward it.

---

## D. Remaining blockers and recommended next batch

Blockers (all recorded in the adapters or above): `player.shoots` codes (no committed anchor for the
enum) on all 14 measured adapters; main-team parent unproven on all; NHL 10 absence code and rating
regime; `player.rating_overall` identity on NHL 10/11/12; RawV1 write path (NHL 10/11, both
platforms); `nhl10-xbox360` offset-36 declaration contradicted by the probe on its own bytes
(finding, not repaired); jersey/logo archive member location and `chunklzx`; xjgw composite-id
distinctness unmeasured.

Recommended next batch:

1. **C0-REG3-COMMIT** — commit the 10 REG3 files as one slice after review (expected 341).
2. **C0-REG4** — the remaining gen7 pairs on their own fixtures: `nhl15-xbox360` re-measurement
   (currently seven audit-derived capacities and candidate discriminator; a ROSTER fixture exists),
   `nhl10-xbox360` re-measurement including the ratings-regime finding, then NHL 07–09 Xbox 360
   (gen6 NHL 07 needs a different table model — schema snapshot first).
3. **C0-AS4 (design acceptance)** — Codex decides C.1/C.2: `/4` with `composite-id` + `resolution_chain`,
   or `/3` with optional additions; then a package that measures xjgw composite distinctness on the
   Legacy anchor and declares the thumbnail chain as candidates.
4. **Ratings audit reconciliation** — a named package that either extends the audit's regime table
   with the probe's rule per anchor (measured maxima for all 27 anchors) or corrects the probe;
   both `nhl10-*` adapters are updated by that package, not before.

## E. Limits honoured

Evidence infrastructure parked (no generators run, no `capability.csv` touched, no live
publication/migration). No game installation, headmesh work, mounted-archive inspection or outside
repository access. No pushes. REG3 uncommitted. No repair of existing adapters (the
`nhl10-xbox360` finding is recorded only). No speculative schema extension implemented; C.1/C.2 are
prose plus TOML sketches. Documentation promotes nothing: no target is game-verified.
