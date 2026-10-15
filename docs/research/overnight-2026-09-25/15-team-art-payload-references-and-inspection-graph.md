# Handoff R104–R106 — in-payload references of team-art / kit members on NHL Legacy (X360 `.rx2`, PS3 `.rpsgl`), read with the RW4 container structure: 5 871 + 5 873 members parsed, 0 refusals; **no RW4 subreferences on any member** and the same three default external-arena entries everywhere; every member's **name dictionary (`0xEC0010`) names its own rasters with its own id tuple** (`jersey_/font_/pant_/sock_/name_/logo_/centerlogo_` + numbers) — an internal self-reference a rename would leave stale; no other-member stem and no djb2 path hash in the non-payload bytes (scoped negative); the "rink logo" family is a superset of the kit (style, variant) grid (candidate only); an inspection graph assembles the evidence and the open requirements — no writer contract

Research lane, 25 September 2026, 22:08 → 22:18 MDT (real clock). Review next task: "inspect in-payload references for team-art / kit
members on Legacy Xbox and PS3 … integrate into an inspection graph; do not promote a writer contract". Claims
`research-r104-team-art-payload-references`, `research-r105-logo-family-vs-kit`, `research-r106-team-art-inspection-graph`. Read-only
over the Legacy loose trees and EB archives on `X:`. No writer; nothing game-verified; no launch.

Machine-readable: [`data/r104-team-art-payload-references.json`](data/r104-team-art-payload-references.json) (probe
`r104_team_art_payload_references.py`), [`data/r105-logo-family-vs-kit-tuples.txt`](data/r105-logo-family-vs-kit-tuples.txt) (probe
`r105_logo_family_vs_kit_tuples.py`, text output), [`data/r106-team-art-inspection-graph.json`](data/r106-team-art-inspection-graph.json)
(probe `r106_team_art_inspection_graph.py`, derived from R102–R104 JSONs only).

> **Note 2026-09-26 01:07 (review [research-20260926-r102-r106-review.md](../../validation/research-20260926-r102-r106-review.md); [overnight-2026-09-26/00-checkpoint-r102-r106-review.md](../overnight-2026-09-26/00-checkpoint-r102-r106-review.md)):** two presentation corrections, no evidence changed.
> (1) "member → other members / arenas: established none" reads **"decoded subreference and atom tables are empty; the external-arena list
> holds three observed constant values (meaning not decoded); the string / hash search is a separately scoped negative"**; internal raster
> names remain a known dependency. Aliases are scoped: full-family byte comparison for nameplate and "rink logo" members (R103b); 12
> sampled pairs per family for jersey / pant / sock / centre logo (R104); other pairs unchecked. (2) The logo tuples are **not** a full grid
> and **not** a superset of the jersey tuples: Legacy X360 has 796 of 905 jersey tuples in common, 109 jersey-only, 1 130 logo-only. A larger
> overlapping family is established; "retired variants" and "per-kit logo" are hypotheses. The inspection graph was regenerated with this
> wording (`data/r106-…json`; the 2026-09-25 output kept as `data/r106-team-art-inspection-graph-v1-20260925T2217.json`). The open-requirements
> list is a research inventory, not a precondition set for a narrowly scoped offline proposal.

## Plain-language summary

The kit, nameplate and logo files on both NHL Legacy discs were opened with their actual container layout, not searched as raw bytes.
Each file has a small table of contents; in these files, the part of that table that would point at other files is always empty, and a
second part carries only three fixed default entries, identical everywhere. What each file does contain is a short list of names for
its own pictures — "jersey_0_12_1_cm", "font_0_12_1_nm" and so on — built from the same numbers as the file's own name. So nothing
inside these files points at another team's files, but each file's inside repeats its own team number, and a rename that changed only
the outside name would leave those inside names behind; whether the game ever reads them is unknown. A byte search of the non-picture
parts for other files' names or path hashes also came back empty, which only means "not found where we looked". Finally, the family
called "rink logos" turns out to cover roughly every style and colour variant of every kit, including retired ones, so what it is and
how it is chosen stays open. Everything is gathered into an inspection map listing what a rename would have to touch and what is still
unknown.

## 1. Container structure applied (RW4, big-endian; decoded on six members, then applied to all, refusing on mismatch)

```
header     0x00 "\x89RW4xb2\0" (X360) / "\x89RW4ps3\0" (PS3) | 0x20 u32 file_count | 0x30 u32 file_table_offset
file table file_count x { u32 offset | u32 0 | u32 size | u32 align | u32 type index | u32 type id }
           type ids: 0x10031 (X360) / 0x10030 (PS3) raw raster payload (offset relative to header size); 0x20003 raster descriptor (0x38 B);
           0xEC0010 name dictionary
manifest   "00010004" | u32 4 | u32 0xC | 4 x u32 section offsets (relative to the manifest)
           0x10005 types {count | 12 | 0 | type ids}: 10030 10031 10032 10033 (10034 PS3) 10010 20003 ec0010 10006
           0x10006 external arenas {count 3 | dict off | 0 | 04000000 ffb00000 04000000}
           0x10007 subreferences {count | ...}      0x10008 atoms {count | ...}
names      0xEC0010 { u32 count | u32 0x1505 | count x { u32 string abs offset | u32 type id (0x20003) | u32 object index } | NUL strings }
           object index -> the raster descriptor chunk it names (verified on every entry)
```

## 2. Results (`data/r104-…json`)

| Measurement | X360 Legacy | PS3 Legacy |
|---|---|---|
| members parsed (jersey / nameplate / pant / sock / "rink logo" / centre logo) | 905 / 904 / 959 / 946 / 1 926 / 231 = **5 871**, 0 refused | 905 / 904 / 959 / 946 / 1 928 / 231 = **5 873**, 0 refused |
| loose copy = unwrapped archive member (12 sampled per family) | 72 / 72 (chunklzx) | 72 / 72 (chunkzip) |
| subreferences (0x10007 count) | **0 on 5 871** | **0 on 5 873** |
| external arenas | `04000000 ffb00000 04000000` on 5 871 (fixed defaults) | same on 5 873 |
| atoms | 0 on all | 0 on all |
| name dictionary word 1 | `0x1505` on all | `0x1505` on all |
| names per member | 1 (2 082 members), 3 (2 857), 5 (28), 6 (901), 7 (3) | 1 / 3 (2 859) / 5 / 6 / 7 same shape |
| dictionary names whose numbers are the member's own tuple | **all** (jersey 5 428, nameplate 904, pant 2 933, sock 946, logo 5 778, centre logo 231); **0 naming another id** | all (logo 5 784); 0 other |
| name stems | jersey: `jersey_{s}_{a}_{v}_cm / _sm`, `jersey_{s}_{a}_{v}_{k}_nm`, `font_{s}_{a}_{v}_cm / nm / sm`; pant `pant_…_cm / nm / sm (/ rm)`; sock `sock_…_cm`; nameplate `name_…_cm`; logo `logo_{a}_{s}_{v}_{k}` (k = 0..n); centre logo `centerlogo_{id}_cm` | same |
| other team-art stems in non-payload bytes outside the dictionary | 0 | 0 |
| djb2 hashes of every team-art path / basename / stem found as big-endian u32 in non-payload bytes | **0** over 5 979 008 B (16.7 k distinct hashes; random expectation 23 — the bytes are structured, not random) | 0 over 5 999 500 B |
| FE crest `t{id}.big` | 318 BIGF containers; inner names a shared vocabulary (`0`, `1`, `sg1`, `sg2` in all 318; numbered pairs `2/3`, `8/9`, … `78/79` in a minority) — meaning open | same |

Readings:
- **No parsed cross-member reference exists in these members:** RW4 subreferences and atoms are empty and the external-arena list is
  the same three default entries on 11 744 members. Inside the stated scope (non-payload bytes), neither other members' names nor their
  EB path hashes occur. Raster payloads were not searched (a 4-byte match there is noise); that is a scoped negative, not proof.
- **Internal self-names are a real rename dependency:** the dictionary strings embed the member's `(style, artid, variant)` (or art id).
  A rename that keeps the bytes would leave internal names naming the old id; whether the engine resolves rasters by these strings, by
  object index or not at all is **open**. A rename contract must decide (keep / rewrite) and — if rewriting — the dictionary layout
  (string offsets, chunk size, alignment) becomes part of it.
- **Aliases:** every family has the loose copy (72 / 72 sampled per tree, and all nameplate / logo members in R103b).

> **Addendum 22:22 (R104b, `data/r104b-team-art-payload-references-nhl15.json`):** the same probe on X360 / PS3 NHL 15 (loose trees): 4 743 /
> 4 745 members, 0 refused; loose = unwrapped member on 72 / 72 samples per tree; subreferences 0, atoms 0, external arenas the same three
> defaults on every member; every dictionary name is the member's own tuple; 0 other-member stems and 0 djb2 hits in 4.84 / 4.86 MB of
> non-payload bytes. Coverage of this result: Legacy + NHL 15, both platforms (four trees, 21 232 members). PS3 NHL 12 / 13 (archive-only)
> and the other titles are not measured.

## 3. The "rink logo" family (R105, candidate)

`rendering/logo/texlib_{a}_{b}_{c}` normalised to (id, b, c) against the jersey files (id, style, variant): X360 Legacy 1 926 logo tuples,
905 jersey tuples, 796 common, 1 130 logo-only (669 on ids that ship jerseys, 461 on ids without); NHL 15 X360 1 784 / 683 / 567 / 1 217;
PS3 13 1 700 / 865 / 713 / 987. Logo-only tuples concentrate on (b, c) = (0 / 1, 0–5) and include variants the id no longer ships as jersey
files (id 1: logos for variants 0–2, jerseys 3–4). Internal names `logo_{a}_{b}_{c}_{k}` (k = 0..2+) — several rasters per member.
**Reading (candidate):** a per-kit logo set over the (style, variant) grid, retained for retired variants; the label "rink logo"
(handoff 04, from the 2026-09-18 join) is **not verified** and may be wrong. Discriminating next step (static): raster dimensions and
count per member vs the kit's crest placements, or a consumer reference — not attempted. Runtime card only once a hypothesis exists.

## 4. Inspection graph (R106; Legacy X360 / PS3) — what a re-key / rename would have to touch or check

| Edge | Status |
|---|---|
| `stockteamjerseys (style, teamartid, variant)` → jersey / pant / sock files | established (R56 per-row; developer resolver) |
| same triple → nameplate `name_{s}_{a}_{v}_cm` | naming association (R103, every row and every discriminating row); runtime selection unobserved |
| `exhibitionteams.artid` → "rink logo" `texlib_{artid}_{b}_{c}` | candidate (positions 2–3, family meaning) |
| `exhibitionteams.artid` → FE crest `t{artid}.big` | established (handoff 04) |
| archive member ↔ loose file | established alias (byte-identical after unwrap) |
| member → its own internal raster names (0xEC0010) | established self-reference; runtime use open |
| member → other members / arenas (RW4 subreferences, external arenas) | established none (defaults only) |
| member non-payload bytes → other team-art names / path hashes | scoped negative |
| EB name table → archive members (slot, row fields, offset order) | established (R102) |

**Open requirements (all must close before any writer contract is proposed):** same-length rename vs changed-length name-table rebuild;
header word `0x10`; runtime lookup key and caches; internal raster names (keep / rewrite; runtime use); loose-vs-archive precedence;
destination occupancy across the DB team-art namespace, loose-only files and other archives / titles; the "rink logo" meaning and slots;
FE crest inner vocabulary; raster payloads (unsearched); titles other than Legacy; PS4 (refused); transaction / recovery. The graph is
[`data/r106-team-art-inspection-graph.json`](data/r106-team-art-inspection-graph.json).

```
python local/runs/overnight-research-2026-09-25/probes/r104_team_art_payload_references.py docs/research/overnight-2026-09-25/data/r104-team-art-payload-references.json
python local/runs/overnight-research-2026-09-25/probes/r105_logo_family_vs_kit_tuples.py > docs/research/overnight-2026-09-25/data/r105-logo-family-vs-kit-tuples.txt
python local/runs/overnight-research-2026-09-25/probes/r106_team_art_inspection_graph.py docs/research/overnight-2026-09-25/data/r106-team-art-inspection-graph.json
```
Executed: r104 × 2 plus one interrupted run (the first run read every loose member in full; it was stopped for speed — its X360 half had
completed and showed a manifest-offset slip (external-arena words shifted by one, atoms unread) and crest inner names misread as ids;
both fixed; the JSON is the second complete run, reading 128 KB per member and full files only for identity samples, 72 full reads per
tree); r105 × 1 (first as scratch, promoted unchanged); r106 × 1. Two exploration scripts (header / manifest dumps) ran once and were deleted.
