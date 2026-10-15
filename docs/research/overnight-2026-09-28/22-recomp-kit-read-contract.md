# Handoff 22 (1 Oct) — Legacy Recompiled kit card read route: jerseys in `nocacherender.big`, pants and socks in `cacherender.big`, shared meshes in `renderboot.big`

Research lane, 1 October 2026, published 01:53 (system clock). After-receipt-8 prompt, continuation. Receipt 67 §4 lists kits /
uniforms as "not built". Handoff 12 §2 asked for "id alignment plus the member route". Claim `research-q21-recomp-kit-read`, written
before work began.
- Read-only: archive directories, every member's head, and a 12-member in-memory sample. Nothing launched or written; no application
  edit.

Machine-readable: [`data/r150-recomp-kit-members.json`](data/r150-recomp-kit-members.json) (`ee97db80…`). Probe:
`probes/r150_recomp_kit_members.py` (`68f92875…`), one run.

## Plain-language summary

- The kit card's files (jersey, pant and sock textures, and the jersey nameplates) are all inside Recompiled's archives:
  - jerseys and nameplates in `nocacherender.big`;
  - pants and socks in `cacherender.big`;
  - the shared shirt, pant, sock and tie shapes in `renderboot.big`.
- Each file appears once, the names equal Xbox 360 Legacy's, and the samples match byte for byte.
- Studio's existing kit filename rules accept the same files here as on Xbox 360 Legacy. No new rule is needed.
- Today Studio shows Recompiled kits as "not measured" and lists rows without textures. That is honest, just incomplete.

## 1. Measured (registered install)

Every member is chunklzx. There are 0 duplicates within or across archives, and every name set equals the X360 Legacy loose folder's.

| Folder (`KitPart::dir`) | Members | Archives | Shapes (digits → N) | Accepted by the existing single-file template |
|---|---|---|---|---|
| `jersey` | 1 883 | `nocacherender.big` 1 869; `renderboot.big` 14 (`jerseytie_N_N_N` 8, `shirt_N_N_N` 4, `texlib_N_N` 2) | `texlib_N_N_N` 905, `name_N_N_N_cm` 904, `jersey_N_N_N_N_N_N` 60, `jerseytie_N_N_N` 8, `shirt_N_N_N` 4, `texlib_N_N` 2 | `texlib_<s>_<a>_<v>` **905**; nameplate `name_<s>_<a>_<v>_cm` **904** |
| `pant` | 976 | `cacherender.big` 968; `renderboot.big` 8 (`pant_N_N_N_N` 6, `texlib_N_N` 2) | `texlib_N_N_N` 959, `pant_N_N_N_N` 15, `texlib_N_N` 2 | `texlib_<s>_<a>_<v>` **959** |
| `sock` | 981 | `cacherender.big` 954; `renderboot.big` 27 (`playersock_N_N` 12, `playersock_N_N_am` 8, `goaliesock_*` 4, `sock_N_nm` / `_rm` / `_sm` 1 each) | `sock_N_N_N_cm` 946, `playersock_N_N` 16, `playersock_N_N_am` 8, `goaliesock_N_N` 4, `sock_N_nm` / `_sm` / `_rm` 2 each, `goaliesock_N_N_am` 1 | single-file `sock_<s>_<a>_<v>_cm` **946** (the Legacy `SockGrammar::SingleFile`) |

**Template acceptance.** "Accepted" means the stem matches the template with no leading zeros, mirroring `KitPart::parse_stem`'s
round trip. These are research counts, not a run of Studio's code.

**The other shapes stay outside the templates**, as they do on the loose tree:
- shared meshes and companions: `shirt_*`, `jersey_*` meshes, `jerseytie_*`, `pant_*`, `playersock_*`, `goaliesock_*`;
- two-token `texlib_N_N`;
- sock `_nm` / `_sm` / `_rm`.

The existing `is_colorway_diffuse` rule (`playersock_*_am` is never a colourway; `registry.rs` tests) applies unchanged.

**Sample** (unwrapped SHA-256, first 16 hex; all equal to X360 loose; full values in the JSON):
- `jersey/shirt_0_0_0` (renderboot) `b59d7172…`; `name_0_326_2_cm` `e8c153c5…` (as R138); `name_1_11_3_cm` `9d5247ac…`;
  `texlib_1_391_0` `887210cf…`;
- `pant/pant_0_0_0_3` `78adb369…`; `pant_0_1_0_3` (renderboot) `6412828e…`; `texlib_0_321_0` `d79e953a…`; `texlib_0_327_0` `9b4b2b03…`;
- `sock/sock_0_93_4_cm` `0df679f3…`; `sock_0_90_2_cm` `0de7d352…`; `sock_1_21_5_cm` `79d1eb0a…`; `sock_1_10_0_cm` `ef3236d7…`.

Largest stored: jersey 2 721 908 B, pant 382 792 B, sock 540 256 B. Whole-member reads are acceptable.

**"Id alignment" (handoff 12 §2).**
- Recompiled's database member `cacheboot.big:db/nhlng.db` is the X360 Legacy anchor (R128, `105d2186…`), and the kit files carry
  the same names.
- So the `stockteamjerseys` triples `(style, art id, variant)` select the same stems on both.
- No separate id mapping is needed for this install. A user-edited database is joined by its own rows, as on X360.

## 2. Current consumers (source reading; not run)

- `kit_contract` (`kit_files.rs:142`): `legacyrecomp` hits the `Some(other)` arm and returns **`Unmeasured`**. Rows are listed
  without textures and nothing is called complete. Honest.
- `nameplate_measured` (`:179`) is false for `legacyrecomp`.
- **Edge case (as 19 §3):** an `nhllegacy` / Xbox project with the Recompiled root attached takes the Measured arm. Its folder listing
  (`:386`) sees the non-existent `game\rendering\jersey` and reports components as missing, although they ship as members. Not
  reproduced.

## 3. Contract (developer; read only)

**Title and roots.**
- `legacyrecomp` with `Platform::Recomp` is admitted as `Measured { sock: SingleFile, extension: "rx2" }`, the same as `nhllegacy`.
- `nameplate_measured` is true for it **only with** the member route.
- Two root forms, resolved deterministically, refusing ambiguity (18 K21).

**One listing source.**
- The archive index under `rendering/jersey|pant|sock/` replaces `read_dir`, with the same `(folder, file name)` list.
- `parse_stem`, `file_stem`, `nameplate_stem`, `is_colorway_diffuse` and the component statuses stay as they are: no second grammar.

**Archive set.**
- All 13 top-level archives. `nocacherender.big` holds the jerseys and nameplates; `renderboot.big` holds the shared meshes the 3D kit
  preview needs.
- Do not encode which archive holds what.

**Status classes.**

| Case | Status |
|---|---|
| Readable complete index, stem present | present: `archive:member` |
| Stem absent, every archive listed | missing |
| Stem absent, any archive unlistable | unknown |
| No archives and no loose tree | the existing "attach" note |
| A corrupt chunklzx member | an error naming it |

**Provenance** per component: `archive:member`, chunklzx, stored and unwrapped sizes and SHA-256. No runtime claim.

**Not included:** kit or nameplate replacement on Recompiled; the 3D kit preview's mesh resolver beyond listing.

## 4. Smallest regressions

**Positive** (registered install, both root forms; X360 Legacy loose tree as reference):
1. For a team with jersey rows, the kit card's components and statuses equal the loose-tree card: jersey, pants, socks and the
   nameplate.
2. Template counts on the member list equal those on the loose list (905 / 904 / 959 / 946).
3. One jersey texlib preview is pixel-equal to the loose route.

**Negative:**
1. A corrupted chunklzx `jersey` member: an error naming it.
2. One unlistable archive: unknown, not missing.
3. An `nhllegacy` / Xbox project with the Recompiled root (§2 edge): no "missing" for shipped members.
4. `playersock_*_am` is never selected as a colourway.
5. The X360 Legacy loose tree: unchanged.
