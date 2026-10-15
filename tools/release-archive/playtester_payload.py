"""Clean, relocatable payload additions for an offline playtester archive."""
from pathlib import Path
import hashlib
import shutil
import sqlite3


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def add_payload(root, corpus, runtime):
    root, corpus, runtime = Path(root), Path(corpus).resolve(), Path(runtime).resolve()
    if not (runtime / "msedgewebview2.exe").is_file():
        raise ValueError("Expected the extracted official x64 Fixed Version runtime")
    # Snapshot only the historical database. Never include personal profiles, import ledgers or caches.
    destination = root / "corpus" / "historical.db"
    destination.parent.mkdir()
    with sqlite3.connect(corpus.as_uri() + "?mode=ro", uri=True) as source:
        with sqlite3.connect(destination) as target:
            source.backup(target)
            if target.execute("pragma quick_check").fetchone() != ("ok",):
                raise ValueError("Historical database integrity check failed")
            metadata = dict(target.execute("select key, value from meta"))
            if metadata.get("schema_version") != "6":
                raise ValueError("Unexpected historical schema")
    shutil.copytree(runtime, root / "webview2")
    notice = root / "licenses" / "webview2-runtime"
    notice.mkdir(parents=True)
    (notice / "NOTICE.txt").write_text(
        "Microsoft Edge WebView2 Fixed Version Runtime (x64), Microsoft Corporation.\n"
        "Official distribution: https://developer.microsoft.com/en-us/microsoft-edge/webview2/\n"
        "Deployment: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution\n"
        "All files of the downloaded runtime are preserved under webview2/.\n"
        "Its embedded third-party notices are available through show_third_party_software_licenses.bat.\n"
        "The runtime is distributed under Microsoft's terms, not Studio's MIT license.\n"
        "This fixed runtime does not auto-update; future Studio bundles must update it.\n", encoding="utf-8")
    return {
        "historical_database": {"path": "corpus/historical.db", "sha256": digest(destination),
                                "bytes": destination.stat().st_size, "metadata": metadata},
        "webview2": {"folder": "webview2", "source_folder": runtime.name,
                     "exe_sha256": digest(root / "webview2/msedgewebview2.exe")},
        "personal_state_included": False,
    }


README = """NHL Modding Studio — playtester package

1. Extract the entire ZIP to a local folder you own (not a network share).
2. Open nhl-studio.exe. No installer, developer tools or separate data download is needed.

Supported host: 64-bit Windows 10/11. Keep every bundled folder beside the executable.
The private WebView2 runtime is selected automatically. Studio prepares read/execute access
for its renderer on this bundled folder; no administrator installation is required.

Start fresh uses baselines/ (the verified anchor inventory and approved files).
Historical browsing opens corpus/historical.db automatically. The database contains 190,303
people across 117 seasons (1910–2026). Formula defaults and caches are created on first use;
your initial statistics/calculation requests may take longer while caches are built.
defs/ supplies game schemas and labels; helpers/ supplies audio decoding and matching source/notices.

Studio artwork (menu and toolbar images, logos, backgrounds, title/platform tiles and strategy
diagrams) is embedded in the executable. Game installations and extracted game artwork/audio
are not included; select your own matching game source during onboarding.
NHL20 fresh creation remains unavailable because its available anchor is an NHL19 alias.
NHL21's bundled baseline has no dictionary or roster save; its database still opens.
Current user qualification is focused on NHL 07 Xbox 360: MS-01 through MS-03 and DB-01 through
DB-06 passed their documented scope; historical asset review remains deferred. Other game/platform
options are available but do not inherit that qualification. Unsupported codecs/adapters and
unfinished editor features retain their existing limitations.

Save Session writes your project; autosave defaults to every 10 minutes. Build creates a .nhlmod
package. Use Mod Manager to activate a package or restore the original installation.
Package Dev Project Files creates a shareable .nhlproject archive.

This is an unsigned playtest build, not a public release or a completed acceptance pass.
See README.txt and BUILD-RECORD.json for outstanding release-readiness items, and
THIRD-PARTY-NOTICES.txt for notices. Game-derived baselines and historical data retain their
source provenance and are not relicensed under Studio's source-code license.
"""
