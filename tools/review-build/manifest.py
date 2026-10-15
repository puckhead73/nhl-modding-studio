"""Review-build manifests (no third-party packages).

  python tools/review-build/manifest.py inputs <out.tsv>
      Every modified / added / untracked file (git status, untracked files expanded) under the build
      input roots, with its SHA-256; deleted paths are listed with an empty hash. Prints the count and
      the manifest's own SHA-256.
  python tools/review-build/manifest.py tree <dir> <out.tsv>
      Every file under <dir> (relative path, bytes, SHA-256), sorted. Prints the count and the tree
      SHA-256 = SHA-256 of the UTF-8 lines "<relpath>\\t<sha256>\\n" in sorted path order.
  python tools/review-build/manifest.py compare <a.tsv> <b.tsv>
      Exit 1 and list the differing rows if two manifests differ (input drift during a build).
"""
import hashlib
import os
import subprocess
import sys

ROOTS = ("crates/", "app/src/", "app/package.json", "app/package-lock.json", "app/index.html", "app/vite.config.ts",
         "app/tsconfig.json", "src-tauri/", "defs/", "Cargo.toml", "Cargo.lock", "package.json", "tools/xma2-helper/",
         # receipt 46: app/public is bundled into the executable; the release tooling shapes the archive.
         "app/public/", "package-lock.json", "tools/release-archive/", "tools/review-build/",
         # ONB-002: the baseline inventory is a build input (the bundled files come from anchors/).
         "baselines/", "anchors/")


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def inputs(out):
    raw = subprocess.run(["git", "status", "--porcelain=v1", "-uall", "-z"], check=True, capture_output=True).stdout.decode("utf-8")
    items = raw.split("\0")
    rows = []
    i = 0
    while i < len(items):
        e = items[i]
        i += 1
        if not e:
            continue
        status, path = e[:2], e[3:]
        if status[0] in "RC":
            i += 1  # the rename source follows
        if not path.startswith(ROOTS) and path not in ROOTS:
            continue
        h = sha(path) if os.path.isfile(path) else ""
        rows.append((status, path, h))
    rows.sort(key=lambda r: r[1])
    text = "status\tpath\tsha256\n" + "".join(f"{s}\t{p}\t{h}\n" for s, p, h in rows)
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print(len(rows), hashlib.sha256(text.encode("utf-8")).hexdigest())


def tree(d, out):
    rows = []
    for base, _, files in os.walk(d):
        for fn in files:
            p = os.path.join(base, fn)
            rel = os.path.relpath(p, d).replace("\\", "/")
            rows.append((rel, os.path.getsize(p), sha(p)))
    rows.sort()
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write("path\tbytes\tsha256\n" + "".join(f"{r}\t{b}\t{h}\n" for r, b, h in rows))
    tree_sha = hashlib.sha256("".join(f"{r}\t{h}\n" for r, _, h in rows).encode("utf-8")).hexdigest()
    print(len(rows), tree_sha)


def compare(a, b):
    la = open(a, encoding="utf-8").read().splitlines()
    lb = open(b, encoding="utf-8").read().splitlines()
    if la == lb:
        print("identical", len(la) - 1)
        return 0
    sa, sb = set(la), set(lb)
    for line in sorted(sa - sb):
        print("-", line)
    for line in sorted(sb - sa):
        print("+", line)
    return 1


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "inputs" and len(sys.argv) == 3:
        inputs(sys.argv[2])
    elif cmd == "tree" and len(sys.argv) == 4:
        tree(sys.argv[2], sys.argv[3])
    elif cmd == "compare" and len(sys.argv) == 4:
        sys.exit(compare(sys.argv[2], sys.argv[3]))
    else:
        print(__doc__)
        sys.exit(2)
