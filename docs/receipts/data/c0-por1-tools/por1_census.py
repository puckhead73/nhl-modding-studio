"""C0-POR1-R1 census: exact, reconciled counts over the three supplied portrait roots.

Read-only. Opens every file in the five inventoried folders, classifies it, and parses its
container and texture headers. Writes aggregate JSON only -- never file contents, never a
per-file listing of private data beyond one example relative path per observed group.

Usage (from the repository root):

    python docs/receipts/data/c0-por1-tools/por1_census.py <out_dir>

Roots are read from POR1_ROOTS below. They are private, uncommitted, user-supplied locations.

WHAT "accepted" MEANS. A file counted under `accepted_by_inspection_parser` is one where
`por1_formats` found the fields it looks for at the offsets it expects. That is not semantic
validation and not proof the file is well-formed for the game.
"""

import collections
import hashlib
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import por1_formats as F  # noqa: E402

POR1_ROOTS = {
    "ps3-nhllegacy": {
        "root": r"H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\fe\ion\artassets",
        "folders": ["playerheads", "playerheadssmall"],
        "kind": "gen7",
    },
    "x360-nhllegacy": {
        "root": r"H:\Emulators\games\XBOX\NHL Legacy - Vanilla\_compiled\fe\ion\artassets",
        "folders": ["playerheads", "playerheadssmall"],
        "kind": "gen7",
    },
    "ps4-nhl21": {
        "root": r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\ui\kettle_assets",
        "folders": ["playerportraits"],
        "kind": "ps4",
    },
}

CURRENT_EXT = {".big", ".gnf"}
BACKUP_EXT = {".bak"}


def group_of(name):
    ext = os.path.splitext(name)[1].lower()
    if ext in CURRENT_EXT:
        return "current"
    if ext in BACKUP_EXT:
        return "backup-labelled"
    return "other"


def member_pattern(members):
    """Normalise member names so per-archive resource ids do not fragment the tuple."""
    out = []
    for m in members:
        n = m["name"]
        out.append("<num>" if n.isdigit() and n not in ("0", "1") else n)
    return tuple(out)


def gen7_file(path):
    with open(path, "rb") as f:
        data = f.read()
    header = F.parse_bigf(data)
    if header is None:
        return {"accepted": False, "reason": "no BIGF magic", "first4": data[:4].hex()}

    texture_members = 0
    tex = None
    for m in header["members"]:
        if m["length"] < 64 or data[m["offset"]:m["offset"] + 2] != F.REFPACK_MAGIC:
            continue
        try:
            _, head = F.refpack_decode(data[m["offset"]:m["offset"] + m["length"]], limit=200)
        except Exception:
            continue
        if head[:4] in (b"DDS ", b"XPR2"):
            texture_members += 1
            if tex is None or m["length"] > tex[0]["length"]:
                tex = (m, head)

    rec = {
        "accepted": True,
        "member_count": header["member_count"],
        "member_pattern": member_pattern(header["members"]),
        "header_size": header["header_size"],
        "size_field_matches_len_minus_16": header["size_field_le"] == len(data) - 16,
        "texture_members": texture_members,
    }
    if tex is None:
        rec["texture"] = None
        return rec

    head = tex[1]
    if head[:4] == b"DDS ":
        d = F.parse_dds(head)
        rec["texture"] = {
            "container": "dds",
            "width": d["width"], "height": d["height"],
            "codec": d["fourcc"], "mip_count": d["mip_count"],
            "caps": f"0x{d['caps']:x}",
            "linear_size": d["pitch_or_linear_size"],
        }
    else:
        x = F.parse_xpr2(head)
        rec["texture"] = {
            "container": "xpr2",
            "width": x["width"], "height": x["height"],
            "data_format_bits": x["data_format_bits"],
            "tiled_bit": x["tiled_bit"],
            "pitch_texels": x["pitch_texels"],
            "mip_descriptor_dword": x["mip_descriptor_dword"],
            "data_size": x["data_size"],
        }
    return rec


def ps4_file(path):
    with open(path, "rb") as f:
        data = f.read(256)
    size = os.path.getsize(path)
    g = F.parse_gnf(data)
    if g is None:
        return {"accepted": False, "reason": "no GNF magic", "first4": data[:4].hex()}
    return {
        "accepted": True,
        "file_bytes": size,
        "texture_members": 1,
        "texture": {
            "container": "gnf",
            "width": g["width"], "height": g["height"],
            "data_format": g["data_format"],
            "num_format": g["num_format"],
            "dst_sel_x": g["dst_sel_x"],
            "tiling_index": g["tiling_index"],
            "last_level": g["last_level"],
            "version": g["version"],
            "contents_size": g["contents_size"],
        },
    }


def main(out_dir):
    os.makedirs(out_dir, exist_ok=True)
    report = {
        "tool": "por1_census.py",
        "tool_sha256": {
            n: hashlib.sha256(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), n), "rb").read()).hexdigest()
            for n in ("por1_formats.py", "por1_census.py")
        },
        "note": "Counts only. 'accepted' means accepted by this inspection parser, not semantically validated.",
        "roots": {k: v["root"] for k, v in POR1_ROOTS.items()},
        "inventory": [],
        "structural_groups": [],
        "totals": {},
    }

    grand_files = grand_accepted = grand_textures = 0

    for loc, spec in POR1_ROOTS.items():
        for folder in spec["folders"]:
            base = os.path.join(spec["root"], folder)
            by_ext = collections.Counter()
            by_group = collections.Counter()
            by_sub = collections.Counter()
            tuples = collections.Counter()
            examples = {}
            files = accepted = textures = 0
            not_accepted = []

            for dirpath, _, names in os.walk(base):
                sub = os.path.relpath(dirpath, base)
                sub = "." if sub == "." else sub
                for name in sorted(names):
                    path = os.path.join(dirpath, name)
                    rel = os.path.relpath(path, base).replace("\\", "/")
                    files += 1
                    ext = os.path.splitext(name)[1].lower() or "<none>"
                    grp = group_of(name)
                    by_ext[ext] += 1
                    by_group[grp] += 1
                    by_sub[sub] += 1

                    rec = ps4_file(path) if spec["kind"] == "ps4" else gen7_file(path)
                    if not rec["accepted"]:
                        not_accepted.append({"path": rel, **rec})
                        continue
                    accepted += 1
                    textures += rec.get("texture_members", 0)

                    key = json.dumps({
                        "group": grp,
                        "member_count": rec.get("member_count"),
                        "member_pattern": rec.get("member_pattern"),
                        "texture_members": rec.get("texture_members"),
                        "texture": rec.get("texture"),
                    }, sort_keys=True)
                    tuples[key] += 1
                    examples.setdefault(key, rel)

            report["inventory"].append({
                "location": loc, "folder": folder,
                "files": files,
                "by_extension": dict(sorted(by_ext.items())),
                "by_group": dict(sorted(by_group.items())),
                "by_subfolder": dict(sorted(by_sub.items())),
                "accepted_by_inspection_parser": accepted,
                "not_accepted": not_accepted,
                "texture_payloads_found": textures,
            })
            for key, n in tuples.most_common():
                report["structural_groups"].append({
                    "location": loc, "folder": folder, "count": n,
                    "example": examples[key], **json.loads(key),
                })
            grand_files += files
            grand_accepted += accepted
            grand_textures += textures

    report["totals"] = {
        "files_inventoried": grand_files,
        "accepted_by_inspection_parser": grand_accepted,
        "texture_payloads_found": grand_textures,
    }

    path = os.path.join(out_dir, "c0-por1-census.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=1, sort_keys=False)
    print(json.dumps(report["totals"], indent=1))
    for row in report["inventory"]:
        print(row["location"], row["folder"], row["files"], row["by_group"],
              "accepted", row["accepted_by_inspection_parser"],
              "textures", row["texture_payloads_found"],
              "not_accepted", len(row["not_accepted"]))
    print("wrote", path)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else ".")
