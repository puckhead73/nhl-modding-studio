# Handoff 17 (28 Sep) — Legacy Recompiled portrait / head preview read route: compact contract

Research lane, 28 September 2026, published 18:51 MDT (system clock). After-receipt-7 prompt, package 3 continuation (the next route
after [16](16-recomp-speech-read-contract.md)). Claim `research-q16-recomp-portrait-head-route`, written before work began.
- Read-only: archive directories, every member's 8-byte head, and a 24-member byte sample.
- Nothing launched or written. No application edit.

Machine-readable: [`data/r146-recomp-portrait-head-members.json`](data/r146-recomp-portrait-head-members.json) (`0f90898a…`). Probe:
`probes/r146_recomp_portrait_head_members.py` (`7022c3fe…`), one run.

## Plain-language summary

Recompiled keeps every player portrait and head inside three archives:
- large portraits in `nocache.big`;
- small portraits in `cache.big`;
- heads in `cacherender.big`.

Each file appears once. Every sampled file matches the Xbox 360 Legacy copy. Studio already has a reader that unpacks these archives
(`read_eb_member`), so the preview only needs to look inside the archives when the loose folders are missing.

## 1. Measured

| Class | Member path | Archive | Members | Wrapper | Duplicates | Sample = X360 Legacy loose |
|---|---|---|---|---|---|---|
| large portraits | `fe/ion/artassets/playerheads/<bucket>/p<N>.big` | `nocache.big` | 9 228 (9 224 + 4 silhouettes, as the 18 Sep handoff 30) | stored (BIGF packages, RefPack inside) | 0 | 6 / 6 |
| small portraits | `fe/ion/artassets/playerheadssmall/<bucket>/p<N>.big` | `cache.big` | 9 229 | stored | 0 | 6 / 6 |
| head bundles | `rendering/player/player_<N>.rx2` | `cacherender.big` | 904 | **chunklzx** | 0 | 6 / 6 |
| head textures | `rendering/player/texlib_<N>.rx2` | `cacherender.big` | 685 | **chunklzx** | 0 | 6 / 6 |

- **Head pairing:** 219 head bundles have no `texlib_<N>`, and 0 texlibs lack a bundle. A preview must report "no texture library"
  for those rather than fail.
- No other `rendering/player/*` members exist.
- Only the 24 sampled members are byte-compared. The whole-archive identity with X360 Legacy is R128's separate evidence (09 K9).

## 2. Contract (developer)

*(Amended 1 Oct 01:40, [18](18-after-receipt8-corrections.md) K21.) "Missing in that archive means not shipped" holds only for a readable complete index. Any unreadable archive (receipt 67: `gamedata/gps.big`, `gamedata/anim/faceposer.big`) makes absent ids unknown. `read_eb_member` decodes and does not validate duplicates; a separate index pass rejects conflicting duplicates within and across archives. The provenance line states the selected source. "Identical to X360 Legacy" is shown only after the current bytes are hashed against a reference. R146's 24 samples are not a census.)*

- **Title identity, roots and order:** as [16](16-recomp-speech-read-contract.md) §2. `legacyrecomp` with `Platform::Recomp`; install
  folder or `game` folder; loose first; member route only when the loose `fe` / `rendering` trees are absent.
- **Paths:** portrait bucket rule unchanged (`AssetRoots::portrait_path`, `assets.rs:420`: `p0_4000` / `p4001_8000` / `p8001_12000`,
  silhouettes). Head file names as `mesh_path` / `texture_paths` (`assets.rs:312`, `:384`).
- **Archive per class:** as §1. Missing in that archive means "not shipped". An unreadable archive means "unknown". Keep these apart,
  as the receipt-65 review asks for containment.
- **Unwrap:** reuse `read_eb_member` (`assets.rs:1509`; `tdb_archive_big::eb::decode_payload` handles chunklzx and chunkzip; an
  unsupported LZX form refuses). Texture decode through `decode_archive_member_texture` (`assets.rs:1541`). The portrait's inner
  BIGF → RefPack → XPR2 chain is unchanged.
- **Consumers:**
  - `portrait_preview` (`registry.rs:417`);
  - `head_preview` (`:681`);
  - `texture_preview` (`:608`);
  - the containment card (12 §2) can reuse the same archive index for present / missing, with unknown kept for unreadable sources.
- **Provenance line:** "archive member `<archive>:<path>`; bytes identical to the X360 Legacy file where sampled (R146) and by archive
  identity (R128); what the recompiled game draws is not observed".
- **Not included:** portrait or head **replacement** in archives (a write route; out of this contract).

## 3. Smallest regressions

**Positive:** on the registered install, attached both ways:
- portrait `p<N>` for one bucketed id previews the same pixels as the X360 Legacy loose file;
- one bundle with a texlib, and one of the 219 without, preview or report correctly.

**Negative:**
- an id with no member reports "not shipped";
- a root without the archives reports "unknown";
- a wrong title is refused;
- the X360 Legacy loose tree is unchanged (loose first).

## 4. Same finding for handoff 16

`read_eb_member` is the established unwrap to reuse for the speech route too. The chunklzx gap in 16 §3 lies in
`gen7_speech_preview::bank_from_archives`'s private unwrap, and `gen7_sfx_banks::unwrap_member` shares it. The latter is harmless for
Recompiled sound banks, which are stored, but should not be reused for chunklzx members.
