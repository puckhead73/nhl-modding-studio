# C0-REG4-R1 — attribution and reproducible verification — receipt

**Package:** make the uncertain NHL 19 / NHL 20 PS4 attribution non-operative; remove
checkout-line-ending dependence from metadata verification; verify the actual (exact) candidate;
close the correction receipt. REG4 stays uncommitted.
**Baseline:** `c3f4181`, branch `feat/phase5-asset-tooling`; continues
[C0-REG4](c0-reg4-ps4-database-adapters.md).
**Date:** 2026-09-15.

**Result.** The committed provenance for `anchors/ps4/nhl19/` and `anchors/ps4/nhl20/` establishes
**neither** label (§A.1). Both adapters keep every measurement — as candidates under unknown blocks
— and establish nothing: every mapping is blocked with unresolved transforms, and regressions show
`human_range`, `scalar_editable`, `registry_population`, the capacity checks and the codec-order
helper yield nothing usable from them. Coverage is now **19 attributed adapters + 2 registered as
representation only** (not 21). Metadata XML pins are LF-normalised **text digests** with a stated
rule and LF/CRLF proofs; binary hashes stay byte-exact. The candidate is an exact export of HEAD
(no autocrlf) plus the same 16-file boundary, verified byte-for-byte against the blobs: **449
passed**, clippy `-D warnings` and rustfmt clean; the same boundary also passes on a CRLF export.
Index empty; nothing pushed.

---

## A. Attribution

### A.1 What the committed provenance establishes

| Evidence inspected | What it says | What it establishes about the title |
|---|---|---|
| `git log -- anchors/ps4/nhl19 anchors/ps4/nhl20` | one commit, `e30b596` (2026-06-27, "Add Xbox360 + PS4 anchors; verify TDB v6 extends across the matrix": "PS4 (nhl19/20/21 full, nhl18 saves-only)"); no per-title capture record | nothing: both folders arrived together, labelled by path |
| `git ls-tree HEAD` on both `databases/` folders | `nhl_game.db`, `nhl_game-meta.xml`, `eng_us-meta.xml`, `entry_draft.db`, `expansion_draft.db`, `initial_experience.db`, `threes_campaign.db` are the **same blob objects**; `nhl20` additionally carries `eng_us.db` and `proam.db` | the shared set is one capture's bytes stored twice; the two extra files are not attribution of the shared ones |
| `docs/audits/anchors-manifest.md` / `data/anchors-manifest.csv` | lists both hashes (`265e4a68…`, `c67f4bfd…`) without remark; "`proam.db` exists only for nhl20"; "`eng_us.db` missing for nhl19 & nhl21" | labels only |
| `docs/audits/roster-capacity.md` | "NHL 20 is identical to NHL 19 (same roster generation in the anchor set)" | an observation of the identity, not a source for either label |
| `docs/audits/data/schema/ps4-nhl19.json` / `ps4-nhl20.json` | identical renderings differing only in the `game` label | a generated label |
| `crates/studio-domain/tests/birthdate_encoding.rs` (lists both paths) | probes both files | a test reading a path |
| the database bytes; the committed saves' record streams (`RosterCustom0`, `Season0`, `ThreeOnThree0`) | no field or record names a version, build or title (searched the snapshot for version/build/year fields; read the JSON records of the saves) | nothing |

Conclusion: byte identity neither disproves nor establishes either label, and no committed
capture provenance exists for either folder. **No shipping identity is invented; neither anchor is
replaced.** Both targets are treated the same way.

### A.2 Representation (adapters `nhl19-ps4.toml`, `nhl20-ps4.toml`, regenerated)

* Every formerly established block is `state = "unknown"` with the measurement preserved in
  `[<block>.candidate]` — value, confidence **as measured on the bytes**, provenance — and a note on
  the unknown block stating the attribution gap (the demotion is a section-aware rewrite; no physical
  number changed). Blocks that were already candidates are unchanged.
* Every mapping: `qualification = "blocked"`, `read_transform = write_transform = "unresolved"`,
  `inverse_verified = false`, no `automated_case`, a `BLOCKED (C0-REG4-R1, attribution)` note.
* Capacities, companion rule and discriminator: candidates too. `coverage_declared.note` says
  "REGISTERED AS REPRESENTATION ONLY".
* Header: the provenance inspection above, the consequence, and — for NHL 20 — the Threes-save
  disagreement **recorded without a cause** ("a mode projection, a title update, an edit, or a
  different capture would each produce it, and none has been distinguished"). The "different
  database generation" statement is withdrawn from the adapter, the inventory and the REG4 receipt.
* Ledger: `(8 mappings, 0 omissions, 0 established, 74 unknown, 50 candidates)` on both.

### A.3 Regressions (`target_ps4_tests::{nhl19_ps4,nhl20_ps4}::the_committed_*_anchors_are_byte_identical_so_neither_attribution_is_established`, plus `tests::registered_representation_is_reported_apart_from_established_target_coverage`)

Through the existing canonical APIs, on the declared adapters: `validate_adapter` = `[]` (a valid
document); for every mapping `qualification == "blocked"`, `transform_unresolved`, `human_range ==
None`, `scalar_editable == None`, no block established; capacities / companion rule / discriminator
not established; `registry_population` → `established_constraints == 0`;
`main_db_capacity_discrepancies` reports all 8 capacities as "not established"; `decode_stored`
panics on the declared team mapping. A **test-only** `promoted()` copy (candidates lifted to
established) then passes the established-only mechanics — which shows the measurements are right
and that attribution, and only attribution, is what is missing. Every other runner (capacities,
misattribution, roster save, birth date, position, main team, codec order, handedness, ratings,
team name, ledger) re-measures the bytes and compares them with the **candidates**; the snapshot
cross-check (`snapshot_tests::discrepancies`) now also checks candidate physical blocks against the
anchor, so a wrong candidate would fail even though it is not operative.

Sweeps in `tests.rs` gained an `ATTRIBUTION_UNRESOLVED` set (the two files) and take the
"nothing established" branch for them: `MEASURED_ON_OWN_ANCHOR` is 17;
`an_unsupported_no_sentinel_claim_is_not_operative` counts four candidates on each (the measured
team sentinel among them, kept apart as `measured` and non-operative; total 58 → 60);
`a_target_supported_derivation_stays_usable`, `supported_existing_constraints_still_validate`,
`a_composite_property_names_every_field_it_spans`,
`a_cross_title_only_enum_is_a_candidate_not_a_derivation` and
`representation_completeness_is_reported_apart_from_registry_population` assert candidates / blocked
on them and are unchanged for the other 19. The new
`registered_representation_is_reported_apart_from_established_target_coverage` reports the split
explicitly: `["nhl19-ps4", "nhl20-ps4"]` registered-only, 19 with established constraints.

Retained from REG4: seven-target scope; main-DB vs save capacity distinctions (blocked on NHL 15,
missing on NHL 20/21, decoded on 16–19); blocked handedness and main-team parent on all seven; the
C1 finding that the engine's team-name resolver reads only `fullname`.

---

## B. Metadata verification without checkout dependence

**Rule** (`target_evidence::text_identity_sha256`, documented on the function): SHA-256 over the
bytes after every CRLF pair (`0x0D 0x0A`) is replaced by LF (`0x0A`); nothing else is touched (a
lone CR, a BOM, whitespace, every other byte count). Two representations agree under it exactly
when they are the same text modulo CRLF/LF line endings. It is **not byte identity** and is never
reported as such; binary anchors (databases, saves) keep `read_pinned` (byte-exact).

* `read_pinned_text(rel, expected_text_sha256)` / `read_pinned_text_at(path, ..)` pin a text file
  by this digest and return the bytes as read.
* Pins renamed and recomputed: `Expect.main_meta_text_sha256` (10 gen7 modules in
  `target_batch_tests`, 7 in `target_ps4_tests`) and `nhl15_ps3_tests::MAIN_META_TEXT_SHA256` — 18
  pins, each now the SHA-256 of the committed LF blob (which is what the rule yields for the LF
  form). The twin comparisons state "text-identical (LF-normalised digest)" for the XML and
  compare the checked-out bytes separately.
* Proofs (`target_evidence::line_endings`, 3 cases on `anchors/ps4/nhl15/databases/nhl_game-meta.xml`,
  pin `2baa3399…`): LF, CRLF and mixed representations all yield the pin; the LF form's byte hash is
  the pin and the CRLF form's is not; a substantive change (`firstname` → `firstnamX`) fails in
  either representation; a lone CR is content and fails; **disposable LF and CRLF copies written to
  a scratch directory** both pass `read_pinned_text_at`, a changed copy is refused.

**Minimum repair:** only the 18 pins above and the reads through them changed; the eight gen7/PS3
pins were the checks that would otherwise refuse the exact (LF) candidate. **No boundary
expansion** — every touched file was already in the REG4 boundary (`nhl15_ps3_tests.rs` included).
Anchors, `.gitattributes`, and the evidence-freshness machinery are untouched.

**Why the earlier candidate had CRLF:** it was produced by `git archive HEAD` on this checkout,
where `core.autocrlf=true`; `git archive` applies the working-tree conversion, so 649 text files
(the metadata XMLs among them) were exported CRLF while their blobs are LF. That is why the REG4
pins — byte hashes of the *checked-out* XMLs — matched there and would not match an exact export.
The R1 candidate is exported with `git -c core.autocrlf=false -c core.eol=lf archive HEAD`.

---

## C. Verification of the actual candidate

**Exact-tree checks** (the deliverable): candidate = exact export of `c3f4181` (1 211 files, every
one verified byte-for-byte against its blob **before** overlay, 0 mismatches) + the 16-file boundary
(→ 1 219 files); fresh `CARGO_TARGET_DIR`; `QA_WRITE` / `QA_EVIDENCE_ROOT` unset; no manifest or
lock change.

| Check | Result | Log (scratchpad `reg4/`) |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **449 passed · 0 failed · 0 ignored** (445 + 3 line-ending cases + 1 representation-vs-coverage case) | `log-r1-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-r1-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-r1-fmt.txt` |
| non-boundary files vs HEAD blobs, **no normalisation** | 0 changed · 0 missing · 0 extra | (inline) |
| boundary files vs working tree | all 16 byte-identical | §D |

**Checkout-portability checks** (separate, not the deliverable): the same boundary over the CRLF
(`autocrlf`) export: **449 passed · 0 failed** (`log-r1-test-crlf.txt`); and within the suite, the
disposable LF / CRLF copies of one metadata XML (§B).

Failures met on the way, none fixed by weakening: the NHL 20 header still said "database
generations" (rewritten to the cause-free wording, and the runner now asserts its absence); a
`nonminimal_bool` clippy lint.

---

## D. Corrected boundary (SHA-256 of working-tree bytes; A = new, M = modified vs `c3f4181`)

```
375f0e4adca02d93a21bed7170cb912c7626e0ca1d848a8b87d65b2617321525  crates/tdb-canonical/src/target_ps4_tests.rs        A  2114 lines
105d12e57936f17b0f0828d08df2e9780cd27cc13c4716fb1ce279d358b681c1  crates/tdb-canonical/src/target_evidence.rs         M  +477/−29
6c0a15e2b506d9d4f9c1e26090110f826f1105ca0aeaa2e7cb702f5e82d09248  crates/tdb-canonical/src/tests.rs                   M  +213/−21
bf509b375a25f4c9ba2c03e68701c161815590eef22c33e64bcb9d2b49df343e  crates/tdb-canonical/src/target_batch_tests.rs      M  +67/−69
a55183e4c4149a52b404bb2940ffad3a3d7917a50f927debceb19f713d465ed0  crates/tdb-canonical/src/snapshot_tests.rs          M  +21/−6
541d0a62f29e9f96a5d946f4b7047ed7632da31384f8e1b7dcf75122eb268806  crates/tdb-canonical/src/legacy_xbox360_tests.rs    M  +9/−6
5051ea00c2217e524ec4061726fc5d19ea59c15f30538fe62f07e4039eaa772c  crates/tdb-canonical/src/nhl15_ps3_tests.rs         M  +12/−6
34b9cbcb24d8954463b038e501a679b96d6e33976121de538c7590de49e7c513  crates/tdb-canonical/src/lib.rs                     M  +2/−0
fe56ca3e65adf08e2057b4e7403bb56b65c835f5fe51cdf09d1e5be88653e5d8  defs/adapters/nhl15-ps4.toml                        A  1057 lines (unchanged since REG4)
ddcf7de08e3e603d6257ed0325a26f4a7aec11ef45b66a2f21ab5d51b7b51fd5  defs/adapters/nhl16-ps4.toml                        A   988 (unchanged)
401ce2f11e473a2988e7d67c60f5ab9cc0ae04b073a01a4d719eee62a7f45391  defs/adapters/nhl17-ps4.toml                        A  1057 (unchanged)
9940b601dfd7f744cce50cc0ca36167ebf410514a7f610a0b517186d3e43061e  defs/adapters/nhl18-ps4.toml                        A  1057 (unchanged)
ed5da10c9492d17bc55b75cba386555688bc5b4405721ecc12cc6a8bfba49301  defs/adapters/nhl19-ps4.toml                        A  1181 (demoted: attribution unresolved)
34a2b9aabb71fdeb41b2ab588d527735927a8ef652980352dfe5ae6c34b9c8b5  defs/adapters/nhl20-ps4.toml                        A  1176 (demoted; Threes wording corrected)
ef985ffd79d8f891055b9a4de35c8893d31804c2077b89f49ac0e83b1e18359e  defs/adapters/nhl21-ps4.toml                        A  1047 (unchanged)
9bd4ee38581d6ab7f22ad4dd65e91f518d7db555789b15dd5a6d1aa080f2a419  docs/receipts/data/c0-reg2-asset-mapping-inventory.csv  M  +9/−2  (nhl19/nhl20 rows: measured, attribution unresolved, representation only)
```

Membership unchanged from REG4 (16 files; no expansion). **Correction (C0-REG4-CLOSE):** this table originally listed the inventory CSV at its REG4 hash `75c3aad3…`; the R1 row corrections to the CSV were made after the table was computed, so the R1 candidate's copy of the CSV was the REG4 one (the file is not read by any test; results unaffected). The hash above is the corrected working-tree file's, and the CLOSE package re-verified the exact staged tree containing it. Documentation outside the boundary
(untracked as before): the REG4 receipt (superseding note, §A.2/§F.1/§E corrections), this receipt,
`docs/plans/project-completion-roadmap.md` and `docs/STATE-OF-THE-REPO.md` (coverage: 19 attributed
+ 2 representation-only; R1 pointer).

Live tree: HEAD `c3f4181`, index empty (`git diff --cached --stat` = 0 lines), nothing pushed,
unrelated modified/untracked entries untouched. Proposed boundary: one commit,
`feat(canonical): register seven PS4 adapters, two as unattributed representation` (the 16 files;
expected 449).

---

## E. Coverage, corrected

| | At `c3f4181` | If REG4 + R1 are accepted |
|---|---|---|
| Attributed partial adapters (established constraints) | 14 / 28 (12 measured on own anchor, 2 structural) | **19 / 28** (17 measured on own anchor, 2 structural) |
| Registered representation only (attribution unresolved, nothing established) | 0 | **2** (`nhl19-ps4`, `nhl20-ps4`) |
| Units with no adapter | 14 | 7 (`nhl07/08/09-xbox360`, `nhl08/09-ps3`, NHL 07 PS3 research copy, Legacy Recompiled) |
| Game-verified | 0 | 0 |

Registration is not support on any of the 21; the two unattributed files are not coverage of any
kind until an independent witness (a capture record, a title identifier in a save written by the
identified game, or a fresh attributed capture) attributes the bytes.

## F. Limits honoured

No anchor altered or replaced; no repository-wide attribute change; evidence-freshness behaviour
untouched; no new codec, schema version, importer or mounted-tree inspection; no shipping identity
invented; nothing committed or pushed; unrelated working-tree changes preserved.
