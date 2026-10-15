# Handoff 16 (28 Sep) — Legacy Recompiled speech read route: an implementation-ready contract for the speech catalogue, name calls, contexts and speech preview; a chunklzx gap in the existing preview reader

Research lane, 28 September 2026, published 18:48 MDT (system clock). After-receipt-7 prompt, third package (the highest-value
missing Recompiled read route in [12](12-reconcile-after-receipt6.md) §2). Claim `research-q15-recomp-speech-route`, written before work
began.
- Read-only: archive directories plus member payloads of 16 MiB or less, and the X360 Legacy loose files, compared in memory.
- Nothing launched or written. No application edit; the developer owns the implementation.

Machine-readable: [`data/r145-recomp-speech-members.json`](data/r145-recomp-speech-members.json) (`d59eaa78…`). Probe:
`local/runs/overnight-research-2026-09-28/probes/r145_recomp_speech_members.py` (`0d8381ea…`), one run.

## Plain-language summary

- Every file the speech pages need is inside two archives of the Recompiled install (`audioboot.big` and `nocache.big`). None
  appears twice.
- Each small file is byte-for-byte the Xbox 360 Legacy file, once unpacked. The two large sound streams have the same size; their
  bytes were not compared.
- Most of these files are packed with **chunklzx**. Studio's existing speech-preview archive reader does not unpack chunklzx, and it
  also misses the install's `game\` folder. So the Recompiled play-by-play preview is expected to fail today.
- This is a reading of the code, not a reproduced failure. The contract below gives the developer the exact files, wrappers, folder
  layouts and tests.

## 1. Measured members (registered install `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game`)

**Totals:**
- 30 `audio/speech/*` and `audio/context/*` members: 9 in `audioboot.big`, 21 in `nocache.big`; none in the other 11 archives.
- **0 duplicate paths.**
- Wrappers: **20 chunklzx**, 10 stored.
- **28 / 28** members of 16 MiB or less are byte-identical to the X360 Legacy loose file after unwrapping.
- The 2 `.sbs` streams are stored, with size equal to the loose file (789 000 896 and 129 156 980 bytes), compared by **size only**.
- The archives' own identity (R128): `audioboot.big` `37b89e91…`, `nocache.big` `9ac38034…`, equal to the X360 Legacy archives.

**Files Studio's speech consumers name** (`speech_catalogue.rs`, `gen7_speech_preview.rs`):

| Member | Archive | Wrapper | = X360 Legacy loose |
|---|---|---|---|
| `audio/speech/pbp_eng_speecheventsubscr.bin` | audioboot.big | chunklzx | yes |
| `audio/speech/pbp_eng_speecheventdef.csi` | audioboot.big | stored | yes |
| `audio/speech/pbp_eng_sentencedata.bin` | audioboot.big | chunklzx | yes |
| `audio/speech/pbp_eng_graffitidata.bin` | audioboot.big | chunklzx | yes |
| `audio/speech/pbp_eng_samples.sbr` | audioboot.big | **chunklzx** | yes |
| `audio/speech/pbp_eng_keywords.bin` (optional in the catalogue) | nocache.big | chunklzx | yes |
| `audio/speech/ann_eng_graffitidata.bin`, `ann_eng_samples.sbr`, `ann_eng_speecheventsubscr.bin` | nocache.big | chunklzx | yes |
| `audio/context/pbp_eng_contexts.bin`, `pbp_eng_contexteventsubscr.bin` | nocache.big | chunklzx | yes |
| `audio/context/pbp_eng_contexteventdef.csi` | audioboot.big | stored | yes |
| `audio/speech/pbp_eng_samples.sbs`, `ann_eng_samples.sbs` (streams) | nocache.big | stored | size only |

## 2. Contract per operation (developer implementation; research does not implement it)

*(Amended 1 Oct 01:40, [18](18-after-receipt8-corrections.md) K21; accepted with amendments, receipt-8 review.) Byte equality is R145 history for this install, not a property of an attached source. Loose-first is Studio policy, not observed loading. Duplicates are refused within and across archives. Absent, unknown and corrupt are kept apart. A partial loose folder falls back per file. `.sbr` / `.sbs` must come from one coherent source. Streams stay windowed, never read through the allocating `read_eb_member`. 18 K21 also reads receipt 67's delivered route against these rules.)*

**Common rules:**
- **Title identity:** the open project must be `legacyrecomp` (alias `nhllegacy-recompiled`) with `Platform::Recomp`. The attached
  install must be detected by `detect_recomp`. A wrong title stays refused, as today.
- **Accepted root layouts:** the install folder (with `game\*.big`) **or** its `game` folder (with `*.big`). Search the archive list
  under `game/` first, then at the root, exactly like the existing `gen7_sfx_banks::layout` Recompiled arm (`gen7_sfx_banks.rs:106–113`).
- **Order:** keep **loose first**, so the X360 Legacy tree and any other loose title are unchanged. Use the member route only when the
  loose `audio/speech` (or `audio/context`) directory is absent.
- **Member lookup:** by exact lower-case path in the EB front table. Search archives in the order `audioboot.big`, `nocache.big`, then
  any other listed archive.
- **Duplicates:** none measured. If a path is found in more than one archive, refuse unless the unwrapped bytes are equal, and name
  both.
- **Unwrap:**
  - `stored`, or `chunklzx` via the existing `tdb_archive_codec::decompress_chunklzx` (window 2^17);
  - any other head, or a failed decode, **refuses**;
  - never pass chunklzx bytes on as "stored".
- **Missing:**
  - a required member absent from every searched archive gives the same error as the loose route, naming the archives searched;
  - `pbp_eng_keywords.bin` stays optional (it is present on this install).
- **Provenance:** each source row reports `archive:member`, stored size, wrapper, and unwrapped length / SHA-256. The UI can state:
  "bytes identical to the X360 Legacy file (R128 / R145); what the recompiled game plays is not observed".

| Operation | Consumer (at reading) | Members |
|---|---|---|
| Speech catalogue | `speech_catalogue()` `speech_catalogue.rs:146` (`resolve_install_path(..,"audio/speech")`, loose only, error at `:150`) | the 5 required pbp files + optional keywords |
| Team name calls | `team_name_calls()` `:358` | pbp events, graffiti, `.sbr`; keywords optional |
| Player name calls | `player_name_calls()` `:465` | pbp events, graffiti, `.sbr`; ann graffiti / `.sbr` / events (the announcer part is optional in the code) |
| Context catalogue | `context_catalogue()` `:648` | the 3 `audio/context` files + pbp speechdef / events |
| Speech preview (banks) | `bank_entries()` → `bank_from_archives()` `gen7_speech_preview.rs:276` | `pbp_eng_samples.sbr` / `ann_eng_samples.sbr` (chunklzx) |
| Speech preview (streams) | `stream_from_archives()` `:300` | `.sbs` in `nocache.big`, stored and streamed in place (already compatible once the root resolves) |

## 3. Gaps found in the existing preview reader (source reading, not reproduced)

- `bank_from_archives` joins `install_root.join(name)` over `BANK_ARCHIVES` (`:196`) with **no `game/` prefix**. With the install folder
  attached, it finds nothing.
- It decodes chunkzip and RefPack, and otherwise labels the bytes `"stored"` (`:283–289`). **It has no chunklzx branch.**
- With the `game` folder attached, `pbp_eng_samples.sbr` (chunklzx in `audioboot.big`) would therefore be parsed as raw chunklzx bytes.
  The expected result is a parse failure, not a silent misdecode, but that is **not reproduced**.
- `coverage()` admits the Recompiled pbp preview (test at `gen7_speech_preview.rs:668`), so the route is reachable.
- Receipt 59's five PCM-equal Recompiled previews were **sound banks** (`gen7_sfx_banks`, stored members), not speech.

*(Note 18:51, [17](17-recomp-portrait-head-read-contract.md) §4: reuse the established `read_eb_member` / `eb::decode_payload` (chunklzx + chunkzip) for the member unwrap rather than extending the private unwraps.)*

## 4. Smallest regressions (developer)

**Positive:**
- On the registered install, attached both as the install folder and as `game`, `speech_catalogue` returns the same entry counts and
  unwrapped source hashes as the X360 Legacy loose tree.
- The sources read `audioboot.big:audio/speech/…`, with wrapper `chunklzx` / `stored`.
- One pbp preview decodes to the same PCM as the X360 Legacy loose route.

**Negative:**
1. Wrong title (`nhl09`) on the Recompiled root: refused, as today.
2. A root with neither loose `audio/speech` nor the archives: the existing error, naming both searches.
3. An owned synthetic EB fixture whose member has a corrupted chunklzx header: refused, never treated as stored.
4. The X360 Legacy loose tree: unchanged results (loose first).

**Not claimed:** write support, runtime playback, or any title other than Legacy Recompiled.

## 5. Evidence classes

| Class | Items |
|---|---|
| **Measured** | members, archives, wrappers, no duplicates, 28 byte comparisons, 2 size-only comparisons |
| **Source reading** | consumer lines, the missing chunklzx branch and `game/` prefix |
| **Unreproduced** | the expected preview failure |
| **Not inferred from R138 sampling** | only the listed members are byte-compared here; nothing else is extended to all files |

## 6. Next Recompiled read routes (not contracted here)

In order: portrait / head preview (`fe/ion/artassets/playerheads*`, `rendering/player`), equipment previews (`rendering/<slot>`), then
arena reads (`cacherender.big` `rendering/stadium`, whose 67 members equal the X360 loose pair, 09 K10). Each needs the same shape of
contract, and the same chunklzx check.
