"""Stage only appearance members from a saved archive inventory for decoder QA.

Usage: python scripts/stage_appearance_fixtures.py INVENTORY_DIR NEW_OUTPUT_DIR
Reads original archives; never modifies a game or overwrites an existing fixture.
"""
import json
import re
import sys
from pathlib import Path


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    inventory, output = map(Path, sys.argv[1:])
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    folders = re.compile(r"^rendering/(player|player_common|coachhead|coach|referee|trainer|commissioner|timekeeper|czhead|czhair)/")
    for listing in sorted(inventory.glob("*-xbox360.jsonl")):
        rows = []
        with listing.open(encoding="utf-8") as source:
            for line in source:
                row = json.loads(line)
                if row["kind"] == "archive_member" and folders.match(row["name"]) and row["name"].endswith(".rx2"):
                    rows.append(row)
        rows.sort(key=lambda row: (row["physical"], row["physical_offset"]))
        total = 0
        opened = {}
        try:
            for row in rows:
                target = (output / listing.stem / row["name"]).resolve()
                if not target.is_relative_to(output):
                    raise ValueError("Archive member escapes fixture root")
                source = opened.get(row["physical"])
                if source is None:
                    source = opened[row["physical"]] = Path(row["physical"]).open("rb")
                source.seek(row["physical_offset"])
                data = source.read(row["size"])
                if len(data) != row["size"]:
                    raise ValueError("Truncated archive member")
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("xb") as dst:
                    dst.write(data)
                total += len(data)
        finally:
            for source in opened.values():
                source.close()
        print(f"{listing.stem}: {len(rows)} files, {total:,} bytes", flush=True)


if __name__ == "__main__":
    main()
