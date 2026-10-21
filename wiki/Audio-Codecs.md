# Audio Codecs

**Evidence:** measured framing grammars on the named vectors, with decodes through reference decoders that returned the declared sample counts. Every decode is a **diagnostic decode**: it proves the bytes can be rebuilt into a standard stream, not what the game plays or how it sounds. No encoder is qualified for any codec.

Where each codec is used per title is tabulated on [Audio](Audio).

## EALayer3 v1 (codec 0x05)

Used for every PS3 class, and on Xbox 360 for NHL 13 to Legacy music and NHL 15 and Legacy play-by-play.

```
ea_frame := u8 pcm_flag (00 or EE) | u8 MPEG header byte (CC = MPEG-1 44.1 kHz, 9C = MPEG-2 24 kHz)
            | 1-bit granule index (MPEG-1 alternates 0, 1) | [MPEG-1 and index 1: scfsi 4 bits per channel]
            | per channel: 12-bit part2_3_length + 47 (MPEG-1) or 51 (MPEG-2) side-info bits
            | main data, padded to a byte
            | [EE only: u16 offset_samples (0) | u16 pcm_samples (47) | u32 0 | 47 × s16 PCM]   after the main data
```

Each EA-frame is one Layer III granule. Interior blocks hold 16 granules per 9 216-sample block at 44.1 kHz or 9 per 5 184 at 24 kHz. The first block carries two extra warm-up granules and a 47-sample PCM frame, which is always the **second** EA-frame of block 0. The public description of this codec places the PCM fields before the granule; these banks place them after, a measured difference. Decoding rebuilds granules into standard MPEG frames using the bit reservoir (main_data_begin up to 511 on MPEG-1, 255 on MPEG-2); output is granules × 576. Stereo music uses per-channel side info; four-channel streams are two interleaved stereo EA-frame streams and six-channel three (inferred from frame order; the six-channel decode is not established).

Refused: a block that does not tile, an unknown flag byte, a header byte that disagrees with the rate class, a PCM frame out of position 2 or with a count other than 47. Open: whether the game discards the 1 152 warm-up samples and plays the 47 PCM samples in their place, and the exact end trim.

## EA-MP3 (codec 0x0b)

Used for PS4 NHL 15 to 17 speech and for PS4 effects, crowd, interface and music.

```
block payload := ea_frame+   (sizes tile the payload exactly)
ea_frame := u16le hdr | [u32le pcm_samples if hdr & 0x8000] | one complete MPEG-1 Layer III frame | [pcm_samples × s16le if flagged]
hdr & 0x7fff = total bytes of the ea_frame
```

Nine frames per block declaring 10 368 samples; 48 kHz mono with a bitrate that varies per frame (32 to 224 kbps). The 0x8000 flag appears only on a sample's first frame with 1 152 PCM samples. **The bit reservoir crosses block boundaries**, so a block can never be decoded or spliced alone. A junction test supports the reading that the embedded PCM replaces decoded frame 0 on 20 of 24 vectors; the preview renders the start that way and is labelled diagnostic. Six mutation classes are refused; a dropped middle block is caught only by the sum of declared block samples against the stream's count, because there is no declared block count.

## EA-Opus (codec 0x0c)

Used for PS4 NHL 18 to 21 speech and NHL 19 to 21 music. Each data block is a run of `u16 BE length | packet` items. Every packet's TOC byte is 0xF8 (CELT full-band, 20 ms, mono, one frame). Re-framed as Ogg Opus the stream decodes to 48 kHz mono with 960 samples per packet. The declared count equals round(`.DUR` × 48 000) and is 359 to 1 258 samples shorter than the decoded length; whether the excess is leading priming or trailing padding is open, so a preview trims to the declared count. The block's samples-in-block word (10 248 per full block) is not the packet sum and its meaning is open. Streams outside 48 kHz mono, including the stereo horn samples in the NHL 21 horn bank, are outside the measured contract.

## EA-Speex (codec 0x09)

Used for the Xbox 360 NHL 11 to Legacy announcer, Xbox 360 NHL 11 to 14 and PS3 NHL 14 play-by-play, and the PS4 NHL 15 to 21 rink announcer.

```
SNR: codec 9, channel config 0, 32 000 Hz
block payload := { u8 length (1..255; 0 refused; 256 not encodable) | Speex frame } × k, consuming the payload exactly
640 samples per frame (20 ms ultra-wideband); 10 frames per 6 400-sample block
first block: 11 frames declaring 6 531 = 11 × 640 − 509
frame byte 0: bit 7 = 0 on all 10 707 frames; bits 6..3 = narrowband mode 1..8
```

Measured on 96 of 96 well-formed vectors across 25 families; two independent Speex decoder implementations both return frames × 640 samples, and decoded − 509 − tail equals the declared count every time. The 509-sample start is a count relationship plus the reference decoder's rule, not a game trace. One PS4 NHL 21 rink-announcer record produces **near-silent output under both tested decoders** (peaks of a few LSB) and is unsuitable for validating onset or decoder fidelity; it is reported as an outlier without assigning intent.

## EA-XMA (codec 0x03)

Used for every Xbox 360 effect bank and stream, Xbox 360 NHL 09 and 10 speech, and NHL 09 to 12 music. A data block is `u32 w0 | one XMA2 packet`, with `w0 >> 2` the payload length; packets are under 0x800 bytes and zero-padded to 0x800. The first block decodes 384 samples more than declared (encoder delay). Decoding wraps the packets in a RIFF XMA2 header and feeds an XMA2-capable decoder; changing the header's declared sample count does not change the decoded length, so the count comes from the bitstream.

Results: 91 of 91 sampled streams decode to exactly the declared count; Xbox 360 NHL 09 stream entries 471 of 471 and in-bank segments 1 983 of 1 985 (the two failures, a chant and a crowd loop body, are short by whole frames and refused by name); every NHL 10 to Legacy `.ast` stream record decodes exactly. NHL 09 uses a **packet-header variant**: the header word is a constant (0x08000000 on 35 014 packets, 0x20000000 on 199), a slot holds one or two packets, and the NHL 10+ header check must not be applied to it or it wrongly refuses 47 records that decode. Two-segment loops (intro and body) decode separately and must not be spliced, because concatenating their packets yields more samples than declared. Four- and six-channel segments are two or three chained stereo chunks, each decoding exactly; the speaker mapping is open. A packet whose frame count is 0 with offset 0x7FFF is a "no frame begins here" marker, not an error.

## What stays open

Six-channel EALayer3 decoding; EA-Opus outside 48 kHz mono; the two short NHL 09 records; one NHL 10 sub-stream; the PS3 NHL 10 Finnish and Swedish announcer banks (never sampled); and all encoding.

Last reviewed: 2026-10-20
