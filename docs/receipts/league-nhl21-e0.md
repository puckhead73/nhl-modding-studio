# NHL21-E0 — stock baseline boot receipt (2026-09-17)

**Plain-language summary.** The unmodified NHL 21 (CUSA18182 01.00) was started on the recorded
shadPS4 build and reached two real league screens showing the baseline label — **Play Now →
Select Team** with "HOCKEYALLSVENSKAN" as the home-side league, and **Rosters → Edit Player** with
the league tiles including "HOCKEYALLSVENSKAN", "SHL", "DEL", "ICE HOCKEY LEAGUE" and the rest.
Boot to the main menu took about 73 seconds; the game created its own profile save on this first
boot. Every game input file hashes the same after the run as before, and no mod, patch, cheat or
game-specific setting was active. This is a **G0 pass for those two surfaces in this environment**
only: it says nothing yet about any edited file. The in-session re-enter and cold-relaunch checks
were not performed in this run (the user, driving the session, reports from prior experience that
the label is consistent every run — recorded as a user report, not an observation of this run).

---

## 1. Package, disposition, scope

* **Package:** NHL21-E0 per [league-l11-q1-r1-review-and-e0-dispatch](../validation/league-l11-q1-r1-review-and-e0-dispatch.md);
  card §2.5 E0 of [league-identity-runtime-qualification](../plans/league-identity-runtime-qualification.md).
* **Disposition: G0 pass** for *Play Now → Select Team* and *Rosters → Edit Player* on the recorded
  environment. Second-surface coverage **complete**. E1–E5 not executed, not authorized here.
* **How it was driven.** The launch, the legal-agreement accept and the navigation to the main
  menu / Rosters were performed by a disposable PowerShell helper (SendInput keyboard events into
  the focused emulator window, window-rectangle captures). The user then took over at the PC, drove
  the game to the two league surfaces and took the two F12 emulator screenshots; automation was
  stopped at the user's request and not resumed. All observations below are from the screenshots
  (visual) unless marked *log*.
* No source edits, modified game files, dictionary/anchor commits, staging, commits or pushes.

## 2. Environment and inputs (recorded this run)

| Item | Value |
|---|---|
| Emulator exe | `H:\Emulators\ShadPS4\shadPS4.exe`, 69 042 176 B, SHA-256 `aeb048921e7749ef9dc580a6009bc0cbff08b193ae9f04cce209c9e7cf012790` (unchanged after the run) |
| Emulator identity (*log*, this run) | `shadps4 v0.16.1 DMC`, revision `d96133b0aee984ede33a7e6063b60df3ddc12bb2`, branch `dev-next`, `checkpoint/p4-entry-0-gd96133b0a-dirty`, remote `puckhead73/shadPS4`; window title `shadPS4 v0.16.1 DMC puckhead73/dev-next checkpoint/p4-entry-0-gd96133b0a-dirty | CUSA18182 - NHL™ 21 <01.00>` |
| Emulator config | `C:\Users\puckh\AppData\Roaming\shadPS4\config.toml` pre-run SHA-256 `a3dcaffceedb73ee7004c070517974ba8f656428a1dcd9f6f8cc6d74899a3ef5`; "Game-specific config exists: false" (*log*); `enableMods = true` but `mods_folder/` empty; `patches/`, `cheats/`, `custom_configs/` empty; no `input_config` (defaults were written on this launch: cross = `n`/kp2, circle = `b`/kp6, options = enter, d-pad = arrows). Post-run the emulator rewrote only `windowPosX/Y` (`config.diff`) |
| Title | `param.sfo` SHA-256 `bb8c97e3…`: TITLE_ID `CUSA18182`, CONTENT_ID `EP0006-CUSA18182_00-NHLICEHOCKEY2021`, APP_VER `01.00`, VERSION `01.01`; *log*: `Game id: CUSA18182 Title: NHL™ 21`, `Fw: 0x7508000 App Version: 01.00` |
| `eboot.bin` | SHA-256 `fc95ca66cc65a45f6d125f5f02920c296d0c6ff1d7a65a727b6e6a25b1dd1731` (unchanged) |
| `databigs/inxp_00.big` | SHA-256 `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` = card §2.1 (unchanged) |
| Loose game tree | only the five shipped config files under `nhl/data/{common,kettle}/config` + `eawebkit.prx`; **no** loose `nhl_game.db` in the game tree (unchanged, 5 files) |
| Update / patch | none beside the game; `H:\Emulators\ShadPS4\Patches\` contains `NHL19.xml` only; `Patches/`, `DLCs/` unused for this title |
| Overlays | no ReShade/injector DLL in the emulator folder (a `reshade-shaders/` asset folder exists, no loader) |
| Save data before | none: no `savedata` directory under the user dir or the emulator folder; `sys_modules/CUSA18182` existed (empty) |
| Launch | `shadPS4.exe -g H:\Emulators\ShadPS4\Games\CUSA18182\eboot.bin`, cwd `H:\Emulators\ShadPS4`, pid 30348, started 23:14:10.8 local |

The stock claim holds: no edited loose database, overlay, selected patch, cheat or game-specific
config was present (pre-inventory), and the inputs re-hash identical (post-inventory).

## 3. Observations (timeline, local time)

| t (from launch) | Observation | Class | Evidence |
|---|---|---|---|
| +38 s | EA legal-agreement screen rendered with text (title bar shows the build/game) | visual | `00a-legal-agreement-t38s.png` |
| +51 s | Cross (`n`) accepted; "Choose your favourite team" step flashed (skipped/timed out on its own) | visual | `00b-favorite-team-prompt-t55s.png` |
| +73 s | Main menu, all modes visible (Play Now, Franchise, …, Rosters) | visual | `00c-main-menu-all-modes-t73s.png` |
| +2 min | Home page "Modify Rosters" with pinned modes (my navigation) | visual | `00e-home-modify-rosters.png` |
| +3 min 09 s | **Surface 1 — "PLAY NOW · SELECT TEAM"**: home side league wheel shows **HOCKEYALLSVENSKAN** (neighbours CHL above, INTER… below); away side Winnipeg Jets (WSH/WPG/ATL abbreviations, OFF 90 / DEF 84 / GOA 87). User-driven, F12 | **visual** | `01-playnow-select-team-hockeyallsvenskan.png` (`c78bb020…`) |
| +4 min 31 s | **Surface 2 — "ROSTERS · EDIT PLAYER"**: league tiles under group headings *North American / Major Junior / Europe*: NHL™, NHL ALUMNI ASSOCIATION, AHL, NHL ALUMNI ALL-TIME, ECHL, WHL, OHL, QMJHL, SHL, EXTRALIGA LEDNIHO HOKEJE, **HOCKEYALLSVENSKAN** (highlighted), LIIGA, NATIONAL LEAGUE, CHAMPIONS HOCKEY LEAGUE, DEL, **ICE HOCKEY LEAGUE**. User-driven, F12 | **visual** | `02-rosters-edit-player-league-tiles.png` (`d4d536c0…`) |
| ≈+6 min | Clean exit by the user; *log*: `UpdatePlayTime … 0:06:19`, `Cache dumped` | log | `logs/shad_log.e0-run.txt` |

**Label rendering note.** The dictionary text is "HockeyAllsvenskan"; both surfaces render it in
the game's uppercase display style ("HOCKEYALLSVENSKAN"). The casing is a UI transform, not a
different string. The Edit Player tiles also show NHL 21-specific texts that the title's own
dictionary carries and the NHL 20/18 proxies do not ("ICE HOCKEY LEAGUE" for `TXT_LEAGUE_EBEL`,
"NHL ALUMNI ALL-TIME") — consistent with the labels coming from NHL 21's own `eng_us.db`. This is
an observation consistent with the model, not a trace of the lookup.

**Unrelated baseline values seen:** SHL, DEL, LIIGA, NATIONAL LEAGUE, CHAMPIONS HOCKEY LEAGUE,
EXTRALIGA LEDNIHO HOKEJE, NHL™, AHL, ECHL, WHL, OHL, QMJHL; Winnipeg Jets name/abbreviation and
ratings on Select Team. AIK's team name and a player were not reached (not required for E0).

**Group headings** *North American / Major Junior / Europe* are not `stockleagues` values —
recorded as a Phase 1.2 lead (another label source).

## 4. Profile / restart behaviour

* **Profile:** created automatically on first boot, no sign-in or online step: *log* shows
  `/savedata0/{NAME, INDEX, DATA}` written; on disk `…\shadPS4\savedata\1\CUSA18182\Profile0\`
  (`DATA` 8 239 B SHA-256 `1bc473ff…`, `INDEX`, `NAME`, `sce_sys/icon0.png`, `sce_sys/param.sfo`).
  The legal agreement and favourite-team prompts appeared once (first boot). Whether they reappear
  on a relaunch was **not observed** (relaunch not performed).
* **Re-enter / cold relaunch:** **not executed in this run.** The user reports, from prior
  experience with this title on this emulator, that the label is consistent every run. That is
  a user report; the card's reload/second-boot check for E1+ still has to be observed when those
  experiments run.
* **Boot time:** ≈73 s launch → main menu on a cold shader cache (first run; `cache/CUSA18182.zip`
  924 726 B was written at exit, so a second boot may be faster — not measured).
* **Emulator stubs (*log*):** NP web API / SSL are stubbed or dummy; a handful of `/app0/nhl/ui/lua/*.lreq`
  files are reported missing by the game's own file layer; none prevented the surfaces above.
  "No dumped system fonts" warning at start — text nevertheless rendered on every screen observed.

## 5. New state left on the machine (reported, preserved, not cleaned)

Under `C:\Users\puckh\AppData\Roaming\shadPS4\` (full list with sizes in `userdir.new.txt`):
`savedata/1/CUSA18182/Profile0/*` (the game's profile — **preserved**), `cache/CUSA18182.zip`
(pipeline cache), `shader/patch/`, `input_config/{default,global}.ini` (emulator defaults written on
first use), `imgui.ini`, `play_time.txt`, `data/CUSA18182/`, `download/CUSA18182/ads/`,
`fonts/{font,font2}/`, `temp/CUSA18182/replayPS4/{Audio.replay 16.9 MB, RenderLowFidelity.replay
811 MB}` (the game's own temp replay buffers under the emulator's `/temp0` mount), the two F12
screenshots under `screenshots/`, and `log/shad_log.txt` (this run; the previous NHL 24 log was
copied to evidence before it was overwritten). `config.toml` differs only in `windowPosX/Y`. Per
the dispatch nothing was deleted or restored: the profile is a real save and the caches are the
emulator's own; the 811 MB temp replay file is the game's and is reported here for the user's
attention, not removed.

## 6. Evidence

[`docs/validation/evidence/league-nhl21-runtime/e0/`](../validation/evidence/league-nhl21-runtime/e0/)
— `SHA256SUMS.txt` covers everything: `screenshots/00a–00e` (helper window captures of the
emulator window rectangle; a sixth capture obscured by the operator's own tool windows was
discarded), `screenshots/01`, `02` (the user's F12 captures, byte-identical to the originals in the
emulator's `screenshots/` folder), `logs/shad_log.e0-run.txt` (this run, 18 108 lines),
`logs/shad_log.pre-run-nhl24-2026-09-15.txt`, `logs/drive.log` (helper timeline), `pre-inventory.txt`,
`post-inventory.txt`, `userdir.new.txt`, `config.pre.toml`, `config.diff`, `input_config.default.ini`.

## 7. Limits

* One environment: a locally modified shadPS4 fork, identified above; nothing transfers to stock
  shadPS4 or a console.
* First-boot only; no relaunch observed. Boot time is a cold-cache figure.
* Visual evidence is the two user F12 captures plus helper captures; no OCR or DOM — the labels
  are read by eye.
* G0 says nothing about any edited file, archive acceptance, placement or precedence.

## 8. Temporary artifacts and cleanup

| Item | Disposition |
|---|---|
| `local/runs/league-nhl21-e0/` (helper `drive.ps1`, captures, F12 copies, inventories, log copies) | **removed** after preservation (literal path, containment/reparse checks; `Test-Path` false) |
| Emulator process pid 30348 | exited by the user; none running at closeout |
| Game inputs | re-hashed identical (§2) |
| Emulator user data | new state reported in §5, nothing deleted |

Index empty; anchors untouched; no application source changed.

## 9. Next action

G0 passed for two named surfaces. Recommend **E1** (archive-member placement) as the next
separately scoped package, per card §2.5 — producing the Studio artifact first (card §2.4, expected
loose output `343404153ae5…` and archive output as recorded in SMOKE-1) and observing *Rosters →
Edit Player* (the richer surface: it shows every league label at once) and *Play Now → Select
Team*, with the reload/second-boot check performed for real. Before E1, note the review's
remaining refinements (blank ≠ consumption; persistent save/cache state now exists — `Profile0`
and `cache/CUSA18182.zip` — and must be identified in the run record). Stopping for Codex review.
