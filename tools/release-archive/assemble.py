"""Assemble the local release-candidate archive (beta decision 3, 27 Sep 2026; receipts 43 and 46).

One self-contained zip, extracted anywhere and run in place:

    <root>/nhl-studio.exe                     Studio (frontend embedded)
    <root>/defs/                              metadata / enum definitions Studio reads beside its executable
    <root>/baselines/                         ONB-002: the bundled baseline inventory (INVENTORY.json) and the
                                              approved vanilla anchor files under files/ — what Start fresh copies
    <root>/helpers/nhlms-xma2-helper.exe      the XMA2 decoder helper (FFmpeg 8.1.3, LGPL 2.1+, unmodified)
    <root>/helpers/nhlms-xma2-helper-notices/ its licences, linked-runtime notices, exact corresponding source,
                                              build scripts and configuration
    <root>/LICENSE                            Studio's MIT licence — only when its copyright holder is recorded
    <root>/THIRD-PARTY-NOTICES.txt            attribution for everything third-party in the archive
    <root>/licenses/rust/<crate>-<version>/   licence files of the Rust crates compiled into nhl-studio.exe
    <root>/licenses/npm/<package>/            licence files of the frontend packages the bundle actually contains
    <root>/licenses/libspeex/                 the vendored libspeex 1.2.1 notice (compiled into nhl-studio.exe)
    <root>/licenses/webview2-sdk/             the WebView2 SDK loader licence / notice (statically linked)
    <root>/licenses/SUPPLEMENTAL-PROVENANCE.json  where each text a package does not ship itself came from
    <root>/README.txt                         layout, prerequisites, launch, status
    <root>/BUILD-RECORD.json                  input identities, release readiness and its blockers
    <root>/MANIFEST.sha256                    SHA-256 of every other file (sha256sum format)

Release readiness (receipt 46): the candidate is marked release-ready only when Studio's licence has its
recorded copyright holder, every bundled third-party package has a licence text, and every embedded
non-code resource has recorded rights. Otherwise BUILD-RECORD.json says `release_ready: false` with the
blockers in plain words, and README / notices / the About page say "internal review build". This is the
archive's own completeness; it is not release approval, publisher authenticity or acceptance.

The frontend closure is measured, not assumed: a sourcemap build of the same inputs lists the npm modules the
bundle holds, and its code must equal the embedded app/dist bundle (sourcemap comments aside).

Deterministic: files sorted, fixed zip timestamps, stored permissions; the same inputs give the same zip.
Nothing is uploaded or published. Standard library only (plus the repository's own npm / cargo).

    python tools/release-archive/assemble.py --exe <nhl-studio.exe> --helper-bundle <dir containing helpers/>
        --out <dir> --name <root folder name> --build-record <json> [--repo <repo root>]
"""
import argparse
import fnmatch
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_info  # noqa: E402

LICENSE_PREFIXES = ("license", "licence", "copying", "notice", "unlicense", "copyright", "authors")
ZIP_TIME = (2026, 9, 27, 0, 0, 0)
HERE = os.path.dirname(os.path.abspath(__file__))
SUPP = os.path.join(HERE, "supplemental-licenses")
# Packages whose published archive ships no licence file → the retrieved texts (fetch_supplemental_licences.py).
SUPPLEMENTAL = {
    ("rust", "alloc-stdlib", "0.2.4"): ["alloc-stdlib-0.2.4"],
    ("rust", "selectors", "0.36.1"): ["selectors-0.36.1"],
    ("rust", "ts-rs", "10.1.0"): ["ts-rs-10.1.0"],
    ("rust", "unic-char-property", "0.9.0"): ["unic-0.9.0"],
    ("rust", "unic-char-range", "0.9.0"): ["unic-0.9.0"],
    ("rust", "unic-common", "0.9.0"): ["unic-0.9.0"],
    ("rust", "unic-ucd-version", "0.9.0"): ["unic-0.9.0"],
    ("rust", "unic-ucd-ident", "0.9.0"): ["unic-ucd-ident-0.9.0"],
    ("rust", "webview2-com", "0.38.2"): ["webview2-com-0.38.2"],
    ("rust", "webview2-com-sys", "0.38.2"): ["webview2-com-0.38.2"],
    ("npm", "@react-three/fiber", "8.18.0"): ["react-three-fiber-8.18.0"],
}
WEBVIEW2_SDK = "webview2-sdk-1.0.3650.58"
NOT_READY = "INTERNAL REVIEW BUILD — NOT RELEASE-READY"


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def licence_files(d):
    try:
        names = sorted(os.listdir(d))
    except OSError:
        return []
    return [n for n in names if os.path.isfile(os.path.join(d, n)) and n.lower().startswith(LICENSE_PREFIXES)]


def rust_inventory(repo):
    """Crates linked into nhl-studio.exe: the normal-dependency closure of the `nhl-studio` package on
    x86_64-pc-windows-msvc, proc-macro crates excluded (they run at build time and are not linked)."""
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--offline", "--filter-platform", "x86_64-pc-windows-msvc",
         "--manifest-path", os.path.join(repo, "src-tauri", "Cargo.toml")],
        check=True, capture_output=True).stdout)
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(p["id"] for p in meta["packages"] if p["name"] == "nhl-studio")
    seen, stack = set(), [root]
    while stack:
        i = stack.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k.get("kind") is None for k in d["dep_kinds"]):
                p = pkgs[d["pkg"]]
                if any("proc-macro" in t["kind"] for t in p["targets"]):
                    continue
                stack.append(d["pkg"])
    ws = set(meta["workspace_members"])
    out = []
    for i in sorted(seen, key=lambda i: (pkgs[i]["name"], pkgs[i]["version"])):
        p = pkgs[i]
        out.append({
            "name": p["name"], "version": p["version"], "license": p.get("license") or ("(file) " + (p.get("license_file") or "none")),
            "repository": p.get("repository") or "", "workspace": i in ws, "dir": os.path.dirname(p["manifest_path"]),
        })
    return out


def _pkg_of(source):
    i = source.rfind("node_modules/")
    if i < 0:
        return None
    parts = source[i + len("node_modules/"):].split("/")
    return "/".join(parts[:2]) if parts[0].startswith("@") else parts[0]


def _strip_map_comment(b):
    return re.sub(rb"\n?/[/*]# sourceMappingURL=[^\n]*", b"", b).rstrip()


def frontend_closure(repo, scratch):
    """The npm packages the embedded bundle actually holds, and its non-code files.

    A sourcemap build of the same inputs names every bundled module; its code must equal the app/dist bundle
    that `tauri build` embedded (the sourcemap comment aside), otherwise the measurement is refused. CSS
    carries no module map, so a package's own preserved banner (`/*! name vX | licence */`) names it."""
    app = os.path.join(repo, "app")
    out = os.path.join(scratch, "frontend-closure")
    if os.path.exists(out):
        shutil.rmtree(out)
    subprocess.run(["npx", "vite", "build", "--sourcemap", "--outDir", out, "--emptyOutDir", "--logLevel", "error"],
                   cwd=app, check=True, shell=(os.name == "nt"))
    dist = os.path.join(app, "dist")

    def code(root):
        got = {}
        for d, _, fs in os.walk(root):
            for f in fs:
                if f.endswith((".js", ".css")):
                    got[os.path.relpath(os.path.join(d, f), root).replace(os.sep, "/")] = hashlib.sha256(_strip_map_comment(open(os.path.join(d, f), "rb").read())).hexdigest()
        return got
    a, b = code(dist), code(out)
    if sorted(a.values()) != sorted(b.values()):
        sys.exit(f"refusing: the sourcemap build's code differs from the embedded app/dist bundle ({sorted(a)} vs {sorted(b)}); rebuild app/dist from the same inputs first")
    names = set()
    for d, _, fs in os.walk(out):
        for f in fs:
            p = os.path.join(d, f)
            if f.endswith(".map"):
                for s in json.load(open(p, encoding="utf-8")).get("sources", []):
                    n = _pkg_of(s.replace("\\", "/"))
                    if n:
                        names.add(n)
            elif f.endswith(".css"):
                for m in re.finditer(rb"/\*!\s*([@\w./-]+)\s+v[\d.]+\s*\|", open(p, "rb").read()):
                    n = m.group(1).decode()
                    if os.path.isfile(os.path.join(app, "node_modules", *n.split("/"), "package.json")):
                        names.add(n)
    resources = []
    for d, _, fs in os.walk(dist):
        for f in fs:
            rel = os.path.relpath(os.path.join(d, f), dist).replace(os.sep, "/")
            if not rel.endswith((".js", ".css", ".html", ".map")):
                resources.append(rel)
    shutil.rmtree(out)
    pkgs = []
    for n in sorted(names):
        d = os.path.join(app, "node_modules", *n.split("/"))
        pj = json.load(open(os.path.join(d, "package.json"), encoding="utf-8"))
        pkgs.append({"name": n, "version": pj.get("version", ""), "license": pj.get("license") or "(not declared)", "dir": d})
    return pkgs, sorted(resources)


def resource_rights(repo, resources):
    """Each embedded non-code file with its recorded rights (embedded-resources.json)."""
    spec = json.load(open(os.path.join(HERE, "embedded-resources.json"), encoding="utf-8"))
    rows = []
    for rel in resources:
        e = next((e for e in spec["entries"] if any(fnmatch.fnmatch(rel, m) for m in [e["match"]] + e.get("also", []))), None)
        rows.append({"file": rel, "description": e["description"] if e else "(not listed)", "rights": e["rights"] if e else "unrecorded",
                     "terms": (e or {}).get("terms", ""), "group": (e or {}).get("group", "(not listed)"), "replacement": (e or {}).get("replacement", "")})
    for e in spec.get("executable_resources", []):
        rows.append({"file": e["source"], "description": e["description"], "rights": e["rights"], "terms": e.get("terms", ""),
                     "group": e.get("group", ""), "replacement": e.get("replacement", "")})
    return rows


def internal_bundling():
    """The recorded decision allowing the unrecorded artwork in internal builds (receipt 64), or None."""
    return json.load(open(os.path.join(HERE, "embedded-resources.json"), encoding="utf-8")).get("internal_bundling")


def copy_supplemental(keys, dest):
    for k in keys:
        src = os.path.join(SUPP, k)
        for f in sorted(os.listdir(src)):
            os.makedirs(dest, exist_ok=True)
            shutil.copyfile(os.path.join(src, f), os.path.join(dest, f))


def studio_licence(root):
    cfg = json.load(open(os.path.join(HERE, "studio-licence.json"), encoding="utf-8"))
    holder = (cfg.get("copyright_holder") or "").strip()
    if not holder:
        return cfg, None
    text = open(os.path.join(HERE, cfg["template"]), encoding="utf-8").read()
    assert "<year> <copyright holders>" in text
    text = text.replace("<year> <copyright holders>", f"{cfg['copyright_year']} {holder}")
    with open(os.path.join(root, "LICENSE"), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    return cfg, holder


def write_notices(root, rust, npm, helper_readme, resources):
    """Copy the licence texts; return the packages still without one and the notices lines (line 2 = status)."""
    missing = []
    lines = []
    w = lines.append
    w("NHL Modding Studio — third-party notices")
    w("=" * 40)
    w("Status: (set by the assembler)")
    w("")
    w("This archive contains third-party software. Each component stays under its own licence; the")
    w("licence texts are in the folders named below. NHL Modding Studio's own source code is under the MIT")
    w("License (LICENSE, when present — see README.txt); that licence does not cover the components below,")
    w("EA game data (none is shipped) or the artwork listed in section 7.")
    w("")
    w("1. FFmpeg — helpers\\nhlms-xma2-helper.exe")
    w("-" * 40)
    w("The XMA2 decoder helper is the ffmpeg command-line program from FFmpeg 8.1.3 (https://ffmpeg.org),")
    w("built unmodified with only the XMA2 decoder, the WAV demuxer, the pcm_s16le encoder / muxer and the")
    w("pipe protocol enabled. This software uses code of FFmpeg (http://ffmpeg.org) licensed under the")
    w("LGPLv2.1 (https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html); its source can be found in")
    w("helpers\\nhlms-xma2-helper-notices\\source\\ (the exact FFmpeg 8.1.3 release tarball and its signature).")
    w("FFmpeg is a trademark of Fabrice Bellard, originator of the FFmpeg project. This helper is not an")
    w("official FFmpeg release. Studio runs it as a separate program; Studio does not link FFmpeg.")
    w("Linked into the helper as well: the mingw-w64 CRT and winpthreads, GCC libgcc / libatomic (GCC Runtime")
    w("Library Exception) and the mingw-w64 default application manifest (declared public domain by its")
    w("MSYS2 package). Configuration: GPL, version-3 and non-free parts disabled; no external libraries.")
    w("Details (configuration, build scripts, toolchain, statically linked runtime notices):")
    w("helpers\\nhlms-xma2-helper-notices\\README.txt and its build\\ and licenses\\ folders.")
    w("")
    for line in helper_readme.splitlines():
        w("    " + line)
    w("")
    w("2. libspeex 1.2.1 — compiled into nhl-studio.exe (diagnostic EA-Speex previews)")
    w("-" * 40)
    w("Copyright Xiph.Org Foundation and contributors; BSD-3-Clause. Licence text: licenses\\libspeex\\COPYING.")
    w("")
    w("3. Microsoft WebView2 SDK 1.0.3650.58 loader — statically linked into nhl-studio.exe")
    w("-" * 40)
    w("Copyright (C) Microsoft Corporation. Licence and notices: licenses\\webview2-sdk\\LICENSE.txt and")
    w("licenses\\webview2-sdk\\NOTICE.txt. The WebView2 Runtime itself is not included (see README.txt).")
    w("")
    w("4. Rust crates compiled into nhl-studio.exe")
    w("-" * 40)
    w("Name, version, declared licence; licence files under licenses\\rust\\<name>-<version>\\. Texts a crate")
    w("does not ship itself were retrieved from its upstream at the published version (marked [upstream]);")
    w("provenance: licenses\\SUPPLEMENTAL-PROVENANCE.json.")
    for c in rust:
        if c["workspace"]:
            continue
        dest = os.path.join(root, "licenses", "rust", f"{c['name']}-{c['version']}")
        files = licence_files(c["dir"])
        for f in files:
            os.makedirs(dest, exist_ok=True)
            shutil.copyfile(os.path.join(c["dir"], f), os.path.join(dest, f))
        mark = ""
        if not files:
            keys = SUPPLEMENTAL.get(("rust", c["name"], c["version"]))
            if keys:
                copy_supplemental(keys, dest)
                mark = "  [upstream]"
            else:
                missing.append(f"rust {c['name']} {c['version']} ({c['license']})")
                mark = "  [NO LICENCE TEXT: not shipped in the crate and not retrieved]"
        w(f"  {c['name']} {c['version']} — {c['license']}{mark}")
    w("")
    w("5. Frontend packages embedded in nhl-studio.exe")
    w("-" * 40)
    w("The packages the bundle actually contains (measured from a sourcemap build of the embedded bundle);")
    w("licence files under licenses\\npm\\<name>\\.")
    for p in npm:
        dest = os.path.join(root, "licenses", "npm", *p["name"].split("/"))
        files = licence_files(p["dir"])
        for f in files:
            os.makedirs(dest, exist_ok=True)
            shutil.copyfile(os.path.join(p["dir"], f), os.path.join(dest, f))
        mark = ""
        if not files:
            keys = SUPPLEMENTAL.get(("npm", p["name"], p["version"]))
            if keys:
                copy_supplemental(keys, dest)
                mark = "  [upstream]"
            else:
                missing.append(f"npm {p['name']} {p['version']} ({p['license']})")
                mark = "  [NO LICENCE TEXT: not shipped in the package and not retrieved]"
        w(f"  {p['name']} {p['version']} — {p['license']}{mark}")
    w("")
    w("6. Not included")
    w("-" * 40)
    w("The Microsoft Edge WebView2 Runtime (a Windows component; see README.txt) and any EA game data.")
    w("")
    w("7. Artwork embedded in nhl-studio.exe")
    w("-" * 40)
    w("Neither Studio's licence nor this archive grants rights in these images. Rights recorded:")
    for r in resources:
        w(f"  {r['file']} — {r['description']} — rights: {r['rights']}{(' (' + r['terms'] + ')') if r['terms'] else ''}")
    ib = internal_bundling()
    if ib and any(r["rights"] == "unrecorded" for r in resources):
        w("")
        w(f"Bundled in this internal build by the {ib['by']}'s decision of {ib['decided']} (\"{ib['decision']}\"). That permits internal")
        w("research / team builds only; it records no rights. Replacement backlog before any external release, by group:")
        for g in sorted({r["group"] for r in resources if r["rights"] == "unrecorded"}):
            n = sum(1 for r in resources if r["group"] == g and r["rights"] == "unrecorded")
            w(f"  {g}: {n} file(s) — replace, or record separately documented rights")
    return missing, lines


README = """NHL Modding Studio — {name}
======================================================================

{status}

Assembled locally by the development session for verification; not published, not uploaded, not launched
by the developer. Nothing here is user, listening or game acceptance, and "release-ready" (when shown)
means only that the archive's own completeness checks passed.

Requirements
  - 64-bit Windows 10 or 11.
  - The Microsoft Edge WebView2 Runtime (Evergreen). It is part of Windows 11 and is already installed on
    the vast majority of Windows 10 PCs, but not all. If Studio's window does not open, install the
    Evergreen WebView2 Runtime from https://developer.microsoft.com/microsoft-edge/webview2/ and retry.
  Studio needs no other runtime: the executable and the helper import only Windows system libraries
  (including the Universal C Runtime that is part of Windows 10 and 11).

Run
  Extract the whole archive anywhere (paths with spaces are fine) and run nhl-studio.exe from the
  extracted folder. Keep these together, next to the executable:
    defs\\      the metadata / enum definitions this build reads
    baselines\\ the bundled vanilla baselines (verified anchor files) that "Start fresh" copies into a project
    helpers\\   the XMA2 decoder helper for Xbox 360 audio previews and its source and notices
  No environment variable and no separate download or build are needed. Studio never searches PATH
  for the helper. Settings → About & Licences shows the licence texts below from these files.

Licences
  NHL Modding Studio's own source code: MIT License. {licence_line}
  THIRD-PARTY-NOTICES.txt lists every third-party component and where its licence text is.
  The XMA2 helper is an unmodified FFmpeg 8.1.3 build (LGPL 2.1 or later); its exact corresponding
  source, build scripts, configuration and linked-runtime notices are in
  helpers\\nhlms-xma2-helper-notices\\.

Integrity
  MANIFEST.sha256 holds the SHA-256 of every other file (check with sha256sum -c or the repository's
  tools/release-archive/verify.py). BUILD-RECORD.json names the inputs this was built from and, when not
  release-ready, why. Integrity is not publisher authenticity: this archive is not code-signed.

Removal: delete the extracted folder.
"""


def tree_manifest(root):
    rows = []
    for d, _, files in os.walk(root):
        for f in files:
            p = os.path.join(d, f)
            rel = os.path.relpath(p, root).replace(os.sep, "/")
            if rel == "MANIFEST.sha256":
                continue
            rows.append((rel, sha(p)))
    rows.sort()
    with open(os.path.join(root, "MANIFEST.sha256"), "w", encoding="utf-8", newline="\n") as f:
        for rel, h in rows:
            f.write(f"{h}  {rel}\n")
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", required=True)
    ap.add_argument("--helper-bundle", required=True, help="directory containing helpers/ (tools/xma2-helper/package-helper.sh output)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--build-record", required=True, help="JSON with the build's input identities, copied as BUILD-RECORD.json")
    ap.add_argument("--repo", default=os.getcwd())
    ap.add_argument("--corpus", help="historical database to snapshot into an offline playtester bundle")
    ap.add_argument("--webview-runtime", help="complete extracted Microsoft x64 fixed runtime")
    ap.add_argument("--pkg-helper-bundle", help="qualified prepare_pkg_helper.py output, with source and runtime notices")
    a = ap.parse_args()
    if bool(a.corpus) != bool(a.webview_runtime):
        ap.error("--corpus and --webview-runtime must be supplied together")
    repo = os.path.abspath(a.repo)
    out = os.path.abspath(a.out)
    if not os.path.basename(out).startswith("nhlms-release-candidate"):
        sys.exit(f"refusing: the output folder must be an nhlms-release-candidate-* folder ({out})")
    stage = os.path.join(out, "stage")
    root = os.path.join(stage, a.name)
    zpath = os.path.join(out, a.name + ".zip")
    if os.path.exists(zpath):
        sys.exit(f"refusing to overwrite an existing candidate: {zpath}")
    if os.path.exists(stage):
        shutil.rmtree(stage)
    os.makedirs(root)
    shutil.copyfile(a.exe, os.path.join(root, "nhl-studio.exe"))
    shutil.copytree(os.path.join(repo, "defs"), os.path.join(root, "defs"))
    # ONB-002: the bundled baselines, produced and verified by the generator from the committed
    # anchors and the audited manifest (never copied by hand).
    subprocess.run(["cargo", "run", "-p", "studio-api", "--bin", "baseline-inventory", "--offline", "--",
                    "--bundle", os.path.join(root, "baselines")], cwd=repo, check=True)
    if not os.path.isfile(os.path.join(root, "baselines", "INVENTORY.json")):
        sys.exit("refusing: the baseline bundle has no INVENTORY.json")
    shutil.copytree(os.path.join(a.helper_bundle, "helpers"), os.path.join(root, "helpers"))
    os.makedirs(os.path.join(root, "licenses", "libspeex"))
    shutil.copyfile(os.path.join(repo, "crates", "tdb-speex", "vendor", "speex-1.2.1", "COPYING"), os.path.join(root, "licenses", "libspeex", "COPYING"))
    copy_supplemental([WEBVIEW2_SDK], os.path.join(root, "licenses", "webview2-sdk"))
    shutil.copyfile(os.path.join(SUPP, "PROVENANCE.json"), os.path.join(root, "licenses", "SUPPLEMENTAL-PROVENANCE.json"))
    helper_readme = open(os.path.join(root, "helpers", "nhlms-xma2-helper-notices", "README.txt"), encoding="utf-8").read()

    # Runtime prerequisites, measured on the two executables that ship.
    pe = {k: pe_info.info(os.path.join(root, *p.split("/"))) for k, p in (("studio", "nhl-studio.exe"), ("helper", "helpers/nhlms-xma2-helper.exe"))}
    foreign = {k: pe_info.non_system(v["imports"] + v["delay_imports"]) for k, v in pe.items()}

    rust = rust_inventory(repo)
    npm, dist_resources = frontend_closure(repo, out)
    resources = resource_rights(repo, dist_resources)
    cfg, holder = studio_licence(root)

    blockers = []
    if not holder:
        blockers.append("Studio's MIT licence file: the copyright holder has not been recorded, so LICENSE is withheld (never filled with a placeholder).")
    unrecorded = [r for r in resources if r["rights"] == "unrecorded"]
    if unrecorded:
        ib = internal_bundling()
        blockers.append(f"Artwork embedded in the executable without recorded rights: {len(unrecorded)} files (box art, platform art, strategy diagrams, logo, backdrop, icon; THIRD-PARTY-NOTICES.txt section 7)."
                        + (f" Bundled for internal builds by the {ib['by']}'s decision of {ib['decided']}; to be replaced (or its rights documented) before external release." if ib else ""))
    for k, v in foreign.items():
        if v:
            blockers.append(f"The {k} executable imports non-system libraries the archive does not document: {', '.join(v)}.")

    missing, notice_lines = write_notices(root, rust, npm, helper_readme, resources)
    if missing:
        blockers.append(f"Third-party licence texts missing for {len(missing)} packages: " + "; ".join(missing) + ".")
    ready = not blockers
    status = "Release candidate: the archive's completeness checks passed (not a release approval)." if ready else (
        NOT_READY + ":\n" + "\n".join("  - " + b for b in blockers))
    notice_lines[2] = "Status: release candidate (completeness checks passed)." if ready else f"Status: {NOT_READY} (see README.txt)."
    with open(os.path.join(root, "THIRD-PARTY-NOTICES.txt"), "w", encoding="utf-8", newline="\r\n") as f:
        f.write("\n".join(notice_lines) + "\n")

    with open(os.path.join(root, "README.txt"), "w", encoding="utf-8", newline="\r\n") as f:
        f.write(README.format(name=a.name, status=status, licence_line=(
            f"LICENSE holds its text (Copyright (c) {cfg['copyright_year']} {holder})." if holder else
            "Its LICENSE file is withheld from this build until the copyright holder is recorded.")))
    rec = json.load(open(a.build_record, encoding="utf-8"))
    rec.update({
        "candidate": a.name,
        "release_ready": ready,
        "release_blockers": blockers,
        # Receipt 64: packaging eligibility only — never an acceptance of features, tests or user observations.
        "distribution": "release-candidate" if ready else "internal-review",
        "internal_bundling": internal_bundling(),
        "artwork_replacement_backlog": {g: sum(1 for r in resources if r["group"] == g and r["rights"] == "unrecorded") for g in sorted({r["group"] for r in resources if r["rights"] == "unrecorded"})},
        "studio_licence": {"spdx": cfg["spdx"], "decision": cfg["decision"], "holder_recorded": bool(holder)},
        "rust_crates_linked": sum(1 for c in rust if not c["workspace"]),
        "npm_packages_bundled": [f"{p['name']} {p['version']}" for p in npm],
        "licence_files_missing": missing,
        "embedded_resources": resources,
        "runtime_imports": {k: {"subsystem": v["subsystem"], "imports": v["imports"], "non_system": foreign[k]} for k, v in pe.items()},
    })
    if a.corpus:
        from playtester_payload import add_payload, README as PLAYTESTER_README
        rec["playtester_payload"] = add_payload(root, a.corpus, a.webview_runtime)
        with open(os.path.join(root, "START-HERE.txt"), "w", encoding="utf-8") as f:
            f.write(PLAYTESTER_README)
        with open(os.path.join(root, "README.txt"), "r", encoding="utf-8") as f:
            readme = f.read()
        start = readme.index("Requirements\n")
        end = readme.index("Licences\n", start)
        readme = readme[:start] + PLAYTESTER_README + "\n\n" + readme[end:]
        with open(os.path.join(root, "README.txt"), "w", encoding="utf-8") as f:
            f.write(readme)
        with open(os.path.join(root, "THIRD-PARTY-NOTICES.txt"), "a", encoding="utf-8") as f:
            f.write("\n8. Playtester data and runtime\n"
                    "  Microsoft Fixed Version WebView2: licenses\\webview2-runtime\\NOTICE.txt\n"
                    "  Historical data: corpus/historical.db; source/schema metadata recorded in BUILD-RECORD.json.\n"
                    "  User-authorized playtester bundling; no additional rights or public-release eligibility inferred.\n")
    if a.pkg_helper_bundle:
        from pkg_helper import add as add_pkg_helper
        rec["ps4_package_helper"] = add_pkg_helper(root, a.pkg_helper_bundle)
        with open(os.path.join(root, "THIRD-PARTY-NOTICES.txt"), "a", encoding="utf-8") as f:
            f.write("\nStandalone shadPS4Plus PKG Extractor 1.1 (unmodified separate executable)\n"
                    "  Program, Crypto++, LLVM runtime and zlib notices: tools/pkg-extractor/licenses/\n"
                    "  Exact program/static dependency source and build recipes: tools/pkg-extractor/source/\n"
                    "  Provenance and reconstruction instructions: tools/pkg-extractor/README.txt and BUNDLE.json\n")
        with open(os.path.join(root, "README.txt"), "a", encoding="utf-8") as f:
            f.write("\nPS4 base PKG import includes the standalone extractor under tools/pkg-extractor/.\n"
                    "Leave the optional extractor path blank in Studio to use this bundled copy.\n"
                    "Updates and DLC are not merged automatically.\n")
    with open(os.path.join(root, "BUILD-RECORD.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(rec, f, indent=2, ensure_ascii=False)
        f.write("\n")
    rows = tree_manifest(root)
    with zipfile.ZipFile(zpath, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for rel in sorted([r for r, _ in rows] + ["MANIFEST.sha256"]):
            zi = zipfile.ZipInfo(f"{a.name}/{rel}", date_time=ZIP_TIME)
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = 0o644 << 16
            with open(os.path.join(root, *rel.split("/")), "rb") as f:
                z.writestr(zi, f.read(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    print(json.dumps({"archive": zpath, "archive_sha256": sha(zpath), "archive_bytes": os.path.getsize(zpath), "files": len(rows) + 1,
                      "rust_crates": rec["rust_crates_linked"], "npm_packages": rec["npm_packages_bundled"],
                      "licence_files_missing": missing, "release_ready": ready, "release_blockers": blockers}, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
