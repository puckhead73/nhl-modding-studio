"""Synthetic pages for scripts/wiki_check.py; plain ``python -m unittest`` (no Blender)."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wiki_check  # noqa: E402

GOOD_SIDEBAR = """### Start
- [Home](Home)
- [[Core Concepts]]

### Modding knowledge base
- [TDB format](TDB-Database-Format)
"""


def write(root: Path, name: str, text: str) -> None:
    (root / name).write_text(text, encoding="utf-8")


class WikiCheckTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="wiki-check-")
        self.root = Path(self.temp.name)
        write(self.root, "_Sidebar.md", GOOD_SIDEBAR)
        write(self.root, "_Footer.md", "footer\n")
        write(self.root, "Home.md", "# Home\n\nSee [concepts](Core-Concepts#projects).\n\nLast reviewed: 2026-10-20\n")
        write(self.root, "Core-Concepts.md", "# Concepts\n\n[[Home]]\n\nLast reviewed: 2026-10-20\n")
        write(self.root, "TDB-Database-Format.md", "# TDB\n\n**Evidence:** measured on the PS3 NHL 14 anchor.\n\nLast reviewed: 2026-10-20\n")

    def tearDown(self):
        self.temp.cleanup()

    def messages(self):
        return [f.message for f in wiki_check.run(self.root)]

    def test_clean_tree_has_no_findings(self):
        self.assertEqual(self.messages(), [])

    def test_broken_and_suffixed_links(self):
        write(self.root, "Home.md", "# Home\n\n[x](Missing-Page) [y](Core-Concepts.md) [z](../docs/DESIGN.md)\n\nLast reviewed: 2026-10-20\n")
        msgs = self.messages()
        self.assertTrue(any("broken link to page 'Missing-Page'" in m for m in msgs))
        self.assertTrue(any("drop the .md suffix" in m for m in msgs))
        self.assertTrue(any("points outside the wiki" in m for m in msgs))

    def test_links_inside_code_fences_and_spans_are_ignored(self):
        write(self.root, "Home.md", "# Home\n\n```\n[x](Not-A-Page)\n```\n\nWrite `[y](Also-Not-A-Page)`.\n\nLast reviewed: 2026-10-20\n")
        self.assertEqual(self.messages(), [])

    def test_unlinked_page_and_missing_review_line(self):
        write(self.root, "Orphan.md", "# Orphan\n")
        msgs = self.messages()
        self.assertIn("page is not linked from _Sidebar.md (directly or through an index page)", msgs)
        self.assertIn("missing 'Last reviewed: YYYY-MM-DD' line", msgs)

    def test_pages_reached_through_an_index_page_count_as_linked(self):
        write(self.root, "TDB-Database-Format.md", "# TDB\n\n**Evidence:** measured.\n\nSee [sub](Schema-NHL-14-PS3).\n\nLast reviewed: 2026-10-20\n")
        write(self.root, "Schema-NHL-14-PS3.md", "# Schema\n\nLast reviewed: 2026-10-20\n")
        msgs = self.messages()
        self.assertNotIn("page is not linked from _Sidebar.md (directly or through an index page)", msgs)
        # Reached only from the knowledge section, so it is a knowledge page and needs evidence.
        self.assertIn("knowledge page lacks an '**Evidence:** ...' line", msgs)
        # A user-guide page linked from a knowledge page keeps its own section.
        write(self.root, "TDB-Database-Format.md", "# TDB\n\n**Evidence:** measured.\n\nSee [c](Core-Concepts) and [sub](Schema-NHL-14-PS3).\n\nLast reviewed: 2026-10-20\n")
        write(self.root, "Schema-NHL-14-PS3.md", "# Schema\n\n**Evidence:** measured.\n\nLast reviewed: 2026-10-20\n")
        self.assertEqual(self.messages(), [])

    def test_knowledge_page_needs_evidence_line(self):
        write(self.root, "TDB-Database-Format.md", "# TDB\n\nLast reviewed: 2026-10-20\n")
        self.assertIn("knowledge page lacks an '**Evidence:** ...' line", self.messages())

    def test_banned_content_is_reported_even_in_code(self):
        bad = "\n".join([
            "# Home", "",
            "```", "E:\\nhl-studio-fixtures\\x", "```",
            "hash " + "a" * 64,
            "see nhl-database-studio and local/runs/1",
            "the NHL 27 build on PS5 (PPSA34063)",
            "patch the eboot",
            "decrypt the executable first",
            "", "Last reviewed: 2026-10-20", "",
        ])
        write(self.root, "Home.md", bad)
        msgs = "\n".join(self.messages())
        for reason in ("local drive path", "private fixture root", "SHA-256 hash", "prior tool repository",
                       "internal process path", "unreleased or out-of-scope title", "PS5 platform",
                       "PS5 product id", "executable internals", "executable decryption"):
            self.assertIn(reason, msgs)

    def test_public_identifiers_and_urls_are_allowed(self):
        write(self.root, "Home.md", "# Home\n\nBLUS31179, CUSA18182, default.xex, dev_hdd0/game, a decrypted PS3 ISO, "
              "https://github.com/puckhead73/nhl-modding-studio, NHL 21, APP_VER 01.60\n\nLast reviewed: 2026-10-20\n")
        self.assertEqual(self.messages(), [])

    def test_images_must_exist_under_images(self):
        write(self.root, "Home.md", "# Home\n\n![a](images/start.png) ![b](start.png)\n\nLast reviewed: 2026-10-20\n")
        msgs = self.messages()
        self.assertTrue(any("missing image 'images/start.png'" in m for m in msgs))
        self.assertTrue(any("must live under images/" in m for m in msgs))
        (self.root / "images").mkdir()
        (self.root / "images" / "start.png").write_bytes(b"\x89PNG")
        write(self.root, "Home.md", "# Home\n\n![a](images/start.png)\n\nLast reviewed: 2026-10-20\n")
        self.assertEqual(self.messages(), [])

    def test_folders_are_refused(self):
        (self.root / "sub").mkdir()
        self.assertTrue(any("folders are not allowed" in m for m in self.messages()))


if __name__ == "__main__":
    unittest.main()
