//! `tdb-snd` — **read-only** readers for EA's SND10 sound banks and the PS4 EA-MP3 speech codec,
//! plus a **diagnostic** MPEG-1 Layer III decode.
//!
//! * [`sbr`] — the `SBle` / `SBbe` `.sbr` bank: DSET record tables (`.SID`, `.OFF`, `.RAM`, `.DUR`,
//!   …) with the four observed field storages, and the `.sbs` stream path. Layout: research
//!   probe `snd10_sbr.py` (2026-09-20, R9), as observed on the NHL 21 PS4 banks and re-used
//!   unchanged by R61 / R67 / R71 on NHL 15–17.
//! * [`sns`] — the EA SNS block stream of one sample: `0x48` header (the 8-byte big-endian SNR
//!   words: codec, channel config, rate, type, loop, sample count), `0x44` data blocks (u24 size,
//!   u32 samples-in-block, payload), `0x45` end.
//! * [`eamp3`] — the PS4 codec `0x0b` **sized-frame grammar** (research R67, contract rev. 2 of
//!   R71 / R75): every data block is a chain of `u16le hdr | [u32le pcm_samples] | mpeg_frame |
//!   [pcm]` frames; the refusing parser checks tiling, MPEG-1 Layer III 48 kHz mono headers, the
//!   `0x8000` flag only on the first frame with exactly 1 152 embedded samples, `main_data_begin`
//!   against the bit reservoir (cap 511), nine frames per 10 368-sample block and Σ declared block
//!   samples = the SNR sample count (the only static check that catches a dropped block).
//! * [`decode`] — the concatenated MPEG frames decoded **frame by frame** through Symphonia's
//!   pure-Rust MP3 decoder (`symphonia-bundle-mp3` 0.6, MPL-2.0; gapless trimming off so a stream
//!   yields exactly frames × 1 152 samples), and the two renderings kept apart: the MPEG-only PCM,
//!   and **candidate A** (the embedded 1 152 samples replace decoded frame 0), trimmed to the SNR
//!   sample count. Candidate A is *not* listening-verified (card A-MP3-1); every rendering is a
//!   **diagnostic preview**, never game audio.
//!
//! What this crate does **not** do: encode anything, write anything, decode gen7 EALayer3 v1
//! (codec `0x05`, a different grammar — research R77 / R80), EA-Opus (`0x0c`), XMA or XAS, or
//! decide which take the game plays. Malformed input is refused by name; nothing panics on
//! truncated bytes.

pub mod abk;
pub mod decode;
pub mod ealayer3;
pub mod eamp3;
pub mod easpeex;
pub mod eaxma;
pub mod helper;
pub mod sbr;
pub mod sns;
pub mod wav;

pub use decode::{decode_layer3, render_candidate_a, DecodeError, Decoded, Rendering};
pub use eamp3::{contract_parse, parse_block, EaFrame, EaMp3Refusal, ParsedSample};
pub use sbr::{Dset, Field, Sbr, SbrError};
pub use sns::{walk_sample, Block, SnrHeader, SnsError};
