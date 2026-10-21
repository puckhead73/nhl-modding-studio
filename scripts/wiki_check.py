"""Lint the public wiki sources under ``wiki/`` before they are mirrored to GitHub.

Checks, in order:

1. Layout: every page is a flat ``Name.md`` file directly under the wiki root
   (the GitHub Wiki flattens folders); the only allowed folder is ``images/``.
2. Links: every ``[text](Target)`` and ``[[Target]]`` link resolves to an existing
   page or image. Page links must not carry a ``.md`` suffix.
3. Sidebar: every page is linked from ``_Sidebar.md``.
4. Banned content: private paths, file hashes, other tool repositories,
   unreleased titles and executable internals never reach the public wiki.
5. Page metadata: every page carries a ``Last reviewed: YYYY-MM-DD`` line, and
   every page listed under the knowledge-base section of the sidebar carries an
   ``**Evidence:**`` line.

Exit code 0 when clean, 1 when any finding is reported. The script never writes.
"""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_WIKI = REPO_ROOT / "wiki"

SPECIAL_PAGES = {"_Sidebar", "_Footer"}
ALLOWED_DIRS = {"images"}
IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg"}
KNOWLEDGE_HEADING = re.compile(r"knowledge", re.IGNORECASE)

# Each entry: (compiled pattern, reason shown to the author).
BANNED: list[tuple[re.Pattern[str], str]] = [
    (re.compile(r"\b[A-Za-z]:[\\/](?![\\/])"), "local drive path"),
    (re.compile(r"nhl-studio-fixtures", re.I), "private fixture root"),
    (re.compile(r"\b[0-9a-fA-F]{64}\b"), "SHA-256 hash of a file"),
    (re.compile(r"\blocal/(runs|coordination)\b"), "internal process path"),
    (re.compile(r"nhl-database-studio", re.I), "prior tool repository"),
    (re.compile(r"rexglue", re.I), "outside SDK repository"),
    (re.compile(r"artemkh", re.I), "third-party author site"),
    (re.compile(r"Users[\\/]puckh\b"), "user home directory"),
    (re.compile(r"\bNHL\s?2[5-9]\b"), "unreleased or out-of-scope title"),
    (re.compile(r"\bPPSA\d*\b"), "PS5 product id"),
    (re.compile(r"\bPS5\b"), "PS5 platform (out of scope)"),
    (re.compile(r"\beboot\b", re.I), "executable internals"),
    (re.compile(r"\bxextool\b|\bxex2?\s+key", re.I), "executable decryption"),
    (re.compile(r"\bdecrypt\w*\s+(?:the\s+|an?\s+)?(?:eboot|xex|executable|elf|game\s+binary)", re.I), "executable decryption"),
]

LINK_INLINE = re.compile(r"(?<!\!)\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
IMAGE_INLINE = re.compile(r"\!\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
LINK_WIKI = re.compile(r"\[\[([^\]]+)\]\]")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s+(.*?)\s*$")
LAST_REVIEWED = re.compile(r"^Last reviewed:\s+\d{4}-\d{2}-\d{2}\s*$", re.M)
EVIDENCE = re.compile(r"^\*\*Evidence:\*\*\s+\S", re.M)
FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})")
INLINE_CODE = re.compile(r"`[^`\n]*`")


@dataclass(frozen=True)
class Finding:
    page: str
    line: int
    message: str

    def __str__(self) -> str:
        return f"{self.page}:{self.line}: {self.message}"


def strip_fences(text: str) -> list[tuple[int, str]]:
    """Return (line_number, line) pairs with fenced code blocks blanked out.

    Code blocks and inline code spans keep their line numbers so later findings
    still point at the right place, but their content is not scanned for links
    or headings (an example link inside backticks is documentation, not a link).
    Banned-content scanning uses the raw text instead: a hash or path inside a
    code block is just as public.
    """
    out: list[tuple[int, str]] = []
    fence: str | None = None
    for number, line in enumerate(text.splitlines(), start=1):
        match = FENCE.match(line)
        if match:
            marker = match.group(1)[0]
            if fence is None:
                fence = marker
            elif marker == fence:
                fence = None
            out.append((number, ""))
            continue
        out.append((number, "" if fence else INLINE_CODE.sub("", line)))
    return out


def page_name_from_wiki_link(target: str) -> str:
    # ``[[Label|Page Name]]`` and ``[[Page Name]]`` both resolve to ``Page-Name``.
    if "|" in target:
        target = target.split("|", 1)[1]
    return target.strip().replace(" ", "-")


def is_external(target: str) -> bool:
    lowered = target.lower()
    return lowered.startswith(("http://", "https://", "mailto:")) or target.startswith("#")


def check_layout(root: Path) -> tuple[list[Finding], dict[str, Path]]:
    findings: list[Finding] = []
    pages: dict[str, Path] = {}
    for entry in sorted(root.iterdir()):
        if entry.is_dir():
            if entry.name not in ALLOWED_DIRS:
                findings.append(Finding(entry.name, 0, "folders are not allowed; the GitHub Wiki flattens them"))
            continue
        if entry.suffix != ".md":
            findings.append(Finding(entry.name, 0, "only .md pages belong at the wiki root"))
            continue
        name = entry.stem
        if name != name.strip() or " " in name:
            findings.append(Finding(entry.name, 0, "page file names use hyphens, never spaces"))
        pages[name] = entry
    images_dir = root / "images"
    if images_dir.exists():
        for entry in sorted(images_dir.rglob("*")):
            if entry.is_file() and entry.suffix.lower() not in IMAGE_SUFFIXES:
                findings.append(Finding(str(entry.relative_to(root)), 0, "images/ holds image files only"))
    return findings, pages


def check_links(root: Path, name: str, lines: list[tuple[int, str]], pages: dict[str, Path]) -> list[Finding]:
    findings: list[Finding] = []
    for number, line in lines:
        for match in IMAGE_INLINE.finditer(line):
            target = match.group(2)
            if is_external(target):
                continue
            if not target.startswith("images/"):
                findings.append(Finding(name, number, f"image '{target}' must live under images/"))
            elif not (root / target).is_file():
                findings.append(Finding(name, number, f"missing image '{target}'"))
        for match in LINK_INLINE.finditer(line):
            target = match.group(2)
            if is_external(target):
                continue
            target = target.split("#", 1)[0]
            if not target:
                continue
            if target.startswith("images/"):
                if not (root / target).is_file():
                    findings.append(Finding(name, number, f"missing image '{target}'"))
                continue
            if "/" in target or "\\" in target:
                findings.append(Finding(name, number, f"link '{target}' points outside the wiki; use a page name or an https URL"))
                continue
            if target.endswith(".md"):
                findings.append(Finding(name, number, f"link '{target}' must drop the .md suffix (wiki page names only)"))
                continue
            if target not in pages:
                findings.append(Finding(name, number, f"broken link to page '{target}'"))
        for match in LINK_WIKI.finditer(line):
            target = page_name_from_wiki_link(match.group(1))
            if target not in pages:
                findings.append(Finding(name, number, f"broken wiki link to page '{target}'"))
    return findings


def sidebar_sections(lines: list[tuple[int, str]]) -> dict[str, set[str]]:
    """Map each sidebar heading to the page names linked beneath it."""
    sections: dict[str, set[str]] = {}
    current = ""
    sections[current] = set()
    for _, line in lines:
        heading = HEADING.match(line)
        if heading:
            current = heading.group(1)
            sections.setdefault(current, set())
            continue
        for match in LINK_INLINE.finditer(line):
            target = match.group(2)
            if not is_external(target):
                sections[current].add(target.split("#", 1)[0])
        for match in LINK_WIKI.finditer(line):
            sections[current].add(page_name_from_wiki_link(match.group(1)))
    return sections


def check_banned(name: str, text: str) -> list[Finding]:
    findings: list[Finding] = []
    for number, line in enumerate(text.splitlines(), start=1):
        for pattern, reason in BANNED:
            match = pattern.search(line)
            if match:
                findings.append(Finding(name, number, f"banned content ({reason}): '{match.group(0)}'"))
    return findings


def run(root: Path) -> list[Finding]:
    if not root.is_dir():
        return [Finding(str(root), 0, "wiki root does not exist")]
    findings, pages = check_layout(root)
    if "_Sidebar" not in pages:
        findings.append(Finding("_Sidebar.md", 0, "missing sidebar"))
    if "Home" not in pages:
        findings.append(Finding("Home.md", 0, "missing Home page"))

    texts = {name: path.read_text(encoding="utf-8") for name, path in pages.items()}
    stripped = {name: strip_fences(text) for name, text in texts.items()}

    for name in sorted(pages):
        findings.extend(check_links(root, name, stripped[name], pages))
        findings.extend(check_banned(name, texts[name]))
        if name not in SPECIAL_PAGES and not LAST_REVIEWED.search(texts[name]):
            findings.append(Finding(name, 0, "missing 'Last reviewed: YYYY-MM-DD' line"))

    if "_Sidebar" in pages:
        sections = sidebar_sections(stripped["_Sidebar"])
        # A page is reachable when the sidebar links it directly or an index page the sidebar
        # links leads to it. Reachability is computed per sidebar section and stops at pages that
        # belong directly to another section, so a knowledge page linking into the user guide does
        # not make the user guide "knowledge".
        direct_owner: dict[str, str] = {}
        for heading, names in sections.items():
            for n in names:
                direct_owner.setdefault(n, heading)
        reach: dict[str, set[str]] = {}
        for heading, names in sections.items():
            seen = set(names)
            frontier = list(names)
            while frontier:
                current = frontier.pop()
                for target in page_links(stripped.get(current, []), pages):
                    if target in seen or direct_owner.get(target, heading) != heading:
                        continue
                    seen.add(target)
                    frontier.append(target)
            reach[heading] = seen
        linked = set().union(*reach.values()) if reach else set()
        for name in sorted(pages):
            if name in SPECIAL_PAGES:
                continue
            if name not in linked:
                findings.append(Finding(name, 0, "page is not linked from _Sidebar.md (directly or through an index page)"))
        knowledge_pages = set()
        for heading, names in reach.items():
            if KNOWLEDGE_HEADING.search(heading):
                knowledge_pages |= names
        for name in sorted(knowledge_pages):
            if name in texts and not EVIDENCE.search(texts[name]):
                findings.append(Finding(name, 0, "knowledge page lacks an '**Evidence:** ...' line"))
    return findings


def page_links(lines: list[tuple[int, str]], pages: dict[str, Path]) -> set[str]:
    """The existing wiki pages a page links to (inline and wiki-style links)."""
    targets: set[str] = set()
    for _, line in lines:
        for match in LINK_INLINE.finditer(line):
            target = match.group(2)
            if not is_external(target):
                target = target.split("#", 1)[0]
                if target in pages:
                    targets.add(target)
        for match in LINK_WIKI.finditer(line):
            target = page_name_from_wiki_link(match.group(1))
            if target in pages:
                targets.add(target)
    return targets


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=DEFAULT_WIKI, help="wiki source folder (default: wiki/)")
    args = parser.parse_args(argv)
    findings = run(args.root)
    for finding in sorted(findings, key=lambda f: (f.page, f.line, f.message)):
        print(finding)
    pages = len([p for p in args.root.glob("*.md")]) if args.root.is_dir() else 0
    print(f"wiki_check: {pages} pages, {len(findings)} finding(s)")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
