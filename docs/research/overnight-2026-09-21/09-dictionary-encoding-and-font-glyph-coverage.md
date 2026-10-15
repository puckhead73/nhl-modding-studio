# Handoff R34 — Localization text is UTF-8 on every title; the shipped fonts' glyph sets and the code points the dictionaries use that no shipped font maps (gen7 + PS4, 175 dictionaries, 27 font trees)

Research lane, 21 September 2026, Priority 1.4 (font glyph availability vs dictionary encoding; locale / platform exceptions). Claim
`research-r34-fonts-encoding`. Read-only; nothing game-verified; no writer. This answers unknown 3 of the 2026-09-20 handoff 04 §6 and gives
the developer's C5 "glyph gate" warning a static basis.

Machine-readable: [`data/r34-font-glyph-coverage.json`](data/r34-font-glyph-coverage.json) (per dictionary: encoding classification, code
points used, per-font and all-font misses; per font tree: every TTF's `cmap` code-point count and the union as ranges). Probe
`local/runs/overnight-research-2026-09-21/probes/r34_font_glyph_coverage.py` (≈ 8 min; the 2026-09-18 `tdb.py` dictionary readers; a
built-in TrueType `cmap` format 4 / 12 reader).

## Plain-language summary

Every localization dictionary the project has — the 22 repo anchors, the 106 private gen7 locale files (PS3 + Xbox 360, 8 locales) and the
47 extracted PS4 files (7 titles × 7–8 locales) — stores its text as **UTF-8** (multi-byte sequences are valid in every one of the 175 files;
no single-byte code page anywhere), and the key strings are ASCII on gen7 but contain UTF-8 on every PS4 file (3 keys per file). The fonts
the games ship are ordinary TrueType files: three "EA Helvetica Neue" cuts (plus `rockeb.ttf` from NHL 10) covering 322–339 code points on
gen7 — Latin-1 / Latin Extended-A / Cyrillic / Greek — and 20–23 TTFs covering 619–1 138 code points on PS4. Against the characters the
dictionaries actually use, the shipped gen7 fonts lack only nine CJK characters (the language-selection words 中文 / 日本語 / 简体 / 繁體,
present in every locale file), three format characters (thin space, right-to-left mark, line separator) and — in the language-neutral
`lng_lg` file of NHL 12–Legacy — eight Polish / Hungarian capitals (Ą Ć Ł Ń Ő Ś Ű Ż); on PS4 the only misses are format characters and two
stray C1 control bytes. Whether the console draws the CJK words from a system font is not established (`fonts.txt` names `arial.ttf` first,
and that file is not shipped — a system-font fallback is a candidate, not a finding). For the rebuild-with-new-glyph feature this means: a new
character in the Latin / Cyrillic / Greek ranges listed in the JSON is inside the shipped fonts on every title; anything else must be
flagged as "no shipped glyph" — the runtime gate stays a card.

## 1. Encoding (measured on all 175 files)

| Set | Files | Text encoding | Keys | Notes |
|---|---|---|---|---|
| Repo anchors (gen7 07–Legacy eng_us, PS4 15–21 eng_us) | 22 | UTF-8 (953–1 415 strings per file carry multi-byte sequences even in English) | ASCII (gen7), UTF-8 (PS4) | NHL 07 / 08 use 92 / 101 code points (no diacritics) |
| Private gen7 locales (`E:\nhl-studio-fixtures\loc`, PS3 + X360 08–Legacy; cze fin fre ger lng rus swe) | 106 | UTF-8, 0 invalid sequences | ASCII | `rus_ru` 184–210 code points (Cyrillic); `lng_lg` 207 |
| PS4 locales (7 titles; cze eng fin fre ger rus swe + NHL 21 lng) | 47 | UTF-8 | UTF-8 (3 keys per file) | up to 280 code points (NHL 21 `lng_lg`) |

The dictionary's Huffman alphabet is therefore a **byte** alphabet over UTF-8 (a new non-ASCII character adds its 2–3 UTF-8 bytes as
symbols), which is consistent with the ≤ 256-leaf trees of handoff 04 (2026-09-20).

## 2. Shipped fonts (measured `cmap` sets)

| Tree | Fonts (loose) | Union code points | `fonts.txt` |
|---|---|---|---|
| Xbox 360 09 | `eahelveticaneue{conbol,hv,med}-mod.ttf` | 322 | `arial.ttf`, the three EA Helvetica cuts (arial not shipped) |
| Xbox 360 10 / 11, PS3 11 | + `rockeb.ttf` (241 code points) | 334 | same (+ `ROCKEB.ttf` listed on NHL 10) |
| Xbox 360 / PS3 12 → Legacy | same four | 336 → 339 | same |
| PS3 08 / 09 / 10 / 12 | none loose (inside the BIG4 / EB archives — not opened) | — | — |
| PS4 15 | 20 TTFs (`data/common/font/*`, `webkitfont/*`, `ui/fonts/*`) | 619 | — |
| PS4 16–21 | 20–23 TTFs (Roboto Medium / Helvetica Neue LT 883–884 code points, Klavika 387, Stratum / NN Rektorat ≈ 500, Trade Gothic 498–504 …) | 1 002–1 138 | — |

The union ranges per tree are in the JSON (`fonts.<tree>._union_ranges`), e.g. gen7 Legacy: `U+0020–007E`, `U+00A0–017F` (Latin-1 +
Extended-A, but see §3 for the 8 capitals the EA cuts omit), `U+0192`, `U+02C6–02DC`, `U+0384–03CE` (Greek), `U+0401–045F` (Cyrillic),
`U+2013–2026`, `U+20AC`, `U+2122` …

## 3. Code points used that no shipped font maps (per tree; full lists in the JSON)

| Tree | Misses | What they are |
|---|---|---|
| gen7 09–Legacy, every locale | `U+4E2D 中 U+4F53 体 U+6587 文 U+65E5 日 U+672C 本 U+7B80 简 U+7E41 繁 U+8A9E 語 U+9AD4 體` (1–2 uses each) | the Chinese / Japanese language names in the language-selection strings |
| gen7 12–Legacy `lng_lg` (+ `cze_cz` on 13 / 15: `U+0104`) | `U+0104 Ą U+0106 Ć U+0141 Ł U+0143 Ń U+0150 Ő U+015A Ś U+0170 Ű U+017B Ż` (5–71 uses each), `U+0301` (combining acute) | Polish / Hungarian capitals used by the language-neutral name file; the EA Helvetica cuts map `U+00A0–017F` **except** these |
| gen7 various | `U+2009` thin space, `U+200F` RLM, `U+2028` line separator, `U+009D` (NHL 09 eng) | format / control characters — not glyphs |
| PS4 15–21 | `U+2009`, `U+200F`, `U+202F` (narrow no-break space, NHL 18 / 21), `U+2060` (word joiner, NHL 21), `U+0081` / `U+0090` (C1 controls in fre / ger texts) | format / control characters and two data defects |
| PS4 `rus_ru`, `cze_cz` letters | none | Cyrillic and Czech are fully covered on PS4; Cyrillic also on gen7 (the `-mod` cuts carry `U+0401–045F`) |

## 4. What this gives the developer

- **Encoding rule:** dictionary text is UTF-8 on every title; a rebuild that adds a character adds its UTF-8 bytes as new tree symbols; a
  writer must refuse invalid UTF-8 and stray C1 bytes (they exist in shipped data — preserve on round-trip, refuse on input).
- **Static glyph gate:** for a rebuilt / edited string, test each code point against the union set of the target tree (JSON ranges); report
  "in the shipped fonts" / "not in any shipped font (system-font fallback unknown)". On gen7 the eight Polish / Hungarian capitals are the
  practical trap (their lowercase forms are covered); on PS4 everything in the Latin / Cyrillic / Greek / Extended blocks is covered.
- **Not claimed:** which font a given screen uses (`fonts.txt` order only), the console's system-font fallback (arial), rendering of the
  format characters, the PS3 08–10 / 12 fonts (in-archive, not opened).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r34_font_glyph_coverage.py docs/research/overnight-2026-09-21/data/r34-font-glyph-coverage.json [--only anchors|private|ps4]   (≈ 8 min)
```
Inputs: the repo anchors, `E:\nhl-studio-fixtures\loc\*\*.db`, `H:\…\CUSA*\databigs\Extracted\nhl\data\kettle\loc\*.db`, the TTFs of the
trees named in the probe (X: / H:). Executed twice (private-only test, then all). Not executed: writes, launches.
