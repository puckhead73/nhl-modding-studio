# C0-POR1-R1 — Qualifying the portrait findings and making them reproducible

> **Package:** C0-POR1-R1 — correct provenance labelling, interpretation strength and scope in
> C0-POR1; reconcile the census; preserve the tooling; re-scope the profile promotions.
> **Date:** 2026-09-08. **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No source-game write, no installation test, no live
> publication, no new game-data search and no broader census. Reruns were limited to the supplied
> roots, read-only, and only where they were needed to substantiate or correct the existing report.
>
> Governs [C0-POR1](c0-por1-portrait-measurements.md) wherever the two differ. POR1 keeps its text
> and carries an appended corrections log (its §12) pointing here.

---

## 1. What was wrong, in one place

C0-POR1's measurements were largely sound. Its **labelling, interpretation strength and scope** were
not, and two of its totals were arithmetically wrong. Eleven corrections:

| # | C0-POR1 claimed | Correction |
|---|---|---|
| C1 | The PS3 tree is a "shipping layout"; 103 files per folder are "third-party-modified", the rest "vanilla" | An **observed installed copy**. Neither a `.bak` sibling nor `mipMapCount = 0` establishes provenance. Groups renamed **current / backup-labelled** and **base-level-only / mip-chain headers** |
| C2 | Decoded-pixel agreement shows the Xbox 360 tree "carries authentic EA content rather than re-encoded content" | **Withdrawn.** Agreement between two copies is evidence about the specimens compared, not about whether either tree was modified |
| C3 | The Xbox 360 chain was validated against "external ground truth" | **Withdrawn.** The two chains share three of their stages; only the Xbox 360-specific stages were exercised (§3) |
| C4 | Alpha is **straight** on the target | **Specimen observation** — 3 PS3 files, one decoder. Removed from the profile |
| C5 | PS4's 384 all-zero blocks "independently confirm" the 340 logical height | **Withdrawn.** Consistent with padding, but a blank image region encodes identically. Height is read from the descriptor |
| C6 | A linear surface would carry no padding, so PS4 must be tiled | **Withdrawn.** Linear surfaces can be pitch-padded. The only evidence is the `tiling_index` field |
| C7 | The id→bucket rule "is NOT the literal folder-name ranges" | **Withdrawn as a rule.** No bucket function was formulated or tested. Only **membership** was observed |
| C8 | "Two structural variants" | Scoped to the **BIGF member shape** alone. **15** distinct structural tuples once dimensions, mips, colour format and group are included (§4) |
| C9 | 36 930 Gen7 archives; 47 351 files | **37 120** archives accepted; **47 542** files (§4) |
| C10 | 14 established profile constraints across three targets | **3**, all on one target (§6) |
| C11 | Targeted tests were "complete coverage" | They are complete for the *reverse-dependency* question only. Not coverage of the findings (§8) |

C1, C2 and C10 are the ones that mattered. The rest are precision.

---

## 2. Provenance, restated honestly

### 2.1 The three corpora

| Corpus | What it is | What that permits |
|---|---|---|
| **PS3** | **Observed installed copy** — the title's own `USRDIR/fe/ion/artassets` layout, identity from its own `PARAM.SFO` | The only corpus attributable to its target, so the only one where anything is established |
| **Xbox 360** | **Derived tree** — `_compiled\` beside the title's shipping archives and a sibling `Extracted BIG Files\` | Nothing established. Not attributable to the shipped title |
| **PS4** | **Extracted tree** — the path is literally `databigs\Extracted\...` | Nothing established. Outer `.big` packaging unexamined |

"Observed installed copy" is deliberately weaker than POR1's "shipping layout". It is **not**
established to be pristine or complete: no shipping manifest was checked, and **`patch.big` — present
at `USRDIR` root — was never opened**, so nothing rules out that it shadows or supersedes the loose
files. That single unopened file is why `discovery` was demoted (§6).

### 2.2 The group labels were an inference presented as a measurement

POR1 called 103 archives per folder "third-party-modified" and the remainder "vanilla". Neither is
established. A `.bak` sibling tells us a *filename*, not who wrote either file or when. And the
mipmap case was **circular**: the "vanilla" group was *defined* by carrying no mip chain, and then
that group's mip count was reported as the finding.

Neutral vocabulary now used everywhere:

| Term | Meaning |
|---|---|
| **current** | extension `.big` or `.gnf` |
| **backup-labelled** | extension `.bak`. The label is the filename's; nothing establishes its content is original |
| **other** | neither — the one `.options` build artefact |
| **base-level-only header** | mip count 0, payload exactly one level |
| **mip-chain header** | more than one declared level |

What is *observed*, with no provenance attached: the PS3 copy holds **206 backup-labelled files** and
**206 current files with mip-chain headers**, against **18 251 current files with base-level-only
headers**. The two 206s are suggestive; nothing here connects them.

Exact game/build identity limits are unchanged and still visible: **build identification is
incomplete on all three copies.** A title id plus an SFO version field is not a build.

---

## 3. Interpretation strength: shared versus independent stages

POR1 said the Xbox 360 chain was "validated against external ground truth". It was not. The precise
account:

```
PS3   chain: parse_bigf -> refpack_decode -> parse_dds  -> decode_dxt5
X360  chain: parse_bigf -> refpack_decode -> parse_xpr2 -> swap16 -> untile -> decode_dxt5
                ^^^^^^^^     ^^^^^^^^^^^^^                                      ^^^^^^^^^^^
                            same code on both sides
```

| Stage | Origin |
|---|---|
| `parse_bigf`, `refpack_decode`, `parse_dds`, `parse_xpr2`, `parse_gnf`, `swap16`, `decode_dxt5` | written for this package |
| `xbox360_tiled_offset` | **transcribed from `crates/tdb-rx2/src/texlib.rs`** — not independent of this repository |

**What the comparison establishes:** for the 50 compared specimens, the Xbox 360-specific stages
(`parse_xpr2`, `swap16`, the transcribed tiling formula) yield the same DXT5 block stream the PS3
file holds directly. That is a real and useful result about those stages.

**What it cannot establish:** anything about a fault common to both chains, since three stages are
literally the same code. And nothing whatever about whether either tree is authentic, unmodified, or
the set the game ships — which is the use POR1 made of it, now withdrawn.

The separate observation that a decoded specimen "looks like a correct portrait" is a **visual
judgement**, not a comparison against a known reference. It is good evidence the shared stages are
not producing garbage; it is not a checksum.

**Comparison output, recorded exactly** (full detail in
[`data/c0-por1-specimen-hashes.json`](data/c0-por1-specimen-hashes.json)): seeded (seed 7) sample of
25 ids per folder, `p0_4000` bucket only. **50 compared, 49 identical, 1 differing.** The differing
file — `playerheadssmall/p127.big` — is a PS3 **current** file that has a backup-labelled sibling
whose block stream **does** match. Stated that way, with no provenance verdict attached to either.

### 3.1 Alpha, rescoped

Three PS3 specimens (`playerheads/p1008`, `playerheads/p1`, `playerheadssmall/p1008`) were decoded
with this package's DXT5 decoder; on semi-transparent texels a colour channel exceeds alpha by up to
253/255, which is inconsistent with premultiplied content **in those three files**. That is a
specimen observation by one decoder in one folder pair. The profile no longer carries a target-wide
alpha mode.

### 3.2 PS4 header fields, rescoped

Every PS4 geometry and format value is an **interpretation of T# descriptor bitfields**, using the
layout documented in `crates/tdb-rx2/src/gnf.rs`. **No PS4 pixel data was decoded.** `data_format 41`
→ BC7 is a **table lookup**, not a decode; `tiling_index 13` was never exercised.

The padding arithmetic (`180 480 − 256 = 180 224 = 128 × 88 × 16`) is **consistent with** a 512 × 352
padded surface carrying 340 logical rows. It is not independent confirmation of the logical height —
a blank image region encodes exactly like padding — and the claim that a linear surface could not be
padded is simply wrong, so it cannot be used to argue the surface is tiled either.

### 3.3 Buckets: membership, not a rule

Observed membership only:

| Bucket | Observed ids |
|---|---|
| `p0_4000` | 1 – 3999 |
| `p4001_8000` | 4001 – 7998 |
| `p8001_12000` | **8003 – 12401** |
| `silhouettes` | 60000 – 60003 |

That `p8001_12000` holds ids above 12 000 is an observation about membership. POR1 turned it into the
finding "the rule is not the literal folder-name ranges" — but no candidate id→bucket function was
ever formulated, let alone tested, so there is no rule to affirm or deny.

---

## 4. The reconciled census

Regenerated by `por1_census.py` into
[`data/c0-por1-census.json`](data/c0-por1-census.json), which is now the **single source** for these
numbers — they are deliberately not restated in the manifest as well.

### 4.1 Files, by folder, extension and group

| Corpus | Folder | Files | `.big` | `.gnf` | `.bak` | `.options` | current | backup-labelled | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| PS3 | `playerheads` | 9331 | 9228 | — | 103 | — | 9228 | 103 | 0 |
| PS3 | `playerheadssmall` | 9332 | 9229 | — | 103 | — | 9229 | 103 | 0 |
| X360 | `playerheads` | 9228 | 9228 | — | — | — | 9228 | 0 | 0 |
| X360 | `playerheadssmall` | 9230 | 9229 | — | — | 1 | 9229 | 0 | 1 |
| PS4 | `playerportraits` | 10421 | — | 10421 | — | — | 10421 | 0 | 0 |
| | **Total** | **47 542** | 37 114 | 10 421 | 206 | 1 | 47 336 | 206 | 1 |

### 4.2 Three counts that are not the same number

| Count | Value | Definition |
|---|---:|---|
| **Files inventoried** | **47 542** | every file present in the five folders |
| **Accepted by the inspection parser** | **47 541** | one exclusion: `p1112_glump.options`, which has no `BIGF` magic (first bytes `2d 62 69 67`, i.e. `-big`) — an EA build command line, not an archive |
| **Texture payloads found** | **47 541** | exactly one per accepted file, in every folder, with no file carrying two |

Gen7 archives accepted: **37 120** (18 663 PS3 including backup-labelled, 18 457 Xbox 360). POR1's
36 930 and 47 351 were both arithmetic errors.

> **"Zero parse failures" means accepted by the inspection parser** — these routines found the fields
> they look for at the offsets they expect. It is **not** semantic validation and does not establish
> that any file is well-formed for the game.

### 4.3 Per-subfolder

| Corpus / folder | `p0_4000` | `p4001_8000` | `p8001_12000` | `silhouettes` |
|---|---:|---:|---:|---:|
| PS3 `playerheads` | 3087 | 3196 | 3044 | 4 |
| PS3 `playerheadssmall` | 3089 | 3196 | 3043 | 4 |
| X360 `playerheads` | 3023 | 3173 | 3028 | 4 |
| X360 `playerheadssmall` | 3026 | 3173 | 3027 | 4 |

### 4.4 What "two structural variants" did and did not cover

POR1's "two structural variants, one per platform generation" described the **BIGF member shape
only**, while reading as though it covered the format. Split properly:

**Uniform** — all 37 120 Gen7 archives carry exactly 6 members in the pattern
`0, <num>, sg1, <num>, sg2, 1` with exactly 1 texture payload; zero exceptions in either group. (The
two numeric member names are per-archive APT resource ids; normalising them is what keeps this from
fragmenting into thousands of pseudo-variants. That normalisation is a choice of this tool and is
recorded as one.)

**Still varying, and every one a real variant:** dimensions (three co-existing Gen7 sizes), mip count
(base-level-only vs mip-chain, PS3 only), colour format (PS4 `num_format`), and PS4 `dst_sel_x`.

**15 distinct observed structural tuples** across the five folders — 13 Gen7, 2 PS4 — enumerated with
exact counts in the census file. Selected rows:

| Corpus / folder | Group | Texture header | Count |
|---|---|---|---:|
| PS3 `playerheads` | current | DDS 512×512 DXT5, mip 0 | 6465 |
| PS3 `playerheads` | current | DDS 512×256 DXT5, mip 0 | 2660 |
| PS3 `playerheads` | current | DDS 512×512 DXT5, **mip 10** | 100 |
| PS3 `playerheads` | current | DDS 512×512 DXT5, **mip 1** | 3 |
| PS3 `playerheads` | backup-labelled | DDS 512×512 DXT5, mip 0 | 102 |
| PS3 `playerheads` | backup-labelled | DDS 512×256 DXT5, mip 0 | 1 |
| PS3 `playerheadssmall` | current | DDS 256×128 DXT5, mip 0 | 9126 |
| PS3 `playerheadssmall` | current | DDS 256×128 DXT5, **mip 9 / 1** | 100 / 3 |
| PS3 `playerheadssmall` | backup-labelled | DDS 256×128 DXT5, mip 0 | 103 |
| X360 `playerheads` | current | XPR2 512×512, fmt 20, tiled, `0x0d10` | 6567 |
| X360 `playerheads` | current | XPR2 512×256, fmt 20, tiled, `0x0d10` | 2661 |
| X360 `playerheadssmall` | current | XPR2 256×128, fmt 20, tiled, `0x0d10` | 9229 |
| PS4 | current | GNF 512×340, fmt 41, tiling 13, **num_format 0** | 10 246 |
| PS4 | current | GNF 512×340, fmt 41, tiling 13, **num_format 9** | 175 |

Note what this makes visible that POR1's grouping hid: the PS3 mip-chain files sit in the **current**
group, and the backup-labelled files are a *separate* population — which is exactly why "vanilla"
could not be read off either.

---

## 5. Reproducibility

The findings were produced by throwaway scripts in a session scratch directory. They are now
preserved at **`docs/receipts/data/c0-por1-tools/`** with a README giving invocation and limits.

| File | SHA-256 | Bytes |
|---|---|---:|
| `por1_formats.py` | `679ebd69e608a3920f634b6541a5b5a688a3ddfcc4960247f7d390dc4834a760` | 12 997 |
| `por1_census.py` | `44f85315c1de95fbdac37552db9928a76c172913835f79d1a44dde3fddd032ec` | 8 848 |
| `por1_specimens.py` | `0e6f0f066656135235e18a3d65d197653db058ac605585962fcf93b19bacad2c` | 6 837 |
| `README.md` | `6c36a077efbf2d992e7327603a20fb368b8991e89b6deac2558fa773582e12ba` | 3 742 |

```sh
python docs/receipts/data/c0-por1-tools/por1_census.py    docs/receipts/data
python docs/receipts/data/c0-por1-tools/por1_specimens.py docs/receipts/data --compare
```

Python 3.12, no third-party dependencies. Each output embeds the SHA-256 of the scripts that
produced it, so a report can be tied to the tooling revision behind it.

**Private bytes stay outside git.** The three roots are hardcoded in the scripts so the run is
reproducible on that machine; the outputs carry counts, header field values, digests and at most one
example relative path per observed group. No game bytes enter the repository.

**Location is a proposal.** `docs/receipts/data/` for executable inspection tooling is my choice, not
an established convention. Redirect it if it belongs elsewhere.

**Reruns performed, and why each was needed:** the census once (POR1 never wrote its output to a
file, and two of its totals were wrong — §4); the specimen hashing and comparison once (to record
exact paths, digests and per-id outcomes rather than a prose summary). Both are read-only over the
supplied roots. No new search, no broader census, no additional specimens.

---

## 6. Profile promotions, re-scoped

### 6.1 The rule applied

A block is `established` only when it is an **inspected-corpus invariant** *and* that corpus is
**attributable to the target**.

* **Specimen observation** → never a target-wide constraint on its own.
* **Inspected-corpus invariant** → holds for every inspected file in that root, **across all
  groups**. A property holding only inside a group *defined* by that property is circular, not
  invariant.
* **Target requirement** → **nothing in this package reaches this level.** No load test, no
  acceptance test, no shipping-manifest check, on any target.

Attribution is where the two derived corpora fail. However exhaustively the Xbox 360 and PS4 trees
were measured, they are not attributable to their shipped titles — and POR1's argument that they were
rested on the pixel agreement now withdrawn (§3).

### 6.2 The result: 14 → 3

| Block | PS3 (POR1 → **R1**) | X360 (POR1 → **R1**) | PS4 (POR1 → **R1**) |
|---|---|---|---|
| `discovery` | established → **unknown + candidate** | unknown | unknown |
| `container` | established → **established** | candidate | candidate |
| `compression` | established → **established** | established → **candidate** | unknown |
| `atlas` | established → **established** | established → **candidate** | candidate |
| `tiling` | established → **unknown + candidate** | established → **candidate** | established → **candidate** |
| `mipmaps` | established → **unknown** | established → **candidate** | established → **candidate** |
| `dimensions` | unknown | unknown | established → **unknown + candidate** |
| `raster` | candidate → **unknown, candidate removed** | candidate → **unknown, candidate removed** | candidate → **unknown, candidate removed** |
| `identity` | candidate → **unknown, candidate removed** | candidate → **unknown, candidate removed** | unknown |
| **Established total** | **6 → 3** | **4 → 0** | **4 → 0** |

**Why the three PS3 survivors survive.** `container`, `compression` and `atlas` hold across **all
18 663 PS3 files — current and backup-labelled alike**. Surviving the third-party-written files is
precisely what makes them invariants rather than group-conditional claims: whoever wrote those files
also produced `BIGF` → RefPack → `DDS` with one texture payload.

**Why each demotion happened:**

* `mipmaps` (PS3) — **circular**, and the corpus is not invariant: 206 current files carry mip chains.
* `tiling` (PS3) — the corpus invariant is that the container is `DDS`; that the *payload* is linear
  rests on three specimen decodes. A DDS file can carry console-swizzled data, so the container does
  not settle it.
* `discovery` (PS3) — observed placement in one copy, with `patch.big` never opened. Not evidence of
  what the game loads.
* Everything on **X360 and PS4** — attribution, not measurement.
* `dimensions` (PS4) — the one genuinely uniform dimension in the family, demoted only because the
  corpus is an extracted tree. Kept as a candidate.

### 6.3 No fabricated values

POR1 filled bundled unknowns to make candidate values well-formed. Removed:

| Candidate | Fabricated field | Action |
|---|---|---|
| `raster` (all three targets) | `color_space` — absent from `DDS`/`XPR2`, **and non-uniform on PS4** (`num_format` 0 on 10 246 files, 9 on 175) | **Candidate removed.** Measurements preserved in the census |
| `raster` (PS4) | `alpha` — never measured on PS4 at all | same |
| `identity` (PS3, X360) | `ambiguity` — a Studio behaviour choice, not measurable here | **Candidate removed** |

A candidate is a proposal, not a hole with a plug in it. Where a bundled value cannot be completed
honestly, the block is `unknown` and the measurement lives in the manifest.

Two values were kept and are flagged rather than silently retained: `deterministic = false` on
`compression` (unchanged from C0-AS1's reasoning — reproducing EA's bytes was never tested), and
`variant = "portrait"` on the PS4 `dimensions` candidate, which is a **label naming the form, not a
measurement**.

**PS4 `compression` is a different kind of unknown** and deliberately carries *no* candidate: the
extracted files are stored decompressed, so the shipping payload codec is **unobservable** here. That
is an absence of evidence, not a measurement of `none`. A test asserts the candidate stays absent.

### 6.4 Schema changes: specified, not implemented

Per the package, **no schema change was made.** Required semantics for the next package:

**A — `dimensions` must express co-existing variants per target.** One target carries three portrait
dimensions, two of them inside one folder. `DimensionsValue.variant` documents an escape ("a target
carrying several declares several profiles") that **is not reachable**: two `[[target_profile]]`
entries for one target are refused by `Rule::DuplicateTargetProfile`, and a profile file is keyed
one-per-family by its filename. Needed: either a per-target *set* of dimension constraints each
carrying its own selector, or a legitimate way to declare several profiles for one (family, target).
The selector must be able to say *which* assets a variant applies to — path family alone is
insufficient, since 512×512 and 512×256 share a folder.

**B — `path_rule` must express one target's multiple path families, and an id-derived directory.**
`PathRuleValue` holds one template and `PATH_TOKEN` has no token for a directory computed from the
id. Needed: several path rules per (family, target), and either a token for a derived directory or an
explicit statement that such a rule is out of scope. Note this is needed *even if* the bucket function
is later identified — the two path families block it on their own.

**C — decide whether `RasterValue` should keep colour space bundled.** It is unmeasurable on Gen7 and
non-uniform on PS4, so as long as it is bundled, `raster` cannot be established on any portrait target
however well the codec is known.

---

## 7. Files changed

| File | Change |
|---|---|
| `docs/receipts/c0-por1-r1-evidence-scope.md` | this record |
| `docs/receipts/c0-por1-portrait-measurements.md` | **appended** §12, a labelled corrections log. Nothing rewritten |
| `docs/receipts/data/c0-por1-portrait-specimens.json` | revision 2 — corrected vocabulary and scope; counts removed in favour of the generated census |
| `docs/receipts/data/c0-por1-census.json` | **new, generated** — reconciled counts |
| `docs/receipts/data/c0-por1-specimen-hashes.json` | **new, generated** — specimen digests and full comparison output |
| `docs/receipts/data/c0-por1-tools/` | **new** — preserved tooling and README |
| `defs/adapters/asset-profiles/assets-portraits.toml` | re-scoped per §6; three-level evidence language throughout |
| `docs/validation/c0-05-fixture-runtime-register.md` | §5.7 corrected; FR13–FR16 restated, FR17–FR18 added |
| `crates/tdb-canonical/src/asset_tests.rs` | population tests updated to the corrected set |

---

## 8. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **162 passed, 0 failed** |
| `validate_asset_profile` on the committed profile | **zero findings** |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **clean** (strict) |
| `cargo fmt --all -- --check` (repo-wide) | **clean — 0 files with diffs** |
| `cargo test -p tdb-qa` | **215 passed, 0 failed** — family vocabulary and evidence ledger unaffected |
| Census regeneration | 47 542 files, 47 541 accepted, 1 excluded, 0 unexpected |
| Specimen re-verification | 22 named specimens, digests unchanged |

**Repo-wide fmt: HEAD versus working tree.** The clean result above describes the **working tree**.
CLAUDE.md records that `cargo fmt --all -- --check` and `cargo clippy … -D warnings` fail **at HEAD**,
because the C0-INT1 repairs that fix them live only in the working tree and cannot be committed
standalone. So this is not a claim that the HEAD baseline defect is resolved — it is untouched, and
nothing in this package repaired or depended on it.

**Coverage, stated correctly.** POR1 called its targeted test runs "complete coverage". They are not.
What is true is narrower: `tdb-canonical` is a leaf crate (no workspace crate depends on it, checked
against every `Cargo.toml`) and the only reader of `defs/adapters/asset-profiles/`, so running its
tests is sufficient **for the reverse-dependency question** — nothing else in the workspace can be
affected by these edits. It says nothing about whether the findings themselves are adequately tested;
they are documentary evidence, and no automated test exercises the portrait corpora.

**One further number corrected.** POR1 §10 reported `cargo test -p tdb-qa` as 207 passed. That total
was taken from a `head`-truncated listing that dropped a twelfth test binary; the correct figure is
**215**. Nothing about the result changes — all passed then and now.

**Preservation, stated correctly.** POR1 implied whole-corpus preservation from 22 specimen hashes.
The accurate statement: **22 named specimens were hashed before and after inspection and 22/22 were
unchanged.** The other 47 520 inventoried files were opened read-only but were **not** hashed before
and after, so no before/after claim is made about them. What covers them is the access mode — every
open in every tool is read-binary, and no tool has a write path to any source location.

---

## 9. Unknowns and next actions

| # | Unknown | Next action |
|---|---|---|
| 1 | `patch.big` (PS3 `USRDIR`) never opened — blocks `discovery` | Examine it; determine whether it shadows loose portraits |
| 2 | Xbox 360 / PS4 corpora not attributable to their shipped titles | Locate portrait members inside the shipping archives and compare |
| 3 | No PS4 pixel decode; BC7 and `tiling_index 13` unexercised | Decode one GNF specimen and check against a known image |
| 4 | Colour space unmeasured on Gen7, non-uniform on PS4 | Determine whether the 175 `num_format 9` files are a distinct class; decide gap **C** |
| 5 | `dimensions` schema gap | Architect decision on semantics **A** |
| 6 | `path_rule` schema gap | Architect decision on semantics **B** |
| 7 | id→bucket function unidentified | Establish it from the game's own lookup, not from folder names |
| 8 | `ubPc.lOnr` binding unexamined on all three targets | Establish with a citation naming these targets |
| 9 | Build identification incomplete on all three copies | Decide what constitutes a complete build id |
| 10 | RefPack determinism untested | Test whether the encoder reproduces EA's bytes |
| 11 | PS3 copy carries third-party writes; provenance of individual files unknown | Register as non-pristine; obtain a copy with known provenance |
| 12 | No target requirement established anywhere | Any load/acceptance testing is a separate, unauthorized step |

**Stop for Codex review. Nothing staged, committed or pushed.**
