"""C0-POR1-R1 specimen tool: hashing, header measurement and the cross-platform comparison.

Read-only. Records exact specimen paths, SHA-256 digests, measured header fields and the block
stream comparison outputs. Never writes to a source location.

Usage (from the repository root):

    python docs/receipts/data/c0-por1-tools/por1_specimens.py <out_dir> [--compare]

`--compare` re-runs the PS3/Xbox 360 block-stream comparison over the recorded id sample.

WHAT THE COMPARISON DOES AND DOES NOT SHOW
------------------------------------------
Both chains share `parse_bigf`, `refpack_decode` and (for the pixel form) `decode_dxt5`. Agreement
therefore exercises only the stages that differ -- `parse_xpr2`, `swap16` and the transcribed
tiling formula -- and cannot detect a fault common to both chains.

It also says NOTHING about whether either tree is authentic, unmodified, or the set the game
ships. It is a statement about the specimens compared, and only those.
"""

import hashlib
import json
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import por1_formats as F  # noqa: E402

PS3 = r"H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\fe\ion\artassets"
X360 = r"H:\Emulators\games\XBOX\NHL Legacy - Vanilla\_compiled\fe\ion\artassets"
PS4 = r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\ui\kettle_assets\playerportraits"

# (location, path relative to that location's root). Every specimen inspected by C0-POR1.
SPECIMENS = [
    ("x360", "playerheads/p0_4000/p1008.big"),
    ("ps3", "playerheads/p0_4000/p1008.big"),
    ("x360", "playerheadssmall/p0_4000/p1008.big"),
    ("ps3", "playerheadssmall/p0_4000/p1008.big"),
    ("ps3", "playerheads/p0_4000/p1.big"),
    ("ps3", "playerheads/p0_4000/p1000.big"),
    ("ps3", "playerheadssmall/p0_4000/p1.big"),
    ("x360", "playerheads/p0_4000/p1.big"),
    ("x360", "playerheads/p0_4000/p1000.big"),
    ("x360", "playerheadssmall/p0_4000/p1.big"),
    ("x360", "playerheads/p0_4000/p104.big"),
    ("ps3", "playerheads/p0_4000/p104.big"),
    ("ps3", "playerheadssmall/silhouettes/p60000.big"),
    ("x360", "playerheadssmall/silhouettes/p60000.big"),
    ("ps3", "playerheads/p0_4000/p1070.big"),
    ("ps3", "playerheads/p0_4000/p1070.big.bak"),
    ("ps3", "playerheadssmall/p0_4000/p127.big"),
    ("ps3", "playerheadssmall/p0_4000/p127.big.bak"),
    ("x360", "playerheadssmall/p0_4000/p127.big"),
    ("x360", "playerheadssmall/p0_4000/p1112_glump.options"),
    ("ps4", "p100.gnf"),
    ("ps4", "p1.gnf"),
]

ROOTS = {"ps3": PS3, "x360": X360, "ps4": PS4}


def full(loc, rel):
    return os.path.join(ROOTS[loc], *rel.split("/"))


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def linear_blocks(loc, path):
    """Return (width, height, row-major DXT5 block stream) for one Gen7 archive."""
    with open(path, "rb") as f:
        data = f.read()
    header = F.parse_bigf(data)
    _, _, payload = F.texture_payload(data, header)
    if payload is None:
        return None
    if payload[:4] == b"DDS ":
        d = F.parse_dds(payload)
        return d["width"], d["height"], payload[d["payload_offset"]:]
    x = F.parse_xpr2(payload)
    raw = F.swap16(payload[x["payload_offset"]:])
    return x["width"], x["height"], F.untile_dxt(raw, x["width"], x["height"])


def compare(out):
    """PS3 vs Xbox 360 block-stream comparison over a seeded id sample from one bucket."""
    results = []
    for folder in ("playerheads", "playerheadssmall"):
        sub = "p0_4000"
        names = sorted(n for n in os.listdir(os.path.join(X360, folder, sub)) if n.endswith(".big"))
        random.seed(7)
        for name in random.sample(names, 25):
            p3 = os.path.join(PS3, folder, sub, name)
            px = os.path.join(X360, folder, sub, name)
            if not os.path.exists(p3):
                results.append({"folder": folder, "file": name, "outcome": "ps3 file absent"})
                continue
            a = linear_blocks("ps3", p3)
            c = linear_blocks("x360", px)
            row = {
                "folder": folder, "file": name,
                "ps3_dimensions": [a[0], a[1]], "x360_dimensions": [c[0], c[1]],
                "ps3_has_backup_sibling": os.path.exists(p3 + ".bak"),
                "identical_block_stream": a[:2] == c[:2] and a[2] == c[2],
            }
            if not row["identical_block_stream"] and row["ps3_has_backup_sibling"]:
                b = linear_blocks("ps3", p3 + ".bak")
                row["backup_matches_x360"] = b[:2] == c[:2] and b[2] == c[2]
            results.append(row)
    out["comparison"] = {
        "method": "seeded (seed=7) sample of 25 ids per folder from the p0_4000 bucket only",
        "shared_stages": ["parse_bigf", "refpack_decode"],
        "stages_exercised": ["parse_xpr2", "swap16", "xbox360_tiled_offset (transcribed from tdb-rx2)"],
        "establishes": ("that for these specimens the Xbox 360-specific stages yield the same block "
                        "stream as the PS3 file. Not authenticity, not provenance, not target scope."),
        "results": results,
        "summary": {
            "compared": sum(1 for r in results if "identical_block_stream" in r),
            "identical": sum(1 for r in results if r.get("identical_block_stream")),
            "differing": sum(1 for r in results if r.get("identical_block_stream") is False),
        },
    }


def main(out_dir, do_compare):
    os.makedirs(out_dir, exist_ok=True)
    here = os.path.dirname(os.path.abspath(__file__))
    out = {
        "tool": "por1_specimens.py",
        "tool_sha256": {
            n: hashlib.sha256(open(os.path.join(here, n), "rb").read()).hexdigest()
            for n in ("por1_formats.py", "por1_specimens.py")
        },
        "roots": ROOTS,
        "specimens": [],
    }
    for loc, rel in SPECIMENS:
        p = full(loc, rel)
        out["specimens"].append({
            "location": loc,
            "path_relative_to_root": rel,
            "bytes": os.path.getsize(p),
            "sha256": sha256(p),
            "group": "backup-labelled" if rel.endswith(".bak")
                     else ("other" if rel.endswith(".options") else "current"),
        })
    if do_compare:
        compare(out)

    path = os.path.join(out_dir, "c0-por1-specimen-hashes.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1)
    print(json.dumps(out.get("comparison", {}).get("summary", {}), indent=1))
    print("specimens:", len(out["specimens"]), "->", path)


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    main(args[0] if args else ".", "--compare" in sys.argv)
