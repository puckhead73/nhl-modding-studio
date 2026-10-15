# Handoff 20 (1 Oct) — Legacy Recompiled equipment-preview read route: nine slot folders as archive members; shared meshes live in `renderboot.big`

Research lane, 1 October 2026, published 01:48 (system clock). After-receipt-8 prompt, third package. Claim
`research-q19-recomp-equipment-read`, written before work began.
- Read-only: archive directories, every member's 8-byte head, and a 36-member in-memory byte sample.
- Nothing launched, extracted to disk or written. No application edit.
- **Preview read route only.** Hockey Shop install / sync on Recompiled stays refused (receipt 67 §2). No equipment write follows.

Machine-readable: [`data/r148-recomp-equipment-members.json`](data/r148-recomp-equipment-members.json) (`37ea0ad5…`). Probe:
`local/runs/overnight-research-2026-09-28/probes/r148_recomp_equipment_members.py` (`98e60593…`), one run.

## Plain-language summary

- All the gear files Studio's equipment previews read (sticks, gloves, helmets, skates, blocker, trapper, pads, goalie masks) are
  inside two Recompiled archives. Each file appears once, the names match Xbox 360 Legacy exactly, and every sampled file matches
  once unpacked.
- **The important detail:** the textures are in `cacherender.big`, but the shared 3D shapes are in `renderboot.big`: the common
  glove, skate, stick, tape, cage, blocker, trapper and pad meshes. A preview that looks only in the first archive would find the
  colours but no shape.
- Today, on Recompiled, the equipment list comes back empty. The 3D preview says "no decodable 3D mesh … in the install's rendering
  tree". The files are there, inside archives; Studio just doesn't read them yet. Nothing is written wrongly.
- The contract keeps Studio's existing filename rules exactly as they are, including the shared shapes they deliberately leave out.
  Research assigns no meaning to the numbers in file names.

## 1. Fully compared versus sampled

| Scope | Evidence |
|---|---|
| Whole-archive identity of `cacherender.big` / `renderboot.big` with X360 Legacy | R128 (earlier) |
| Every member's wrapper, path, duplicates; name set versus the X360 Legacy loose folder | **full, now** (R148) |
| Bytes, member versus X360 loose | **sampled**: 4 per folder, 36 in total, all equal. No full per-member equality is claimed |
| A later or modified install | not covered: history for this install (18 K21) |

## 2. Measured members (registered install `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game`)

Every member is **chunklzx**. There are **0** duplicate paths within or across archives, and every folder's name set **equals** the
X360 Legacy loose folder's.

| Folder (slot) | Members | `cacherender.big` | `renderboot.big` (what) | Stem shapes (digits → N; no meaning assigned) |
|---|---|---|---|---|
| `playerstick` (sticks) | 83 | 74 | 9: `playerstick_N_N` 4, `playersticktape_N_N` 4, `texlib_N` 1 | `texlib_N` 75, `playerstick_N_N` 4, `playersticktape_N_N` 4 |
| `goaliestick` | 86 | 78 | 8: `goaliestick_N_N` 4, `goaliesticktape_N_N` 4 | `goaliestick_N_tm` / `_dm` 38 + 38, `goaliestick_goaliestick_tm` / `_dm` 1 + 1, `goaliestick_N_N` 4, `goaliesticktape_N_N` 4 |
| `glove` | 578 | 575 | 3: `glove_N` (the shared meshes) | `glove_N_N_tm` 159, `feglove_N_N_tm` 159, `glove_N_N_dm` 53, `feglove_N_N_dm` 45, `glove_N_nm` / `_am` / `_sm` 53 each, `glove_N` 3 |
| `helmet` | 394 | 348 | 46: `helmet_N_N` 21, `chinstrap_N_N` 21, `texlib_N` 1, `visor_N_nm` / `_rm` / `_sm` 1 each | `visor_N_N_N` 90, `helmet_N_N_N_dm` 86, `helmet_N_N` 54, `chinstrap_N_N` 54, `jawprotector_N_N` 45, `texlib_N` 20, `helmet_N_N_tm` 19, `padding_N_N` 17, `visor_N_cm` 5, … (13 shapes) |
| `skate` | 51 | 32 | 19: `playerskate_N` 3, `goalieskate_N` 3, `skatelace_N` 3, `skatetongue_N_N` 9, `texlib_N` 1 | `texlib_N` 33, `skatetongue_N_N` 9, `goalieskate_N` 3, `playerskate_N` 3, `skatelace_N` 3 |
| `blocker` | 1 304 | 1 296 | 8: `blocker_N_N` 6, `blocker_N_am` 2 | `blocker_N_N_N_tm` / `feblocker_N_N_N_tm` 380 + 380, `_dm` 136 + 136, `blocker_N_N_sm` 136, `_nm` 128, `blocker_N_N` 6, `blocker_N_am` 2 |
| `trapper` | 1 296 | 1 288 | 8: `trapper_N_N` 6, `trapper_N_am` 2 | the same pattern: `_tm` 380 + 380, `_dm` 132 + 132, `_sm` / `_nm` 132 each, `trapper_N_N` 6, `trapper_N_am` 2 |
| `goaliepad` (pads) | 654 | 650 | 4: `goaliepad_N` 3, `goaliepad_am` 1 | `goaliepad_N_N_tm` / `fegoaliepad_N_N_tm` 191 + 191, `_dm` 67 + 67, `goaliepad_N_nm` / `_sm` 67 each, `goaliepad_N` 3, `goaliepad_am` 1 |
| `goaliehelmet` (masks) | 599 | 575 | 24: `cage_N_N_N` 9, `helmet_N_N` 9, `throatprotector_N_N_N` 6 | `goaliehelmet_N_N_cm` 539, `helmet_N_N` 24, `cage_N_N_N` 15, `throatprotector_N_N_N` 9, `texlib_N` 8, `goaliehelmet_N_am` / `_nm` 2 each |

**Total 5 045 members.** Sample rule: per folder, the two smallest members, then two at one-third strides. The full hashes are in
the JSON (`folders.<slot>.sample`), and every sample has `equals_x360_loose: true`.

**Role tokens, recorded as filename facts only:**
- map suffixes `_tm` / `_dm` / `_cm` / `_nm` / `_sm` / `_am` / `_rm`;
- the `fe` prefix (`feglove`, `feblocker`, `fetrapper`, `fegoaliepad`);
- `texlib_`;
- un-suffixed mesh containers;
- the companion prefixes `playersticktape`, `goaliesticktape`, `playerskate`, `cage`, `visor`, `chinstrap`, `jawprotector`,
  `padding`, `throatprotector`, `skatelace`, `skatetongue`, `goalieskate`.

Research infers **no** id, hand, model, style, LOD or thumbnail meaning from any number in these names.

## 3. Current consumers (source reading, working tree of 1 Oct; not run)

| Consumer | Behaviour on a Recompiled root today |
|---|---|
| `list_equipment_slot_assets` (`equipment_slots.rs:407`; also `_in` `:571`) | `read_dir` of the non-existent `game\rendering\<folder>` fails, giving an **empty list**. An absent folder, an unreadable folder and an archive-held folder are not told apart |
| `resolve_equipment_piece` / `resolve_helmet_pieces` (`equipment_piece.rs:84`, `:159`; player equipment tab `registry.rs:894`, `:1185`) | `None`: nothing resolves |
| `slot_preview_pieces` → `equipment_item_preview` (`registry.rs:1630`) | the error "no decodable 3D mesh for {slot} id {id} in the install's rendering tree". That is the wrong class: the meshes ship, as members |
| `find_prefixed_mesh` (`equipment_piece.rs`) | reads each candidate whole and decode-checks it as a mesh. A member route must unwrap before that check |
| `thumbnail_prefix` (createplayer thumbnails) | **no current consumer** reads it (only spec fields). Excluded from this contract; handoff 11 found no data edge for gen7 thumbnails |
| Hockey Shop sync / plan / apply | refused for Recompiled (receipt 67) |

## 4. Contract (developer; preview read route only)

**Title and roots.**
- `legacyrecomp` with `Platform::Recomp`. Two root forms, resolved deterministically, refusing ambiguity (18 K21).
- The slot specs apply unchanged as `nhllegacy`'s (`for_title` has no Legacy variant). The archives are identical (R128).

**One listing source, two back ends.**
- The member route supplies the **same `(folder, file name)` list** that `read_dir` supplies on a loose tree, from the archive index
  under `rendering/<folder>/`.
- Everything after that is unchanged and shared, with **no second grammar**:
  - `match_stem_id`, the matchers, `classify_stem`, the borrow rule;
  - `find_prefixed_mesh`'s prefix and decode check;
  - `pick_color_texture`, `resolve_visor_mesh`.
- That preserves Studio's existing exclusions:
  - skates' shared `playerskate_*` stays outside the `texlib` matcher;
  - the goalie-mask matcher's `id_index 1` excludes single-token files;
  - `visor_*`, `chinstrap_*`, `jawprotector_*` and `padding_*` stay out of the helmet matchers;
  - the `fe…` variants stay as they are matched today.
- It also keeps the distinct piece / texture roles (mesh, map suffix, texlib, companion), because the stems are identical.

**Archive set.**
- All 13 top-level EB archives, with `renderboot.big` **required** for previews:
  - it holds every shared or companion mesh measured here (`glove_N`, `playerskate_N`, `playerstick_N_N` / tape,
    `goaliestick_N_N` / tape, `cage_N_N_N`, `helmet_N_N`, `blocker_N_N`, `trapper_N_N`, `goaliepad_N`);
  - a route limited to `cacherender.big` would list textures and silently lose every shared shape.
- Do not encode which archive holds what. List all archives, and refuse conflicting duplicates (0 measured).

**Status classes.**

| Case | Status |
|---|---|
| Readable complete index, stem present | present: `archive:member` |
| Stem absent, every archive listed | missing |
| Stem absent, any archive unlistable | unknown. Never an empty "no files" list presented as complete |
| No archives and no loose tree | the existing "attach" note, never "no decodable mesh" |

**Decode.**
- The shared unwrap (chunklzx here). A decode failure is an error naming the member. Never "stored", never "missing".
- Then the existing mesh and texlib decoders on the unwrapped bytes.
- Largest stored members: see the JSON (`max_stored` per folder). These are small containers; whole-member reads are acceptable.

**Loose policy.** Loose first is Studio policy (18 K21), with per-file fallback, and the source shown per file.

**Provenance.** Each piece names `archive:member`, wrapper and unwrapped SHA-256. "Matches X360 Legacy" may be shown only after a
verified hash.

**Not included:**
- equipment install, clone or replacement on Recompiled;
- thumbnails;
- PS3 mesh preview (still disabled);
- any meaning for numbers in file names.

## 5. Smallest regressions

**Positive** (registered install, both root forms; X360 Legacy loose tree as reference):
1. For each of the nine slots, the member-route `list_equipment_slot_assets` equals the loose-route list exactly: the same ids and
   the same relative files. This is a names-only comparison, cheap and complete.
2. 3D preview equal to the loose route (mesh and texture source hashes):
   - one glove id: its shape is borrowed from a `renderboot.big` `glove_N`;
   - one skate: texlib plus the shared `playerskate_N` from `renderboot.big`;
   - one goalie mask: a `goaliehelmet_N_N_cm` paint plus a `renderboot.big` `cage_N_N_N`;
   - one stick: texlib plus `playersticktape`.
3. The helmet list on Recompiled equals the loose list: the visor / chinstrap / jaw-protector / padding exclusions still hold.

**Negative:**
1. An owned synthetic EB with one corrupted chunklzx member: an error naming it.
2. An owned index **without** `renderboot.big`:
   - a preview whose shape lives there reports the shape missing or unknown;
   - it never borrows a different mesh, and never reports success with no shape.
3. One unlistable archive: unknown, never an empty "complete" list.
4. A root without archives: the "attach" note, not "no decodable 3D mesh".
5. Hockey Shop on Recompiled: still refused.
6. The X360 Legacy loose tree: unchanged.

## 6. Delivery state

- These three Recompiled contracts (arena 19, equipment 20, and portrait / head 17 as amended) are not implemented.
- The developer is under the **r67 verification freeze** (`docs/plans/development-20261001-verification-freeze.md`), which suspends
  the remaining Recompiled read routes until the user or coordinator dispatches work.
- These handoffs wait for review and for a post-freeze dispatch.
