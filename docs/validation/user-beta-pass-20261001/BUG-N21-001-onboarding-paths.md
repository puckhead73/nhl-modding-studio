# N21-001 — PS4 onboarding conflates emulator, game and save locations

2026-10-01. Priority P1: blocks initial NHL21 onboarding. User-observed failure on frozen r67; source inspection corroborates the assumptions. No corruption or write reported. No implementation/build/dispatch performed; bounded correction handoff prepared under verification freeze.

## Reproduction and evidence

1. Launch verified r67; select NHL21 PS4 onboarding.
2. Initial folder selection requests the shadPS4 application folder and promises discovery of both game data and saves.
3. User reports game files can be anywhere, independently of the emulator installation. Studio does not find saves in the user's roaming profile.

Expected: select the actual NHL21 game folder independently, and discover/select saves independently. Actual: emulator-relative assumptions prevent the requested onboarding.

User-supplied save root: `C:\Users\puckh\AppData\Roaming\shadPS4\savedata`. Read-only directory listing confirms `1\CUSA18182` beneath it. Save payloads were not read, decoded or modified. Exact game path and patch remain unknown. Do not infer codec/edit support merely from directory discovery.

Source evidence in current checkout (not asserted byte-identical to r67 source):

- `app/src/pages/OpenProject.tsx:615` and `:635`: picker/hint explicitly require folder containing shadPS4.exe.
- `app/src/lib/catalog.ts:77`: separate save location enabled only for Xbox/Recompiled; comment incorrectly assumes PS4 saves are under emulator root.
- `crates/studio-domain/src/discovery.rs:405`: game lookup expects Games/CUSA or a CUSA child of chosen root; arbitrary directly selected game directory is not handled here.
- `crates/studio-domain/src/discovery.rs:510`: save lookup uses selected-root-relative user/savedata, savedata or direct CUSA child; no independent roaming root.
- `crates/studio-domain/src/discovery.rs:375`: first matching CUSA save directory selected; multiple profile/variant discovery needs explicit handling.
- `crates/studio-api/src/commands.rs:97`: documented save_path ignored for PS3/PS4.

## Bounded correction contract

1. PS4 onboarding asks for **Game folder**, accepting a directly selected arbitrary folder; use native metadata/content to validate title/region rather than requiring a CUSA-named folder or proximity to shadPS4.exe. Retain existing parent-library layouts as optional supported input.
2. Expose independent **Save-data folder**, suggesting the current Windows user's roaming `shadPS4/savedata` location. Resolve roaming location from the OS/user environment; never hardcode puckh or a drive. Allow override for custom/portable configurations.
3. Inventory matching title saves across user profiles/region candidates with explicit paths and selection; do not silently choose the first matching profile or combine different games/patches. Handle supplied base, user and title directories deliberately.
4. Carry both locations through detection, validation, remembered paths and reopen. Asset/extraction roots derive from the game, not emulator or save folder. Preserve recognition/codec limits and source/copy safety.
5. Provide distinct missing-game, missing-save and unrecognized-save explanations. No recursive whole-drive discovery, migration or relocation of user files. No automatic extraction or writes to originals to make discovery succeed.

## Acceptance checks for a separately numbered correction candidate

- Synthetic arbitrary game directory on one drive plus roaming-shaped saves elsewhere: finds correct title and inventory.
- Direct game directory whose name is not CUSA; validate metadata and refuse mismatched title.
- Existing Games/CUSA and parent-library inputs remain usable.
- Custom/portable save-root override; base/user/title-level selection.
- Multiple users/variants: visible disambiguation, no first-match data loss.
- Missing/unreadable roots and unsupported payload: accurate distinct messages, no false editability claim.
- Persist/reopen independently selected roots without regression to Xenia, RPCS3 or Recompiled discovery.
- User repeats NHL21 A01/A02 and actual save inventory on their paths, then DB/save opening and resource attachment. Rerun affected startup/discovery/open/save-selection checks only; accepted NHL21-E1 identity result is not invalidated merely by path-selection changes.

Freeze discipline: preserve r67, build identity and dirty checkout. No new candidate until bounded implementation is explicitly dispatched. No message to developer/session sent. Retain this report while gathering remaining independent findings.
