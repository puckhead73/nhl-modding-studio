"""Verify a release-candidate archive (receipt 43). Standard library only; launches nothing.

    python tools/release-archive/verify.py check <archive.zip> <extract-dir>
        Extract into <extract-dir> (must not exist yet), then check:
          - one root folder; every file listed in MANIFEST.sha256 with the right hash, nothing extra;
          - the required layout: nhl-studio.exe, defs/enums/, helpers/nhlms-xma2-helper.exe, the helper's
            notices README, FFmpeg licence, linked-runtime licences, exact source tarball + signature,
            build scripts / configuration, THIRD-PARTY-NOTICES.txt naming FFmpeg, BUILD-RECORD.json;
          - pinned identities: the helper binary and the FFmpeg source tarball, and the executable
            against BUILD-RECORD.json.
          - (receipt 46) every licence path the notices name exists; every listed crate / npm package has a
            licence folder with a file unless the notices flag it as missing; the WebView2 SDK and supplemental
            provenance files; both executables import only Windows system libraries;
          - the release status is consistent: a candidate that claims `release_ready` must carry Studio's
            LICENSE with a real copyright line, no blockers, no missing texts and no unrecorded artwork; one
            that does not must say "NOT RELEASE-READY" in README.txt and the notices and name its blockers.
        Prints the extracted root and exits 0, or lists every failure and exits 1.
    python tools/release-archive/verify.py check --release <archive.zip> <extract-dir>
        The same, and also fails unless the candidate is release-ready.
    python tools/release-archive/verify.py selftest <archive.zip> <scratch-dir>
        Proves the checks refuse broken archives: rebuilds copies with the helper, the source tarball,
        the helper notices, the FFmpeg licence or THIRD-PARTY-NOTICES removed, one file altered, and one
        file added, and requires `check` to fail on each. <scratch-dir> must not exist; it is removed.
"""
import hashlib
import json
import os
import re
import shutil
import sys
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_info  # noqa: E402

HELPER_SHA256 = "dc3533ed3234782703ebeab72034d9298b920b3e1fbb7c3fb9a9d2adfd818c21"
FFMPEG_SOURCE_SHA256 = "7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3"
N = "helpers/nhlms-xma2-helper-notices/"
REQUIRED = [
    "nhl-studio.exe", "README.txt", "BUILD-RECORD.json", "THIRD-PARTY-NOTICES.txt", "licenses/libspeex/COPYING",
    "helpers/nhlms-xma2-helper.exe", N + "README.txt",
    N + "licenses/ffmpeg/COPYING.LGPLv2.1", N + "licenses/ffmpeg/LICENSE.md",
    N + "licenses/gcc-runtime/COPYING.RUNTIME", N + "licenses/gcc-runtime/COPYING3",
    N + "licenses/mingw-w64-crt/COPYING", N + "licenses/mingw-w64-winpthreads/COPYING",
    N + "source/ffmpeg-8.1.3.tar.xz", N + "source/ffmpeg-8.1.3.tar.xz.asc",
    N + "build/build-helper.sh", N + "build/configure-args.txt", N + "build/config.h", N + "build/toolchain.txt",
    "licenses/webview2-sdk/LICENSE.txt", "licenses/webview2-sdk/NOTICE.txt", "licenses/SUPPLEMENTAL-PROVENANCE.json",
    # ONB-002: the bundled baseline inventory and one approved anchor from each platform family.
    "baselines/INVENTORY.json", "baselines/files/xbox360/nhllegacy/databases/nhlng.db",
    "baselines/files/ps3/nhl14/databases/nhlng.db", "baselines/files/ps4/nhl21/databases/nhl_game.db",
]
NOT_READY = "NOT RELEASE-READY"


def notices_consistency(root, notices):
    """Links named by the notices exist; every listed package has its licence folder (receipt 46)."""
    fails = []
    for word in re.split(r"[\s(),;]+", notices):
        w = word.rstrip(".:")
        if (w.startswith("licenses\\") or w.startswith("helpers\\")) and "<" not in w:
            rel = w.replace("\\", "/").rstrip("/")
            if not os.path.exists(os.path.join(root, *rel.split("/"))):
                fails.append(f"the notices name {rel}, which is missing")
    section = None
    for line in notices.splitlines():
        if re.match(r"^\d\. ", line):
            section = "rust" if "Rust crates" in line else "npm" if "Frontend packages" in line else None
            continue
        m = re.match(r"^  (\S+) (\S+) — ", line)
        if not (section and m):
            continue
        name, version = m.groups()
        folder = os.path.join(root, "licenses", "rust", f"{name}-{version}") if section == "rust" else os.path.join(root, "licenses", "npm", *name.split("/"))
        has = os.path.isdir(folder) and any(os.path.isfile(os.path.join(folder, f)) for f in os.listdir(folder))
        if not has and "NO LICENCE TEXT" not in line:
            fails.append(f"{section} {name} {version}: no licence text under {os.path.relpath(folder, root)} and not flagged as missing")
    return fails


def release_consistency(root, rec, notices):
    fails = []
    if "release_ready" not in rec:
        return ["BUILD-RECORD.json records no release readiness (assembled before the release gate)"]
    readme = open(os.path.join(root, "README.txt"), encoding="utf-8").read() if os.path.isfile(os.path.join(root, "README.txt")) else ""
    lic = os.path.join(root, "LICENSE")
    if os.path.isfile(lic):
        t = open(lic, encoding="utf-8").read()
        parts = t.split("\n\n")
        if not t.startswith("MIT License") or len(parts) < 2 or "<" in parts[1] or not re.search(r"Copyright \(c\) \d{4} \S", t):
            fails.append("LICENSE is not the MIT text with a real copyright line")
    if rec["release_ready"]:
        if not os.path.isfile(lic):
            fails.append("claims release-ready without Studio's LICENSE")
        if rec.get("release_blockers"):
            fails.append("claims release-ready while naming blockers")
        if "NO LICENCE TEXT" in notices or rec.get("licence_files_missing"):
            fails.append("claims release-ready with third-party licence texts missing")
        if "rights: unrecorded" in notices or any(r.get("rights") == "unrecorded" for r in rec.get("embedded_resources", [])):
            fails.append("claims release-ready with embedded artwork whose rights are unrecorded")
        if NOT_READY in readme or NOT_READY in notices:
            fails.append("claims release-ready but README / notices say not release-ready")
    else:
        if not rec.get("release_blockers"):
            fails.append("not release-ready but names no blocker")
        # Receipt 64: an archive that is not release-ready is an internal review build, whatever else it records.
        if "distribution" in rec and rec["distribution"] != "internal-review":
            fails.append(f"not release-ready but its distribution is {rec['distribution']!r}, not 'internal-review'")
        if NOT_READY not in readme:
            fails.append("README.txt does not say the candidate is not release-ready")
        if NOT_READY not in notices:
            fails.append("THIRD-PARTY-NOTICES.txt does not say the candidate is not release-ready")
    return fails


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def check(archive, dest, release=False):
    fails = []
    if os.path.exists(dest):
        return None, [f"{dest} already exists (use a fresh extraction folder)"]
    with zipfile.ZipFile(archive) as z:
        names = z.namelist()
        tops = {n.split("/")[0] for n in names}
        if len(tops) != 1:
            return None, [f"expected one root folder, found {sorted(tops)}"]
        for n in names:
            if n.startswith("/") or ".." in n.split("/"):
                return None, [f"unsafe member path {n}"]
        z.extractall(dest)
    root = os.path.join(dest, tops.pop())
    files = set()
    for d, _, fs in os.walk(root):
        for f in fs:
            files.add(os.path.relpath(os.path.join(d, f), root).replace(os.sep, "/"))
    listed = {}
    mpath = os.path.join(root, "MANIFEST.sha256")
    if not os.path.isfile(mpath):
        return root, ["MANIFEST.sha256 missing"]
    for line in open(mpath, encoding="utf-8"):
        h, rel = line.rstrip("\n").split("  ", 1)
        listed[rel] = h
    for rel, h in sorted(listed.items()):
        p = os.path.join(root, *rel.split("/"))
        if not os.path.isfile(p):
            fails.append(f"listed but missing: {rel}")
        elif sha(p) != h:
            fails.append(f"hash differs: {rel}")
    for rel in sorted(files - set(listed) - {"MANIFEST.sha256"}):
        fails.append(f"not in the manifest: {rel}")
    for rel in REQUIRED:
        if rel not in files:
            fails.append(f"required file missing: {rel}")
    if not os.path.isdir(os.path.join(root, "defs", "enums")):
        fails.append("required folder missing: defs/enums")
    # ONB-002: every inventoried baseline file is present with the inventoried hash.
    ip = os.path.join(root, "baselines", "INVENTORY.json")
    if os.path.isfile(ip):
        inv = json.load(open(ip, encoding="utf-8"))
        if inv.get("schema") != "nhlms-baselines/1":
            fails.append(f"baselines/INVENTORY.json has schema {inv.get('schema')!r}")
        for t in inv.get("targets", []):
            for c in t.get("components", []):
                p = os.path.join(root, "baselines", "files", *c["path"].split("/"))
                if not os.path.isfile(p):
                    fails.append(f"baseline file missing: baselines/files/{c['path']}")
                elif sha(p) != c["sha256"]:
                    fails.append(f"baseline file differs from its inventory hash: baselines/files/{c['path']}")
    hp = os.path.join(root, "helpers", "nhlms-xma2-helper.exe")
    if os.path.isfile(hp) and sha(hp) != HELPER_SHA256:
        fails.append("the helper is not the recorded build (SHA-256 differs)")
    sp = os.path.join(root, *(N + "source/ffmpeg-8.1.3.tar.xz").split("/"))
    if os.path.isfile(sp) and sha(sp) != FFMPEG_SOURCE_SHA256:
        fails.append("the FFmpeg source tarball is not the signed 8.1.3 release (SHA-256 differs)")
    tp = os.path.join(root, "THIRD-PARTY-NOTICES.txt")
    t = ""
    if os.path.isfile(tp):
        t = open(tp, encoding="utf-8").read()
        for needle in ("FFmpeg 8.1.3", "LGPLv2.1", "nhlms-xma2-helper-notices\\source", "libspeex", "WebView2 SDK"):
            if needle not in t:
                fails.append(f"THIRD-PARTY-NOTICES.txt does not mention {needle!r}")
        fails += notices_consistency(root, t)
    for rel in ("nhl-studio.exe", "helpers/nhlms-xma2-helper.exe"):
        p = os.path.join(root, *rel.split("/"))
        if os.path.isfile(p):
            try:
                i = pe_info.info(p)
                foreign = pe_info.non_system(i["imports"] + i["delay_imports"])
                if foreign:
                    fails.append(f"{rel} imports non-system libraries: {', '.join(foreign)}")
            except ValueError as e:
                fails.append(f"{rel}: {e}")
    rp = os.path.join(root, "BUILD-RECORD.json")
    ep = os.path.join(root, "nhl-studio.exe")
    if os.path.isfile(rp) and os.path.isfile(ep):
        rec = json.load(open(rp, encoding="utf-8"))
        if rec.get("exe_sha256") != sha(ep):
            fails.append("nhl-studio.exe differs from BUILD-RECORD.json exe_sha256")
        if rec.get("helper_exe_sha256") != HELPER_SHA256:
            fails.append("BUILD-RECORD.json names another helper")
        fails += release_consistency(root, rec, t)
        pkg_folder = os.path.join(root, "tools", "pkg-extractor")
        if "ps4_package_helper" in rec or os.path.exists(pkg_folder):
            from pkg_helper import validate as validate_pkg_helper
            try:
                pkg_record = validate_pkg_helper(pkg_folder)
                if rec.get("ps4_package_helper") != pkg_record:
                    fails.append("PKG helper differs from BUILD-RECORD.json")
            except (OSError, ValueError, KeyError) as e:
                fails.append(f"PKG helper bundle: {e}")
        if "playtester_payload" in rec:
            payload = rec["playtester_payload"]
            for rel, expected in [
                ("corpus/historical.db", payload["historical_database"]["sha256"]),
                ("webview2/msedgewebview2.exe", payload["webview2"]["exe_sha256"]),
            ]:
                path = os.path.join(root, *rel.split("/"))
                if not os.path.isfile(path) or sha(path) != expected:
                    fails.append(f"playtester payload missing or changed: {rel}")
            for rel in ["START-HERE.txt", "licenses/webview2-runtime/NOTICE.txt", "webview2/msedge.dll", "webview2/icudtl.dat"]:
                if not os.path.isfile(os.path.join(root, *rel.split("/"))):
                    fails.append(f"playtester required file missing: {rel}")
            # Personal history/settings must never leak into a clean playtester snapshot.
            corpus_dir = os.path.join(root, "corpus")
            if os.path.isdir(corpus_dir) and sorted(os.listdir(corpus_dir)) != ["historical.db"]:
                fails.append("playtester corpus contains personal or generated state")
        if release and not rec.get("release_ready"):
            fails.append("not release-ready: " + "; ".join(rec.get("release_blockers") or ["no readiness recorded"]))
    return root, fails


def rebuild(src_root, out_zip, drop=(), alter=None, add=None, consistent=False, prefix=False):
    """A copy of the extracted archive, broken as asked. `consistent`: rewrite MANIFEST.sha256 to match
    the broken copy, so only the layout / identity checks can catch it."""
    name = os.path.basename(src_root)
    entries = {}
    for d, _, fs in os.walk(src_root):
        for f in fs:
            p = os.path.join(d, f)
            rel = os.path.relpath(p, src_root).replace(os.sep, "/")
            if any(rel == x or rel.startswith(x.rstrip("/") + "/") or (prefix and rel.startswith(x)) for x in drop):
                continue
            data = open(p, "rb").read()
            if rel == alter:
                data = data + b"\n"
            elif isinstance(alter, tuple) and rel == alter[0]:
                data = alter[1](data)
            entries[rel] = data
    if isinstance(add, tuple):
        entries[add[0]] = add[1]
    elif add:
        entries[add] = b"extra"
    if consistent:
        entries["MANIFEST.sha256"] = "".join(f"{hashlib.sha256(v).hexdigest()}  {k}\n" for k, v in sorted(entries.items()) if k != "MANIFEST.sha256").encode()
    with zipfile.ZipFile(out_zip, "w", zipfile.ZIP_STORED) as z:
        for rel, data in sorted(entries.items()):
            z.writestr(f"{name}/{rel}", data)


def selftest(archive, scratch):
    if os.path.exists(scratch):
        sys.exit(f"{scratch} already exists")
    os.makedirs(scratch)
    try:
        good_root, fails = check(archive, os.path.join(scratch, "good"))
        if fails:
            sys.exit("the archive itself fails: " + "; ".join(fails))
        cases = {
            "helper removed": dict(drop=["helpers/nhlms-xma2-helper.exe"]),
            "source removed": dict(drop=[N + "source/"]),
            "helper notices removed": dict(drop=[N + "README.txt"]),
            "FFmpeg licence removed": dict(drop=[N + "licenses/ffmpeg/"]),
            "runtime notices removed": dict(drop=[N + "licenses/gcc-runtime/"]),
            "third-party notices removed": dict(drop=["THIRD-PARTY-NOTICES.txt"]),
            "defs removed": dict(drop=["defs/"]),
            "a file altered": dict(alter="defs/enums/player-style.toml"),
            "a file added": dict(add="helpers/extra.dll"),
            "helper removed, manifest rewritten": dict(drop=["helpers/nhlms-xma2-helper.exe"], consistent=True),
            "source removed, manifest rewritten": dict(drop=[N + "source/"], consistent=True),
            "notices removed, manifest rewritten": dict(drop=["THIRD-PARTY-NOTICES.txt", N + "README.txt"], consistent=True),
            "helper replaced, manifest rewritten": dict(alter="helpers/nhlms-xma2-helper.exe", consistent=True),
            # receipt 46: broken notice links and a false release status, each with a matching manifest.
            "libspeex licence removed, manifest rewritten": dict(drop=["licenses/libspeex/"], consistent=True),
            # The first crate folder the archive actually holds (a fixed name may not be in the closure).
            "a crate's licence folder removed, manifest rewritten": dict(drop=["licenses/rust/" + sorted(os.listdir(os.path.join(good_root, "licenses", "rust")))[0] + "/"], consistent=True),
            "WebView2 SDK notice removed, manifest rewritten": dict(drop=["licenses/webview2-sdk/NOTICE.txt"], consistent=True),
            "record flipped to release-ready, manifest rewritten": dict(alter=("BUILD-RECORD.json", lambda b: b.replace(b'"release_ready": false', b'"release_ready": true')), consistent=True),
            "distribution relabelled for release, manifest rewritten": dict(alter=("BUILD-RECORD.json", lambda b: b.replace(b'"distribution": "internal-review"', b'"distribution": "release-candidate"')), consistent=True),
            "not-ready banner removed, manifest rewritten": dict(alter=("README.txt", lambda b: b.replace(b"NOT RELEASE-READY", b"")), consistent=True),
            "placeholder LICENSE added, manifest rewritten": dict(add=("LICENSE", b"MIT License\n\nCopyright (c) <year> <copyright holders>\n\nPermission"), consistent=True),
        }
        ok = True
        for label, kw in cases.items():
            z = os.path.join(scratch, label.replace(" ", "-") + ".zip")
            rebuild(good_root, z, **kw)
            _, f = check(z, os.path.join(scratch, "x-" + label.replace(" ", "-")))
            print(f"{label}: {'refused' if f else 'ACCEPTED'} — {f[:3]}")
            ok &= bool(f)
        _, f = check(archive, os.path.join(scratch, "x-release-mode"), release=True)
        rr = json.load(open(os.path.join(good_root, "BUILD-RECORD.json"), encoding="utf-8")).get("release_ready")
        if not rr:
            print(f"release mode on a not-release-ready candidate: {'refused' if f else 'ACCEPTED'} — {f[-1:] if f else ''}")
            ok &= bool(f)
        print("selftest", "passed" if ok else "FAILED")
        return 0 if ok else 1
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


def main():
    args = sys.argv[1:]
    release = "--release" in args
    args = [a for a in args if a != "--release"]
    if len(args) != 3 or args[0] not in ("check", "selftest") or (release and args[0] != "check"):
        sys.exit(__doc__)
    sys.argv = [sys.argv[0]] + args
    if sys.argv[1] == "check":
        root, fails = check(sys.argv[2], sys.argv[3], release=release)
        if fails:
            print("FAILED:\n  " + "\n  ".join(fails))
            sys.exit(1)
        print(f"ok: {root}")
    else:
        sys.exit(selftest(sys.argv[2], sys.argv[3]))


if __name__ == "__main__":
    main()
