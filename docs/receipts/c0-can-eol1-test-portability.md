# C0-CAN-EOL1 — newline-independent canonical test construction

**Package:** C0-CAN-EOL1 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **HEAD at start and at end:** `0a791892dcfef11d079e873c2444d5e70bf29262` (unchanged) · **Index:** empty throughout

| | |
|---|---|
| **Result** | **Repaired and verified in both line-ending representations** |
| Live files changed | **2**, both test modules: `build_override_tests.rs`, `asset_tests.rs` |
| Canonical suite | **200 passed / 0 failed** in **LF** and **200 passed / 0 failed** in **CRLF** (was 184/7 under CRLF) |
| Scoped `clippy -D warnings` / `fmt --check` | exit 0 in **both** representations |
| Staged / committed / pushed | **none** — commit authorization is paused and was not exercised |

> No production Rust, no fixture, no `defs/`, no `Cargo.toml`/`Cargo.lock`, no `.gitattributes`, no
> git configuration, no dependency and no schema behaviour was touched. No live-tree build or test
> was run — every compilation, test, clippy and rustfmt invocation happened inside isolated scratch
> candidates. No game data was accessed. All prior scratch evidence is retained and verified intact.

---

## 1. Result in one paragraph

The seven failures were reproduced in the retained C0-CAN-COMMIT1 candidate, then traced to a single
mechanism: **`git archive`/checkout expanded LF blobs to CRLF, and the test anchors are written with
`\n`.** The repair normalizes CRLF to LF **only in the fixture-reading helper that builds textual
cases**, preserving lone CR, and replaces every silent `str::replace` on fixture text with a guard
that names its expected match count. Both changed files are test modules. The audit found the
vulnerability is confined to `build_override_tests.rs` — `tests.rs` has **no** mutation sites at all
and every `replace` in `asset_tests.rs` operates on inline Rust literals, which are LF regardless of
checkout — so the "845 / 1098 / 1459 escape-sequence" figures quoted in the previous receipt
described nothing about exposure and are corrected in §7. Ten focused cases now pin the behaviour,
and the fix was **temporarily neutralized in scratch** to prove they catch the defect: with `lf()`
stubbed out, all seven original failures return **plus four of the new cases**.

---

## 2. Conversion boundary — where the bytes actually change

One affected fixture, `crates/tdb-canonical/tests/fixtures/build-overrides/synthetic-approved-slot.toml`,
in all four representations. **Retained evidence was read, never modified**; the archive member was
extracted to a separate directory.

| Representation | Bytes | CR count | SHA-256 (first 16) |
|---|---|---|---|
| Live working-tree source | 1962 | **0** | `9f89ad98126dc81c` |
| Recorded staged Git blob (`caf9563d…`, still in the object DB from the C0-CAN-COMMIT1 staging) | 1962 | **0** | `9f89ad98126dc81c` |
| Retained archive member (`c0-can-commit1\staged.tar`) | 2019 | **57** | `af7ace4e2f734e32` |
| Extracted candidate | 2019 | **57** | `af7ace4e2f734e32` |

**Conversion happens at `git archive`, not at `git add`.** The blob is *byte-identical* to the live
file — this fixture was already LF, so staging normalized nothing. The `git add` warning
(`LF will be replaced by CRLF the next time Git touches it`) is a statement about **checkout**, and
that is exactly where the 57 CR bytes appear.

### 2.1 Observation vs expected behaviour, separated

`git archive` is not `git clone`, so the candidate demonstrated the archive path only. A separate
throwaway repository was built to observe the checkout path directly — LF blob, three commits' worth
of lines:

| Operation | CR count in the working file |
|---|---|
| Blob as stored | **0** |
| `clone -c core.autocrlf=true` | **3** |
| `clone -c core.autocrlf=false` | **0** |
| `clone -c core.autocrlf=input` | **0** |
| `git archive` under this machine's `core.autocrlf=true` | **3** |

So: **observed** — an LF blob becomes CRLF on checkout **iff** the consuming clone sets
`core.autocrlf=true`, and `git archive` matches that path exactly. **Expected, not separately
observed** — that Git for Windows' installer default is `autocrlf=true`, which is why this is the
common Windows case rather than a rare one.

### 2.2 Correction to the C0-CAN-COMMIT1 universal claim

That receipt said the suite "fails on **any** fresh Windows clone". Not demonstrated, and not true as
stated: a Windows clone with `core.autocrlf=false` or `input` receives LF and passes. The
demonstrated statement is **"fails on a checkout that expands LF to CRLF — i.e. any clone with
`core.autocrlf=true`, the Git for Windows default"**. A labelled correction is appended to that
receipt (§7).

---

## 3. Audit of the three modules — by call site, not by escape-sequence count

| Module | `read_to_string` | `.replace(` | `.replacen(` | File-derived text mutated by a **multi-line** anchor | Vulnerable? |
|---|---|---|---|---|---|
| `tests.rs` | 3 | **0** | 0 | — | **No.** No mutation sites exist. Its 81 `contains` calls are all on produced strings (`detail`, `note`, `derivation`, `property_paths`), none on file text |
| `asset_tests.rs` | 5 | 9 | 1 | **0** | **No.** All 10 mutate inline Rust literals or `format!` of them (`observed`, `two`, `derived`, `rule`, `sharing_only`, `scoped`), which are LF whatever the checkout. The one file-derived site, `adapter_text.replacen(…, 1)` at ~2254, uses a **single-line** anchor that matches under either representation |
| `build_override_tests.rs` | 2 | 7 | 0 | **6 sites** | **Yes — the entire exposure** |

The six vulnerable sites, all reading `fixture("build-overrides/…")`:

| Site | Form | Was it silent? |
|---|---|---|
| `record_from` helper | `assert!(contains)` then `replace` | Guarded — failed loudly (4 of the 7) |
| `no_override_record_can_name_a_canonical_source_field_to_change` | bare `text.replace(from, to)` in a loop | **Silent** — mutated nothing, then asserted the *unedited* document is refused |
| `an_unknown_field_anywhere_is_a_parse_error_not_a_silent_drop` | same | **Silent** |
| `a_support_or_acceptance_claim_has_no_spelling_in_an_override_record` | `text.replace` with `format!` | Anchor single-line; insert multi-line — matched, so it passed |
| `an_unknown_setting_or_a_mistyped_operand_is_a_parse_error` | bare `replace` in a loop | **Silent** |
| `a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint` (~1423) and the rule-loop at ~940 | `fixture(…).replace(from, to)` | Single-line anchors — matched, so they passed |

The three silent sites failed only because the unedited fixture happens to parse cleanly. **Had the
fixture been independently invalid, they would have passed while testing nothing.** That is the
hazard the guards close, independently of line endings.

---

## 4. The repair

### 4.1 `build_override_tests.rs`

Four helpers added, and every raw `str::replace` on fixture text routed through a guard:

| Helper | Contract |
|---|---|
| `fixture_raw(rel)` | The bytes exactly as they sit on disk. For cases where the bytes *are* the subject |
| `lf(text)` | Collapses `\r\n` to `\n` **and nothing else**. A lone `\r` is content and is preserved |
| `fixture(rel)` | `lf(fixture_raw(rel))` — for constructing textual cases |
| `replace_exactly(text, from, to, n)` | Asserts the anchor occurs exactly `n` times, then replaces. `#[track_caller]` |
| `replace_one` / `replace_present` | Exactly-one; and replace-all-requiring-at-least-one |

`lf` short-circuits when no `\r` is present, so the LF path allocates nothing extra.

**A semantic trap caught during verification.** The first attempt routed `record_from` through
`replace_one`. That broke
`an_observed_source_format_authorises_reproducing_it_and_nothing_else`, whose edit `("bc7","rgba8")`
legitimately matches **twice** in `synthetic-refused-codec.toml` and is meant to reach both. The
original helper's contract was *replace all, require ≥1*, and tightening it to exactly-one would have
silently narrowed an existing assertion. `replace_present` restores the original reach while removing
the silence, and a new case pins the two-site reach so it cannot narrow again. Recorded because the
verification matrix — not review — is what caught it.

### 4.2 `asset_tests.rs`

One change: the file-derived `adapter_text.replacen(…, 1)` gains a pre-assert that the anchor is
present and a post-`assert_ne!` that the document actually changed. Its anchor is single-line and was
never line-ending-vulnerable; the guard addresses the "silently does nothing" half of the audit. The
committed adapter is **still read raw and never normalized** there, deliberately — that case is about
what the parser accepts.

### 4.3 What was deliberately *not* done

- **`tests.rs` untouched.** No mutation sites, and its raw reads feed the parser real CRLF on a
  Windows checkout — genuine coverage that normalizing would have destroyed.
- **No production input normalized.** `parse_*` entry points still receive raw bytes everywhere.
- **Digests and fingerprints untouched.** `profile_content_digest` and `override_fingerprint` hash a
  canonical re-encoding built from the **parsed** structure (`Canon` buffer in `build_override.rs`);
  production code never hashes file text. Verified by reading the implementation, and now pinned by a
  test (§5). This is what made normalizing the fixture readers safe rather than merely convenient.
- **No assertion removed, no expected failure broadened, no unchanged fixture accepted.** The change
  is strictly additive in strictness: every previously-silent mutation now fails loudly.
- **No `.gitattributes`, git config, dependency, `Cargo.*` or schema change.**

---

## 5. New cases (10; suite 191 → 200 across the crate, `build_override_tests` 44 → 53)

| Case | Proves |
|---|---|
| `an_anchor_matches_in_both_line_ending_representations` | The same document in LF and CRLF yields the same parsed record and the same mutation; asserts the CRLF form genuinely *fails* to contain the raw anchor, so the case cannot silently stop testing the hazard |
| `a_mutation_anchor_that_matches_nothing_fails_explicitly` | `#[should_panic(expected = "occurs 0 times, expected 1")]` |
| `an_ambiguous_mutation_anchor_fails_explicitly` | A doubled document makes a one-site anchor ambiguous and fails |
| `a_stated_match_count_is_enforced_exactly` | `replace_exactly` honours counts other than one |
| `normalization_touches_crlf_only_and_preserves_a_lone_cr` | `a\rb` survives; `a\r\r\nb` collapses only the pair; an escaped `\r` inside a TOML string is untouched |
| `the_parser_accepts_raw_lf_and_raw_crlf_documents` | All four override fixtures parse identically in both forms; the profile parses from `fixture_raw` unnormalized |
| `digests_and_fingerprints_are_independent_of_line_endings` | Content digest and approval fingerprint are equal across representations **and** still equal the value the committed fixture carries |
| `a_record_from_edit_reaches_every_occurrence_and_still_refuses_a_silent_no_op` | Pins the deliberate two-site reach (§4.1) |
| `a_document_wide_edit_that_matches_nothing_fails_explicitly` | The ≥1 guard fires |

---

## 6. Executed verification

### 6.1 Scratch

`H:\c0-can-eol1\` — new, created by this package. Candidates `cand-lf`, `cand-crlf`,
`cand-crlf-neutralized`; logs in `logs\`; five external `target-*` directories. Each candidate is
`git archive HEAD` (**1160 files** = 1135 + the 25 overlay) with the 25 files written in one
representation and the `Cargo.toml`/`Cargo.lock` additions applied — **fresh copies of the named
files, never the dirty workspace**. Hooks inspected in both: **no `.cargo/config.toml`, no
`build.rs`, no toolchain pin**. `CARGO_NET_OFFLINE=true` and `--offline` throughout; every
`CARGO_TARGET_DIR` external.

Fixture representation confirmed per candidate: `cand-lf` 1962 bytes / **0 CR**; `cand-crlf` 2019
bytes / **57 CR** — the latter byte-identical to the fixture that failed in C0-CAN-COMMIT1.

### 6.2 Reproduction of the original failure

Re-run in the **retained, unmodified** `H:\c0-can-commit1\candidate`:

```
cargo test -p tdb-canonical --locked --offline
test result: FAILED. 184 passed; 7 failed
```

All seven names match the C0-CAN-COMMIT1 list exactly. Afterwards `tar -df` against that package's
staged archive reports **no** content, size or mod-time differences and all 27 recorded inputs still
hash as before — the evidence was read, not disturbed.

### 6.3 The repair, in both representations

| Check | `cand-lf` | `cand-crlf` |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **exit 0 — 200 passed, 0 failed, 0 ignored** | **exit 0 — 200 passed, 0 failed, 0 ignored** |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** | exit 0, **0 warnings** |
| `cargo fmt -p tdb-canonical -- --check` | exit 0 | exit 0 |

Diagnostics are all canonical-scoped; no dependency warnings appear in a `-p tdb-canonical` build.
No workspace check was run in this package — the previously recorded `tdb-rx2`/`studio-domain`
warnings are unrelated to it and unchanged.

**Formatting note.** `rustfmt` was run **in `cand-lf`, never against the live tree**, and the
formatted file copied back with LF preserved. The reformat touched only lines 1695+, inside the block
added at 1682; lines 1–100 (the new helpers) came back byte-identical, and `tests.rs`/`asset_tests.rs`
were untouched by it.

### 6.4 Neutralization probe — scratch only

`cand-crlf-neutralized` is a copy of `cand-crlf` with `lf()` replaced by `text` (a no-op), **marked
in-file as a scratch-only probe**. Live code was never mutated for this.

```
test result: FAILED. 189 passed; 11 failed
```

The eleven are **all seven original failures** plus four new cases —
`an_anchor_matches_in_both_line_ending_representations`,
`normalization_touches_crlf_only_and_preserves_a_lone_cr`,
`the_parser_accepts_raw_lf_and_raw_crlf_documents` and
`digests_and_fingerprints_are_independent_of_line_endings`. The regression is real: remove the fix
and the new tests say so.

### 6.5 Candidate integrity

Both candidates end at **1160 files** — membership unchanged, and **zero** writes under
`src-tauri/gen/` (the scoped `-p tdb-canonical` invocations do not trigger the Tauri codegen that the
earlier workspace check did). All 25 sources in each candidate remain content-identical to the live
files, representation aside.

---

## 7. Corrections carried into `c0-can-commit1.md`

A labelled §9 is appended there covering (a) the universal-Windows-clone overstatement of §2.2 above,
(b) that conversion occurs at checkout/archive rather than at `git add`, and (c) that its
"845 / 1098 / 1459" escape-sequence counts measured lines containing `\n`, not vulnerable cases — the
true exposure was **six sites in one module**. Everything else in that receipt stands: the staging
boundary, the 27-path scope, the withheld commit and the preservation evidence are unaffected.

---

## 8. Preservation and the revised candidate

### 8.1 Live tree

| Check | Result |
|---|---|
| HEAD | `0a791892…` — unchanged |
| Index | empty |
| Of the 25 canonical paths | **2 changed, 23 byte-identical** |
| Production sources (`lib`, `model`, `asset`, `asset_v1`, `asset_v2`, `build_override`, `v1`, crate `Cargo.toml`) | **all 8 unchanged** |
| All 10 fixtures + 4 `defs/` documents | **unchanged** |
| Root `Cargo.toml`, `Cargo.lock` | **unchanged** |
| `.gitattributes`, `core.autocrlf` | **unchanged** (`true`) |

### 8.2 The revised candidate — the earlier hashes no longer describe it

**C0-SLICE2 §2.3's 25 hashes are superseded.** Membership is unchanged (still 25 paths, none added or
removed); two hashes move:

| File | C0-SLICE2 hash | **Revised hash** |
|---|---|---|
| `crates/tdb-canonical/src/build_override_tests.rs` | `3cb34528559a15dc…` | **`59f84e9882b915f03daa7ce96f306b3b893fdf0c9e3278c8a10f467aaaa9180f`** |
| `crates/tdb-canonical/src/asset_tests.rs` | `1d926f92ef75e6a6…` | **`2bd9621d43631f89079d549c6a18b3288ca86842ffcc7f07e622b77f3d2898bd`** |

The other 23 keep their C0-SLICE2 values. Any future staging must re-verify against **this** table.
Test counts are now `tests.rs` 83 · `asset_tests.rs` 64 · `build_override_tests.rs` **53** = **200**.

### 8.3 Scratch evidence

`H:\c0-slice2`, `H:\c0-lock1`, `H:\c0-can-commit1` and `H:\c0-can-eol1` all present; nothing deleted.
The C0-CAN-COMMIT1 candidate is verified byte-identical to its staged archive after being re-run.

---

## 9. Unresolved questions

1. Is the repair accepted, and does commit authorization resume against the revised hashes of §8.2?
   The 27-path scope is otherwise unchanged.
2. Should the reproduction on **Linux/CI** be added before a commit? Everything here is Windows-only,
   and the LF candidate is the closest proxy — it is not a CI run.
3. `tests.rs` and `asset_tests.rs` deliberately read some committed documents **raw**, so they keep
   real CRLF parser coverage on Windows and none on Linux. Worth stating as intended, or should both
   representations be exercised explicitly there too?
4. Nothing here constrains how *other* crates read fixtures. A repo-wide convention — anchors never
   spanning newlines, or a shared test helper — is a larger question this package did not open.

---

## 10. Package boundaries observed

Edits confined to the two named test modules and this receipt, plus the labelled correction to
`c0-can-commit1.md`. No production Rust, definitions, fixtures, fixture policy, `Cargo.*`,
`.gitattributes`, git configuration, dependency or schema change. No live-tree build or test, no
staging, no commit, no push, no broad reset, no game-data access, and no deletion of scratch
evidence. **Stopping here for architect review.**
