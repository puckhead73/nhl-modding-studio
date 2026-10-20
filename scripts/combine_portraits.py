"""Copy game-specific portrait exports into one folder with game suffixes.

Run without arguments to enter the source and destination folders, or use --dry-run
to validate names and display counts without writing anything. Python 3.9+.
"""

import argparse
import hashlib
import shutil
from collections import Counter
from pathlib import Path


def prompt_folder(label):
    while True:
        # Accept quoted paths pasted from Windows Explorer.
        value = input(f"{label}: ").strip().strip('"').strip("'")
        if value:
            return Path(value).expanduser()
        print("Please enter a folder path.")


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(block)
    return checksum.digest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    source = (args.source or prompt_folder("Source folder containing the game folders")).resolve()
    destination = (args.destination or prompt_folder("Destination folder for combined portraits")).resolve()
    if not source.is_dir():
        parser.error(f"Source folder does not exist: {source}")
    if destination == source or source in destination.parents:
        parser.error("Destination must be outside the source tree.")
    if destination.exists() and not destination.is_dir():
        parser.error(f"Destination is not a folder: {destination}")

    # Preflight the entire flattened namespace before copying any portraits.
    plan = []
    names = {}
    counts = Counter()
    for game in sorted(source.iterdir()):
        if not game.is_dir() or not game.name.lower().startswith("nhl"):
            continue
        for portrait in sorted(game.rglob("*")):
            if not portrait.is_file() or portrait.suffix.lower() != ".png":
                continue
            output = destination / f"{portrait.stem}_{game.name}{portrait.suffix}"
            key = output.name.casefold()
            names[key] = names.get(key, 0) + 1
            plan.append((portrait, output))
            counts[game.name] += 1
    if not plan:
        parser.error("No PNG portraits found under NHL game folders.")

    # Some games contain both pngs/ and portraits/. Preserve both variants,
    # adding the subfolder only for ambiguous names, before the game suffix.
    resolved = []
    used = set()
    for portrait, output in plan:
        if names[output.name.casefold()] > 1:
            relative = portrait.relative_to(source)
            folder = "_".join(relative.parts[1:-1])
            output = destination / f"{portrait.stem}_{folder}_{relative.parts[0]}{portrait.suffix}"
        key = output.name.casefold()
        if key in used:
            parser.error(f"Unresolved filename collision: {output.name}")
        used.add(key)
        resolved.append((portrait, output))
    plan = resolved

    # Existing identical copies are skipped; differing files are never overwritten.
    pending = []
    skipped = 0
    for portrait, output in plan:
        if output.exists():
            if (not output.is_file()
                    or portrait.stat().st_size != output.stat().st_size
                    or digest(portrait) != digest(output)):
                parser.error(f"Destination already contains a different file: {output}")
            skipped += 1
        else:
            pending.append((portrait, output))

    for game, count in counts.items():
        print(f"{game}: {count:,}")
    print(f"Destination: {destination}")
    print(f"Total: {len(plan):,}; to copy: {len(pending):,}; identical existing: {skipped:,}")
    if args.dry_run:
        print("Dry run complete. No files written.")
        return

    destination.mkdir(parents=True, exist_ok=True)
    for index, (portrait, output) in enumerate(pending, start=1):
        # Exclusive creation also protects against files appearing after preflight.
        with portrait.open("rb") as input_stream, output.open("xb") as output_stream:
            try:
                shutil.copyfileobj(input_stream, output_stream, length=1024 * 1024)
            except BaseException:
                output_stream.close()
                output.unlink(missing_ok=True)
                raise
        shutil.copystat(portrait, output)
        if index % 1000 == 0 or index == len(pending):
            print(f"Copied {index:,}/{len(pending):,}", flush=True)
    print(f"Done. Copied {len(pending):,}; skipped {skipped:,}. Originals preserved.")


if __name__ == "__main__":
    main()
