# Receipt 04 (P4.1 + P4.2) — PS4 NHL 15–21 speech banks listed and EA-MP3 previewed as a diagnostic

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** Branch `feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Nothing staged,
committed, pushed or launched; the extracted PS4 banks on `H:` were read only. Claim:
`dev-p4-ps4-ea-mp3-preview`. Research consumed: R67 (2026-09-22 handoff 03), R71 rev. 2 (pm
handoff 02) as corrected by R75 (pm handoff 07), the `snd10_sbr.py` bank layout (2026-09-20, R9).
Clock: 13:05 → 13:33 (MDT; the session clock jumped from 05:53 to 13:05 between packages).

## 1. Plain-language summary

PS4 NHL 15–17 play-by-play speech is stored as "EA-MP3": ordinary MP3 frames wrapped in EA's own
size-prefixed frames inside sound-bank block streams, with 1 152 raw samples tucked into the very
first frame. Research established the wrapping exactly and found strong static support for what
those raw samples are for (they stand in for the decoder's first, unusable frame — "candidate A"),
without anyone having listened. Studio had no reader for any of this.

Now it does, as a **diagnostic preview**: a new engine crate reads the SND10 bank tables, walks a
sample's block stream, checks the EA-MP3 grammar the way the research's refusing parser does
(every violation refused by name, including a dropped block), decodes the MPEG frames with a
pure-Rust MP3 decoder, and renders either the plain decoded stream or candidate A — always under
the label *"diagnostic preview (…; not game-verified)"*. On the speech page a PS4 project lists
its banks and samples with the codec measured from each sample's header, and decodes a selected
EA-MP3 sample on request into a playable WAV, with the bank's and the sample's identity shown.
All 24 research vectors pass through the new code with the research's frame counts, hashes of the
source bytes and RMS profile; the real NHL 17 bank (40 018 samples) lists and previews. No
replacement, encoder or game claim is made anywhere.

## 2. Decoder decision (P4.1)

| Candidate | License | Pure Rust | Maintained | Notes |
|---|---|---|---|---|
| **`symphonia-bundle-mp3` 0.6.1** (+ `symphonia-core`) | MPL-2.0 | yes | 2026-08-13 release; 13.3 M downloads | MPEG-1 / MPEG-2 Layer I–III, mono + stereo, MSRV 1.85 (repo 1.96); per-packet `Result`, no panics on malformed frames; gapless trimming switchable (turned **off** so frames × 1 152 samples come out); transitive deps all MIT / Apache-2.0 (`bitflags`, `bytemuck`, `lazy_static`, `log`, `smallvec`, `num-*`) |
| `minimp3` 0.6.1 | MIT | **no** — bindings to the minimp3 C library | 2025-08 | rejected: a C dependency for a diagnostic route |
| `puremp3` 0.1.0 | MIT / CC0 | yes | **2019**, one release | rejected: unmaintained |
| ffmpeg (the research oracle) | — | shell dependency | — | rejected as a production dependency (brief P4.1) |

MPL-2.0 is file-level copyleft: using the crates unmodified as dependencies imposes no obligation
on Studio's own code; the choice is recorded here for the coordinator. No prior dependency policy
document exists in the repository (searched `docs/DESIGN.md`, the working agreement, CLAUDE.md).
The output is **not** bit-exact with ffmpeg's `mp3` / `mp3float` (0 / 24 hash matches — expected
across decoders); the decoder-independent checks agree (§4).

## 3. Files changed

| Path | Change |
|---|---|
| `Cargo.toml`, `Cargo.lock` | new workspace member `crates/tdb-snd`; Symphonia added (lock +151 lines) |
| `crates/tdb-snd/` (**new**) | `sbr.rs` — SND10 `SBle` / `SBbe` bank reader (four field storages, DSETs, stream path placeholder detection) + `synth::build_bank`; `sns.rs` — SNS block walk (`0x48` / `0x44` / `0x45`), SNR header split, `encode_sample` helper; `eamp3.rs` — sized-frame grammar, MPEG-1 header parser (MPEG-2 rejected), the R71 refusing `contract_parse` (Σ block samples, tiling, flag, 1 152, 48 kHz mono, reservoir ≤ 511, nine frames per 10 368), synthetic frame helpers; `decode.rs` — frame-by-frame Symphonia decode, `Rendering::{MpegOnly, CandidateA}` with the contract labels; `wav.rs` — PCM16 WAV writer; 10 unit tests; `tests/ea_mp3_real_vectors.rs` (`--ignored`) |
| `crates/studio-domain/src/ps4_speech_bank.rs` (**new**), `lib.rs`, `Cargo.toml` | `list_banks`, `list_samples` (codec census from the headers, id filter, cap), `preview_sample` (grammar → decode → rendering → WAV, refusals by codec label / grammar / decoder); PS4 15–21 only; `tests/ps4_speech_bank.rs` (synthetic install; `--ignored` real NHL 17) |
| `crates/studio-api/src/{dto,registry,convert,commands,lib,bindings}.rs` | six DTOs; `ps4_speech_banks` / `ps4_speech_samples` / `ps4_speech_preview` (off-main commands); export roots 148 → 155 (pin moved) |
| `app/src/bindings/Ps4Speech*.ts` (6, generated), `app/src/lib/client.ts` | shapes |
| `app/src/components/Ps4SpeechBankCard.tsx` (**new**) + `.test.tsx`, `app/src/pages/SpeechCatalogue.tsx` | bank selector, sample list with codec pills, explicit **Decode diagnostic preview**, rendering choice, label + diagnostics + identity, one owned `<audio>` released on any selection change; late replies dropped |

## 4. Evidence

- **R71 vectors** ([`p4-ea-mp3-real-vectors.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-ea-mp3-real-vectors.log)):
  24 / 24 — bank `.OFF` = research offset; SNS bytes SHA-256 = research; codec / rate / channels /
  sample count / block count = research; grammar accepted with the research's total frames, trim and
  MPEG byte count; dropped-middle-block (16 applicable) and truncated-last-block refused; decode =
  frames × 1 152 samples, 48 kHz mono; the 96-sample-window RMS at 0 / 1 152 / 2 304 within 5 % of
  the research (typically 0.05 %); embedded PCM RMS within 0.5; first-sample-over-1-LSB within 128
  of the research (a rounding-sensitive threshold; e.g. 267 vs 264, 388 vs 338). Banks: NHL 15
  2 527 DSETs, NHL 16 4 435, NHL 17 3 188; the stream-path buffer carries the placeholder text on all three.
- **Real NHL 17 tree** ([`p4-ps4-speech-bank-real-nhl17.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-ps4-speech-bank-real-nhl17.log)):
  4 banks (`pbp_samples` 40 018 samples / 984 MB stream, `_closed_beta` 20 613, `_inxp` 727,
  `_inxp_and_cb` 12 927); census of the first 5 000 rows: EA-MP3 5 000; sample 15360 previews in
  both renderings (2 blocks, 17 frames 9/8, trim 289, first > 1 LSB at 314, 38 634-byte WAV).
- Synthetic: grammar mutations (10 classes), header parser edge cases, silent-frame decode, both
  renderings, WAV header; domain listing / census / refusals on a synthetic install; card behaviour.

## 5. Checks run ([`p4-checks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-checks.log))

| Run | Result |
|---|---|
| `cargo test -p tdb-snd` | 10 passed (lib); real-vector case ignored by default, **1 passed** with `--ignored` |
| `cargo test -p studio-domain --test ps4_speech_bank` | 1 passed; real NHL 17 case **1 passed** with `--ignored` |
| `cargo test -p studio-api --lib` | 31 passed after the export-root pin moved to 155 (the first run failed on the old pin — recorded) |
| `export-bindings --check` | in sync |
| `npx vitest run` (full) | **34 files, 224 passed** (Ps4SpeechBankCard 3) |
| `npx tsc --noEmit` | clean |
| `cargo tree -p tdb-snd` licenses | MPL-2.0 (Symphonia ×2), the rest MIT OR Apache-2.0 |

## 6. Decisions and limitations

- **Diagnostic only, by construction.** The preview label is fixed by the rendering; candidate B is
  not offered (R71 §1–2 reject it); nothing says "game audio". Replacement / encoding stays refused
  and the card says so. Listening acceptance is the user's (card A-MP3-1, morning review).
- **Coverage**: PS4 NHL 15–21 speech directories; the EA-MP3 route admits codec `0x0b`, 48 kHz,
  mono, stream type only (R67 §4 gate). Other codecs list with their label and refuse preview.
  NHL 18–21 banks were not measured by research (the R71 set is 15–17); the listing runs on them
  by the same layout but their codec census is an observation the first run will produce, not a claim.
- Bank sample counts aggregate every DSET carrying `.SID` + `.OFF` (the research picked the largest
  DSET only), which is why NHL 17 lists 40 018 samples where R71 saw a 3 863-record table.
- The `.sbs` window read per preview is capped at 16 MB (speech samples are far smaller); a
  sample longer than that would refuse at the block walk, not truncate silently.
- Not done here: gen7 EALayer3 v1 (P4.3, next), horn sources (P4.4), catalogue integration of the
  preview into name-calls / commentary rows (the PS4 speech catalogue itself is not built; the
  contexts listing carries no sample ids), event / bank mutation (no contract).

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_p4_api.py` | kept until closeout, then deleted |
| Temp roots `nhlms-ps4snd-*` | removed on drop |
| Background processes | none |

## 8. Next action

P4.3 — gen7 EALayer3 v1 diagnostic preview on the R77 / R80 measured tuples (granule grammar,
reservoir rebuild, Symphonia decode), then P4.4 horn sources under R78 / R81.
