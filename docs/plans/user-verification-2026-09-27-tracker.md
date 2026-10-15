# Per-title verification tracker

Use with the [master plan](E:/Repositories/nhl-modding-studio/docs/plans/user-verification-2026-09-27.md). All cells begin unrun. Codes: **P** pass, **F** fail, **R** expected refusal, **B** setup blocked, **N** not run, **—** not applicable. Mixed cells need an issue/result reference, e.g. `P; L6=R; issue 04`.

Build/executable: `E:\nhlms-review-build-2026-09-27\nhl-studio.exe` (SHA-256 `4dc5c540769b86858dd3be28255fdbe5e2282b3a146d20a51f4ca0b48455c502`, built 26 Sep 2026 12:14–12:17 MDT)  Source manifest: `docs/validation/evidence/overnight-development-2026-09-24/review-build/` (dirty-source manifest, 828 entries; build-identity.json)
Second bundle (receipts through 33): `E:\nhlms-review-build-r33-2026-09-27\nhl-studio.exe` (SHA-256 `d44ad451372525ceaaa3602aa39b937e8ecbde2cc6e1476ecbe6a6e15953e7b8`, built 27 Sep 2026 05:48–05:51 MDT, with `helpers\` XMA2 helper `dc3533ed…`)  Manifest: `…/review-build-r33/` (841 entries). Record which bundle each result used.
Third bundle (receipts through 38) — superseded by the fourth bundle: `E:\nhlms-review-build-r39-2026-09-27\nhl-studio.exe` (SHA-256 `cd00650b36e7c70f8373d22a411acfbde1ee4b5feb411c585047f23bd93a025b`, built 27 Sep 2026 07:39–07:42 MDT; helper `dc3533ed…`)  Manifest: `…/review-build-r39/` (852 entries).  Review date: __________
Ninth bundle (receipts through 64) — **current**, internal review candidate (not release-ready by its record): `E:\nhlms-review-build-r64-2026-09-28\NHL Modding Studio rc-20260928-r64\nhl-studio.exe` (SHA-256 `b7ab3ce5d5a03cb1255a0a1b78d76a04a4608511a5935c997146206fd73577ba`; the verified extraction of `rc-20260928-r64.zip` `6f88b5f7…`)  Manifest: `…/review-build-r64/` (952 entries).  Review date: __________

Eighth bundle (receipts through 59) — superseded by the ninth, internal review candidate (not release-ready by its record): `E:\nhlms-review-build-r59-2026-09-28\NHL Modding Studio rc-20260928-r59\nhl-studio.exe` (SHA-256 `940a24c097cd8d49d76ae3335ae63d6cc083227ff6482512574b410bdaccf4d6`, built 28 Sep 2026 07:50–07:52 MDT; the verified extraction of `rc-20260928-r59.zip` `aaf1e4fb…`)  Manifest: `…/review-build-r59/` (949 entries).  Review date: __________

Seventh bundle (receipts through 56) — superseded by the eighth, internal review candidate (not release-ready by its record): `E:\nhlms-review-build-r56-2026-09-28\NHL Modding Studio rc-20260928-r56\nhl-studio.exe` (SHA-256 `6eea54c1d5f9eee1c5cd9819866b9c8905e57b2d5b429bc2456773480252dfcd`, built 28 Sep 2026 07:01–07:03 MDT; the verified extraction of `rc-20260928-r56.zip` `d918ee6a…`)  Manifest: `…/review-build-r56/` (947 entries).  Review date: __________

Sixth bundle (receipts through 53) — superseded by the seventh, internal review candidate (not release-ready by its record): `E:\nhlms-review-build-r53-2026-09-28\NHL Modding Studio rc-20260928-r53\nhl-studio.exe` (SHA-256 `139656966a5ff8cb58bb69e086432328c4f247b7ccba71ac90a145360894212f`, built 28 Sep 2026 06:23–06:26 MDT; the verified extraction of `rc-20260928-r53.zip` `9899c076…`)  Manifest: `…/review-build-r53/` (944 entries).  Review date: __________
Fifth bundle (receipts through 48) — superseded by the sixth: `E:\nhlms-review-build-r48-2026-09-28\NHL Modding Studio rc-20260928-r48\nhl-studio.exe` (SHA-256 `73e16eb56e2fe346082ec451941e1cb908d23d5d5736eed1e6d129de6346015b`, built 28 Sep 2026 05:22–05:25 MDT; the verified extraction of `rc-20260928-r48.zip` `5f59eba7…`)  Manifest: `…/review-build-r48/` (916 entries).  Review date: __________
Fourth bundle (receipts through 42) — superseded by the fifth: `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\nhl-studio.exe` (SHA-256 `0ee1c38c3cb3fdcfbdedd6976ac66b1cca1a27ce26c07645046149b4dd498b80`, built 27 Sep 2026 12:56–12:59 MDT; the verified extraction of the release-candidate archive `7e81d48b…`)  Manifest: `…/review-build-r43/` (861 entries).  Review date: __________

Each column refers to the master-plan IDs. Desktop completion requires a disposition for every applicable check, not simply opening the page. Listening and game acceptance are separate. Repeat positive checks on every row; run disruptive/negative checks on the representative routes unless a title-specific concern warrants more.

## Primary targets — 27 title/platform rows

| Title / platform | Open/save U | Leagues L | Text/defaults D | Teams/kits T | Players/equipment P | Audio inspection A | Copy/restore W | Listening | Game | Notes / issues |
|---|---|---|---|---|---|---|---|---|---|---|
| NHL 07 Xbox 360 | N | N | N | N | N | N | N | N | N | Old schema; league text unresolved; image/resize limits |
| NHL 08 Xbox 360 | N | N | N | N | N | N | N | N | N | +50 ratings, weight base 150; BIG4; raw equipment labels |
| NHL 09 Xbox 360 | N | N | N | N | N | N | N | N | N | `DEL` key; archive-bank route currently refuses; BIG4 |
| NHL 10 Xbox 360 | N | N | N | N | N | N | N | N | N | +50/base150; BIG4+BH; speech codec coverage varies |
| NHL 11 Xbox 360 | N | N | N | N | N | N | N | N | N | +36/base120; two socks; raw group names; texlib equipment |
| NHL 12 Xbox 360 | N | N | N | N | N | N | N | N | N | Two socks; EB assets; groups |
| NHL 13 Xbox 360 | N | N | N | N | N | N | N | N | N | Stored Overall; one sock; arena assignment |
| NHL 14 Xbox 360 | N | N | N | N | N | N | N | N | N | One sock; no measured nameplate association in current card |
| NHL 15 Xbox 360 | N | N | N | N | N | N | N | N | N | Nameplates; name-call preview; XMA bank preview refusal |
| NHL Legacy Xbox 360 | N | N | N | N | N | N | N | N | N | Full representative; Legacy equipment labels |
| NHL 08 PS3 | N | N | N | N | N | N | N | N | N | Own PS3 dictionary, not Xbox-labelled anchor proxy; RW4ps3 |
| NHL 09 PS3 | N | N | N | N | N | N | N | N | N | Archived sound banks 186; own dictionary; BIG4 |
| NHL 10 PS3 | N | N | N | N | N | N | N | N | N | Archived sound banks 199; BIG4 header exception |
| NHL 11 PS3 | N | N | N | N | N | N | N | N | N | Loose sound banks; two socks; RW4ps3 |
| NHL 12 PS3 | N | N | N | N | N | N | N | N | N | Use extracted folder; archive kits/two socks/nameplates; 219 banks |
| NHL 13 PS3 | N | N | N | N | N | N | N | N | N | Archive kits/nameplates; 213 banks; Hershey residual |
| NHL 14 PS3 | N | N | N | N | N | N | N | N | N | Loose banks; raw equipment labels; no measured nameplate lookup |
| NHL 15 PS3 | N | N | N | N | N | N | N | N | N | 217 banks; crowd-loop pairs; nameplates |
| NHL Legacy PS3 | N | N | N | N | N | N | N | N | N | Full representative; Legacy equipment labels |
| NHL 15 PS4 base | N | N | N | N | N | N | N | N | N | DB settings; own archive variant; PNG logos; multi-stream GNF |
| NHL 16 PS4 | N | N | N | N | N | N | N | N | N | CSV settings; EA-MP3; GNF; dictionary |
| NHL 17 PS4 | N | N | N | N | N | N | N | N | N | CSV `_DEFAULT_`; inferred group portions; WC slots; songs |
| NHL 18 PS4 | N | N | N | N | N | N | N | N | N | Paired global defaults; Spengler inference; Austrian round tags |
| NHL 19 PS4 base | N | N | N | N | N | N | N | N | N | Base/patch/loose overrides separate; multi-stream GNF |
| NHL 20 PS4 actual CUSA15439 | N | N | N | N | N | N | N | N | N | Never use the NHL 19 copy labelled `ps4/nhl20` |
| NHL 21 PS4 | N | N | N | N | N | N | N | N | E1 accepted; rest N | First full Studio pass; own dictionary |
| NHL Legacy Recompiled | N | N | N | N | N | N | N | N | N | Actual root/adapter; no assumed `_compiled`; feature coverage explicit |

The NHL 07 PS3 research copy is **not silently counted as an installed primary target**. If a real usable source is registered, add it below and record provenance. A shared DB hash between Xbox and PS3 can support data equivalence but does not replace asset/dictionary/root checks on both platforms.

## Source variants / optional registrations

| Variant | Source identified | Open/group/schedule | Settings | Save/reopen copy | Restore | Runtime selection | Notes |
|---|---|---|---|---|---|---|---|
| NHL 15 PS4 patch DB (`patch_inxp00.big`) | N | N | N | N | N | N | Separate from base; do not assume patch is active merely because it exists |
| NHL 19 PS4 patch 56 | N | N | N | N | N | N | Own identity and recorded override state |
| Other installed patch actually being tested | N | N | N | N | N | N | Add title/version/hash |
| NHL 07 PS3, only if available | N | N | N | N | N | N | Source/coverage to be established |
| Xbox disc-image input(s), read/stage only | N | N | — | N | — | — | Record image title/source; no live image write |

## Representative checks that must not disappear inside a broad “pass”

| Check | Suggested title(s) | Result / issue |
|---|---|---|
| Pending/undo/redo, save then undo, stale review, acknowledged Close/Cancel | NHL 21 PS4 + Legacy X360 | N |
| Project switching during a slow request; same team ID in two projects | Two review projects | N |
| Colour edit → acknowledgment → undo → same value again | NHL 21 PS4 | N |
| Invalid membership pair; valid single-group move | NHL 21 PS4 + Legacy | N |
| Schedule no-op import, explicit mapping, stale preview refusal | Legacy + one PS4 | N |
| Dictionary attach/detach, growth, new-key refusal, glyph warning | Own dictionaries across PS4 + gen7 order classes | N |
| PS4 CSV paired global edit + pending/save/reopen | NHL 18 or 21 | N |
| Texture A → B → undo B leaves A | PS4 archive-member route | N |
| Shared private-copy versus whole-group scope/conflict | PS4 16–21 shared member | N |
| Xbox partial same-size; changed-size mipped refusal; stale plan | Legacy Xbox 360 copied texture | N |
| BIG4+BH consistency; NHL 07 cascade if prepared | NHL 10 X360/PS3; NHL 07 optional copy | N |
| Required sock missing → incomplete, restore → complete | NHL 11/12 disposable tree | N |
| PS3 archive kits and nameplate source paths | NHL 12 + 13 | N |
| Stream source identities; archived bank vs loose stream | PS3 NHL 09/10/12/13 | N |
| Crowd stereo pairs and loop point | PS3 NHL 15 | N |
| Equipment vocabulary stays title-scoped | Legacy, NHL 10, PS4 NHL 19 | N |
| Corrected goalie consistency (included: receipt 27 in the 26 Sep build) | NHL 08, NHL 11/Legacy, PS4 | N |

## End-of-title record (copy once per tested row)

Title/platform/version:
Build:
DB/meta/dictionary sources:
Attached copied asset root:
Edited records/assets:
Studio results and issue IDs:
Listening observations:
Game observations (if any):
Restoration method and hash verification:
Remaining cards / expected refusals:
User disposition:

## Additions from tonight's sessions

Do not infer delivery from an active prompt. Candidate deliveries may be recorded as “unassigned”; fill in the actual build only after checking its manifest. Unassigned rows are not evidence of build inclusion or a manual pass.

| Receipt / feature | Included build | Applicable targets | Added steps / superseded expectation | Result |
|---|---|---|---|---|
| 27 — goalie consistency / V-69 | 26 Sep review build (`4dc5c540…`) | Declared goalie consistency fields; seven-title focused automated coverage | P2: stored value, no rating tint/offset; declared range retained; fitted Overall unchanged | Not run manually |
| 27 — tournament browsing / V-70 | 26 Sep review build (`4dc5c540…`) | NHL 17 World Cup; PS4 18–21 CHL/Spengler contracts (automated label cases on 18/19/21) | Browse-only; named World Cup group stage, unnamed CHL/Spengler participants, interpretation labels; no edit/import/generation | Not run manually |
| 27 — gloves/helmet/visor/skates / V-71 | 26 Sep review build (`4dc5c540…`) | 25 equipment-bearing anchors plus genuine NHL 20; NHL 07 custom-player fields excluded | P6: stored IDs outside measured catalogues, pending/undo/reopen; goalie gear is a separate package | Not run manually |
| 28 — title-scoped nationality / birth-state country labels / V-72 | 26 Sep review build (`4dc5c540…`) | NHL 10, NHL 11 / 12 (changed); NHL 08 / 09 and NHL 13 → PS4 21 unchanged; automated on 25 anchor directories × 21 national teams | P1 identity: national-team players show their own country; codes without a national-team label on NHL 08–12 show as numbers (not a failure) | Not run manually |
| 29 — EA-Speex diagnostic previews / A-SPX-2 | **Not in the 26 Sep build**; **in the r33 bundle** (`d44ad451…`) | X360 NHL 11 → Legacy speech, PS3 NHL 14 pbp, PS4 NHL 15–21 rink announcer | listening card A-SPX-2 on the r33 bundle | Not run |
| 30 — EA-XMA helper route | **Not in the 26 Sep build**; **in the r33 bundle** with the helper | X360 NHL 10 → Legacy ram entries | A-XMA-2 on the r33 bundle | Not run |
| 31 — goalie gear 4.5 / V-74; X360 NHL 14 announcer Speex; "England" wording on NHL 10–12 | **Not in the 26 Sep build**; **in the r33 bundle** | 25 anchor directories + genuine NHL 20 | the 26 Sep build shows "Great Britain" for the British code on NHL 10–12 (expected there); the r33 bundle shows "England" | Not run |
| 32 — XMA2 helper binary | r33 bundle `helpers\` (`dc3533ed…`) | Xbox 360 XMA previews | helper found beside the executable without any environment variable; V-75 missing-helper state | Not run |
| 34 — pair-quota schedules; round-tag refusal / V-76, P1-A-q | r39 bundle (`cd00650b…`) | any league table with the game layout; PS4 18–21 Austrian refused | Generate… → pair quotas; odd-pair host choice; one undo step | Not run |
| 35 — style labels per title / V-77 | r39 bundle | X360 / PS3 NHL 07–10 changed; NHL 11+ unchanged | NHL 08 / 09 rail and roster = player page names; NHL 07 / 10 numbers | Not run |
| 36 — PS3 NHL 09 / 10 / 12 / 13 music; X360 NHL 09 banks / A-EAL3-2, A-XMA-4 | r39 bundle | PS3 09 / 10 / 12 / 13; X360 NHL 09 | previews; the two R131 short records refuse | Not run |
| 37 — logo resize on NHL 07 → 11 packages / T-3b | r39 bundle | X360 07 → 11, PS3 08 → 11 (extracted packages) | full-quad logos take a new size; calendar crops refuse | Not run |
| 38 — portrait / head reassignment; goalie mask notes / V-78, V-79, P34-1 | r39 bundle | X360 09 → Legacy, PS3 15 / Legacy, PS4 portraits; X360 / PS3 08 / 09 masks | Check / Apply / Undo; refusals by name | Not run |
| 40 — goalie gear file check (NHL 12 pads; NHL 08 / 09 mask paint) / V-74, V-79 | r43 bundle (`0ee1c38c…`) | X360 / PS3 NHL 12 pads; NHL 08 / 09 goalies | pads 54 lists 20 files on NHL 12; `maskcolor` shows its files | Not run |
| 41 — schedule generation keeps retained rows / V-81, V-76 (corrected recipe) | r43 bundle | any table with a kept row (NHL All-Star, a club left out) | the kept row stays with every value, or a named refusal | Not run |
| 42 — team goalie mask paints / V-80 | r43 bundle | NHL 10 → PS4 21 | slots per title; file counts on X360; 248 / 249 reported on PS4 | Not run |
| 43 — release-candidate archive (layout, notices, helper beside the exe) | the archive / r43 bundle | Windows | extract anywhere, run in place; `THIRD-PARTY-NOTICES.txt` and the helper notices present | Not run |
| 44 — NHL 08–11 division labels by clubs; NHL 07 header division / V-85 | r48 bundle (`73e16eb5…`) | X360 / PS3 NHL 07 → 11 | "Division 3: CBJ · CHI · DET · NSH · STL"; NHL 10 / 11 conferences "(inferred)"; NHL 07 CHI Central | Not run |
| 45 / 47 — Hockey Shop NHL 12 pads: title naming, reviewed install, `.bak`, stale refusal / V-83 | r48 bundle | a **copy** of X360 NHL 12 | 41 pads (no 0 / 2 / 3); "File naming: NHL 12"; a changed file after review refuses | Not run |
| 46 — About & Licences; notices; startup message / V-82 | r48 bundle | Windows | status "Internal review build — not a release" with two blockers; licence texts open; nothing online needed | Not run |
| 49 — venue-art gallery / V-86 | r53 bundle (`13965696…`) | X360 NHL 10 / 13 / 15 / Legacy; PS3 15 / Legacy (sizes only) | board-ad materials with their pictures; changed file refused; other titles "not measured" | Not run |
| 50 / 53 — X360 XMA: in-bank intro + loop, surround pairs, no-frame packet fix; NHL 11 / 13 / 14 streams / A-XMA-5, A-XMA-6 | r53 bundle | X360 NHL 10 → Legacy banks; NHL 11 / 13 / 14 `.ast` | previews at declared lengths; listening is yours | Not run |
| 51 — PS4 goalie mask pair / V-87 | r53 bundle | PS4 15–21 incl. genuine NHL 20 (install attached) | class shown, 99 reported not refused; another title's install refused | Not run |
| 52 — PS4 skater gear lists and pictures / V-88 | r53 bundle | PS4 17–21 (15 / 16 catalogue check only) | list item + key + picture; helmet 99 reported | Not run |
| 54 — PS4 goalie mask pictures / V-90 | r56 bundle (`6eea54c1…`) | PS4 17–21 (15 / 16 no picture) | the pair's picture with its basis caption | Not run |
| 55 — reassignment on archive-held installs / V-89 | r56 bundle | PS3 NHL 09–14, Legacy Recompiled (install attached; copy) | allowed checks name an archive member; missing ids refused; no install → "attach the install" | Not run |
| 56 — NHL 10 / 15 / Legacy stream shapes / A-XMA-7 | r56 bundle | X360 NHL 10, 15, Legacy `.ast` | previews at declared lengths; listening is yours | Not run |
| 57 — PS4 gear / mask names / V-91 | r59 bundle (`940a24c0…`) | PS4 17–21 (install attached) | "BAUER 4500"-style names; key in the tooltip | Not run |
| 58 — PS3 venue pictures; more gallery titles / V-92 | r59 bundle | PS3 NHL 14 / 15 / Legacy; X360 NHL 11 / 12 / 14 | PS3 pictures open; the four titles list venue art | Not run |
| 59 — Recompiled player edits (defect) / V-93; Recompiled sound banks / A-XMA-8 | r59 bundle | Legacy Recompiled (copy) | weight / ratings / birth date / contract editable; banks listed from its archives, previews labelled | Not run |
| 61 — Hockey Shop failed undo keeps originals | r64 bundle (`b7ab3ce5…`) | developer fault tests only; no card | — | n/a |
| 63 — PS4 head reassignment / V-94, V-95 | r64 bundle | PS4 NHL 15–21 incl. genuine NHL 20 (install attached; copy) | shipped heads allowed with class notes; 10391 / absent refused; both fields on 17–21; stale apply refused | Not run |
| 48 — grouped schedule generation / V-84 | r48 bundle | X360 / PS3 NHL 10 → Legacy, PS4 15 → 21 Swiss / Finnish | shipped groups only on an exact match; host rule required for odd counts; apply / undo / save a copy | Not run |
| 33 — XMA speech / music banks, `.ast` XMA streams, Cancel / A-XMA-2, A-XMA-3, V-75 | r33 bundle (`d44ad451…`) | X360 NHL 09 / 10 speech, NHL 09–12 music; `.ast` streams on NHL 10 / 12 / 15 / Legacy (NHL 15 loop pairs) | previewable rows; missing-helper banner and row reasons; Cancel during a long decode; NHL 11 / 13 / 14 `.ast` streams and NHL 09 banks stay refused (expected) | Not run |
| 28 — stored-code box beside labelled dropdowns / V-73 | 26 Sep review build (`4dc5c540…`) | every labelled field; range audit automated on NHL 21 and genuine NHL 20 | type an unlabelled in-range code (e.g. nationality 200 on NHL 20 / 21), review, save / reopen on a copy; out-of-range is not written | Not run manually |
