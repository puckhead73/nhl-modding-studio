# Audio

**Evidence:** measured by static, read-only parsing of the games' sound banks and streams: a codec census of 418 PS4 banks and 12 PS3 speech and music banks plus 862 PS3 effect banks, a survey of every loose bank on 16 Xbox 360 and PS3 trees, and decode tests of sampled streams through reference decoders. Nothing was listened to in a game, and no audio can be written back; see the replacement section below.

The games' audio comes in three containers and five EA codecs, mixed by title, platform and purpose. This page is the map; the detail is on [Audio Banks and Streams](Audio-Banks-and-Streams), [Audio Codecs](Audio-Codecs), [Goal Horns, Chants and Music](Goal-Horns-Chants-and-Music) and [Commentary and Speech Data](Commentary-and-Speech-Data).

## Containers by family

| Family | Speech and music | Effects, horns, chants, crowd |
|---|---|---|
| gen6 (NHL 07, 08) | an older speech system (`pbp_eng.{big,csi,cti,evt,idx,nam}`) that the later model does not cover; NHL 08 ships 212 `.abk` banks and about 226 MB of streams; codecs not measured | |
| gen7 (Xbox 360 and PS3, NHL 09 to Legacy) | `SBKR` `.sbr` index plus `SBKS` `.sbs` stream file; a menu and arena music slot table `music_musicboxdata.bin` (`MBX0`) | `ABKC` `.abk` bank plus an `S10S` `.ast` stream file when the bank has streamed records |
| PS4 (NHL 15 to 21) | `SBle` `.sbr` banks (the SND10 format) with DSET tables, plus an optional `.sbs` | same banks; every horn in one `hornbank.sbr` |

The Xbox 360 and PS3 bank index formats are the same, and bank ids are shared across platforms. On Xbox 360 the archived bank members are `chunklzx`-wrapped while the stream files sit raw in `nocache.big`; on PS3 the archived banks and streams are stored, unwrapped. The Legacy Recompilation's audio is byte-identical to Xbox 360 Legacy ([NHL Legacy Recompiled](NHL-Legacy-Recompiled)).

## Codecs

Every sample stream starts with an 8-byte SNR header whose codec nibble identifies the encoding:

| Nibble | Codec | Status |
|---|---|---|
| 0x03 | EA-XMA | framing measured, decodes |
| 0x05 | EALayer3 v1 | grammar measured, decodes |
| 0x06 | EALayer3 v2 | header only, inferred label; only remnant samples |
| 0x09 | EA-Speex | identified from structure, decodes |
| 0x0b | EA-MP3 | grammar measured, decodes |
| 0x0c | EA-Opus | packet framing measured, decodes |

## Codec by title and class

| Title and platform | Play-by-play | Announcer | Music | Effects, horns, chants, crowd |
|---|---|---|---|---|
| X360 NHL 09 | EA-XMA 24 kHz mono | EA-XMA 24 kHz | EA-XMA 44.1 kHz stereo | EA-XMA (packet-header variant) |
| X360 NHL 10 | EA-XMA 24 kHz | EA-XMA 24 kHz (also Finnish and Swedish announcers) | EA-XMA 44.1 kHz stereo | EA-XMA |
| X360 NHL 11, 12 | EA-Speex 32 kHz mono | EA-Speex 32 kHz | EA-XMA 44.1 kHz stereo | EA-XMA |
| X360 NHL 13, 14 | EA-Speex 32 kHz | EA-Speex 32 kHz | EALayer3 v1 44.1 kHz stereo | EA-XMA |
| X360 NHL 15, Legacy | EALayer3 v1 44.1 kHz mono (MPEG-1) | EA-Speex 32 kHz | EALayer3 v1 stereo | EA-XMA |
| PS3 NHL 09, 10, 11, 12, 13 | EALayer3 v1 24 kHz mono (MPEG-2) | EALayer3 v1 24 kHz | EALayer3 v1 stereo | EALayer3 v1 |
| PS3 NHL 14 | EA-Speex 32 kHz | EALayer3 v1 24 kHz | EALayer3 v1 stereo | EALayer3 v1 |
| PS3 NHL 15, Legacy | EALayer3 v1 44.1 kHz (MPEG-1) | EALayer3 v1 24 kHz | EALayer3 v1 stereo | EALayer3 v1 |
| PS4 NHL 15, 16, 17 | EA-MP3 48 kHz mono | rink announcer EA-Speex 32 kHz | EA-MP3 | EA-MP3 |
| PS4 NHL 18 | EA-Opus 48 kHz mono | EA-Speex | open | EA-MP3 |
| PS4 NHL 19, 20, 21 | EA-Opus | EA-Speex | EA-Opus | EA-MP3, part EA-Opus |
| PS4 NHL 21 horn bank | | | | EA-Opus 48 kHz **stereo** and EA-MP3 |

## What can and cannot be done

- **Listing and diagnostic preview** exist for most classes (see the [Commentary and Speech Catalogue](Commentary-and-Speech-Catalogue) page of the user guide). A diagnostic decode means a reference decoder accepted the rebuilt stream and returned the declared sample count; it is not listening evidence.
- **Replacement is not supported on any family.** Same-length substitution of one stream would need an encoded payload of the same codec, rate, channels, byte length and per-block sample words; no qualified encoder exists for EALayer3 v1, EA-XMA or EA-Speex, the EA-MP3 bit reservoir crosses block boundaries so blocks cannot be spliced, and EA-Opus has no packet re-framer. Arbitrary-length import is further blocked by undecoded table words whose semantics are unknown. Reassignment through database fields (pointing an arena at another listed horn id, swapping a PS4 sentence's sample group, swapping a gen7 music-box context) is the only edit route, and it is runtime-unverified.

Last reviewed: 2026-10-20
