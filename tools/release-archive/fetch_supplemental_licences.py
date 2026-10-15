"""Retrieve licence / notice texts that the exact bundled package versions do not ship themselves (receipt 46).

Each text comes from the package's own upstream source at the **exact published version**:
  - Rust crates: the git commit recorded by the crate's own `.cargo_vcs_info.json` (written by `cargo publish`);
  - npm packages: the `gitHead` the registry records for that version (checked against the lockfile integrity);
  - the Microsoft WebView2 SDK static loader: the SDK NuGet package whose `WebView2LoaderStatic.lib` is
    byte-identical to the one `webview2-com-sys` links (checked here);
  - an upstream with no licence file at all (Servo `selectors`): the licence's own authoritative text.

Writes tools/release-archive/supplemental-licenses/<package>-<version>/<file> and PROVENANCE.json (URL, basis,
SHA-256, retrieval date). Downloads only these named files; installs nothing. Standard library only.

    python tools/release-archive/fetch_supplemental_licences.py [--check]
        --check: re-download and require byte-identical files (no write).
"""
import datetime
import hashlib
import io
import json
import os
import sys
import urllib.request
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "supplemental-licenses")
REGISTRY = os.path.join(os.path.expanduser("~"), ".cargo", "registry", "src", "index.crates.io-1949cf8c6b5b557f")


def gh(repo, commit, path):
    return f"https://raw.githubusercontent.com/{repo}/{commit}/{path}"


def crate_commit(dirname):
    """The commit `cargo publish` recorded for this crate version."""
    return json.load(open(os.path.join(REGISTRY, dirname, ".cargo_vcs_info.json")))["git"]["sha1"]


# (key, files: [(saved name, url)], basis). Commits are read from the registry copy, not typed in.
def sources():
    s = []
    c = crate_commit("alloc-stdlib-0.2.4")
    s.append(("alloc-stdlib-0.2.4", [("LICENSE", gh("dropbox/rust-alloc-no-stdlib", c, "LICENSE"))],
              f"crate .cargo_vcs_info.json commit {c}; repository root licence (the crate folder has none)"))
    c = crate_commit("ts-rs-10.1.0")
    s.append(("ts-rs-10.1.0", [("LICENSE", gh("Aleph-Alpha/ts-rs", c, "LICENSE"))],
              f"crate .cargo_vcs_info.json commit {c}; repository root licence"))
    c = crate_commit("unic-common-0.9.0")
    for n in ("unic-char-property", "unic-char-range", "unic-ucd-version"):
        assert crate_commit(f"{n}-0.9.0") == c, n
    s.append(("unic-0.9.0", [(f, gh("open-i18n/rust-unic", c, f)) for f in ("LICENSE-MIT", "LICENSE-APACHE", "COPYRIGHT.md")],
              f"crate .cargo_vcs_info.json commit {c} (the same for unic-common, unic-char-property, unic-char-range, "
              "unic-ucd-version 0.9.0); repository root licences and copyright statement"))
    c = crate_commit("unic-ucd-ident-0.9.0")
    s.append(("unic-ucd-ident-0.9.0", [(f, gh("open-i18n/rust-unic", c, f)) for f in ("LICENSE-MIT", "LICENSE-APACHE", "COPYRIGHT.md")],
              f"crate .cargo_vcs_info.json commit {c} (published from another commit than the other unic 0.9.0 crates); "
              "repository root licences and copyright statement"))
    c = crate_commit("webview2-com-0.38.2")
    assert crate_commit("webview2-com-sys-0.38.2") == c
    s.append(("webview2-com-0.38.2", [("LICENSE", gh("wravery/webview2-rs", c, "LICENSE"))],
              f"crate .cargo_vcs_info.json commit {c} (the same for webview2-com-sys 0.38.2); repository root licence"))
    c = crate_commit("selectors-0.36.1")
    s.append(("selectors-0.36.1", [("MPL-2.0.txt", "https://www.mozilla.org/media/MPL/2.0/index.txt")],
              f"crate declares MPL-2.0; its upstream (servo/stylo at the recorded commit {c}) has no licence file, so "
              "the licence's authoritative text from mozilla.org is used. Source Code Form: crates.io selectors 0.36.1 "
              f"(https://crates.io/crates/selectors/0.36.1) and https://github.com/servo/stylo/tree/{c}/selectors"))
    s.append(("react-three-fiber-8.18.0", [("LICENSE", gh("pmndrs/react-three-fiber", "f81bfe8cb29373d2687433c6130e0ce1daf7222c", "LICENSE"))],
              "npm registry gitHead f81bfe8cb29373d2687433c6130e0ce1daf7222c for @react-three/fiber 8.18.0 (= tag v8.18.0); "
              "tarball integrity equals app/package-lock.json"))
    return s


WEBVIEW2 = ("Microsoft.Web.WebView2", "1.0.3650.58")


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "nhlms-release-archive"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def sha(b):
    return hashlib.sha256(b).hexdigest()


def webview2():
    """The SDK version webview2-rs pins at the crate's commit; the loader it links must be byte-identical."""
    c = crate_commit("webview2-com-sys-0.38.2")
    main_rs = fetch(gh("wravery/webview2-rs", c, "crates/update-bindings/src/main.rs")).decode()
    assert f'const WEBVIEW2_VERSION: &str = "{WEBVIEW2[1]}";' in main_rs, "the crate pins another SDK version"
    url = f"https://www.nuget.org/api/v2/package/{WEBVIEW2[0]}/{WEBVIEW2[1]}"
    pkg = fetch(url)
    z = zipfile.ZipFile(io.BytesIO(pkg))
    names = z.namelist()
    local = open(os.path.join(REGISTRY, "webview2-com-sys-0.38.2", "x64", "WebView2LoaderStatic.lib"), "rb").read()
    lib = next(n for n in names if n.replace("\\", "/").lower().endswith("build/native/x64/webview2loaderstatic.lib"))
    assert sha(z.read(lib)) == sha(local), "the linked static loader is not from this SDK package"
    files = [n for n in names if os.path.basename(n).lower().startswith(("license", "notice", "thirdpartynotices", "third-party"))]
    assert files, names
    return files, z, url, sha(pkg), lib, sha(local)


def main():
    check = "--check" in sys.argv
    today = datetime.date.today().isoformat()
    prov = []
    got = {}
    for key, files, basis in sources():
        for name, url in files:
            got[(key, name)] = (fetch(url), url, basis)
    wfiles, z, wurl, wsha, wlib, wlibsha = webview2()
    for n in wfiles:
        got[(f"webview2-sdk-{WEBVIEW2[1]}", os.path.basename(n))] = (
            z.read(n), f"{wurl} ({n})",
            f"NuGet {WEBVIEW2[0]} {WEBVIEW2[1]} (package SHA-256 {wsha}), the SDK version webview2-rs pins at the "
            f"webview2-com-sys 0.38.2 commit; its {wlib} is byte-identical to the WebView2LoaderStatic.lib the crate links "
            f"into nhl-studio.exe (SHA-256 {wlibsha})")
    bad = []
    for (key, name), (data, url, basis) in sorted(got.items()):
        p = os.path.join(OUT, key, name)
        if check:
            if not os.path.isfile(p) or open(p, "rb").read() != data:
                bad.append(f"{key}/{name}")
            continue
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data)
        prov.append({"package": key, "file": name, "url": url, "sha256": sha(data), "basis": basis, "retrieved": today})
    if check:
        print("identical" if not bad else "DIFFERENT: " + ", ".join(bad))
        return 1 if bad else 0
    with open(os.path.join(OUT, "PROVENANCE.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(prov, f, indent=2)
        f.write("\n")
    for p in prov:
        print(p["package"], p["file"], p["sha256"][:16])
    return 0


if __name__ == "__main__":
    sys.exit(main())
