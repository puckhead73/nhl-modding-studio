"""Synthetic audit data for scripts/wiki_generate.py; plain ``python -m unittest`` (no Blender)."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wiki_generate  # noqa: E402

SNAPSHOT = {
    "game": "nhl14", "platform": "ps3", "generation": "gen7", "db": "nhlng.db", "endian": "big",
    "tdb_version": 6, "meta_present": True, "meta_file": "nhlng-meta.xml", "table_count": 1, "field_total": 2,
    "tables": [{
        "raw_id": "cPbu", "canonical_id": "ubPc", "name": "exhibitionplayerbiotable",
        "record_len_bytes": 8, "current_records": 10, "max_records": 10, "field_count": 2,
        "fields": [
            {"raw_id": "zIBw", "canonical_id": "wBIz", "name": "game_id", "kind_code": 3, "meta_type": "integer",
             "bit_offset": 0, "bit_width": 14, "range_low": 0, "range_high": 16383, "key": True, "updatable": False, "fk_parent": None},
            {"raw_id": "BSXd", "canonical_id": "dXSB", "name": "team", "kind_code": 3, "meta_type": "integer",
             "bit_offset": 14, "bit_width": 8, "range_low": -1, "range_high": 254, "key": False, "updatable": True, "fk_parent": "exhibitionteams"},
        ],
    }],
}
SAVE_CSV = "platform,game,slot,detected_type,codec,inner_tdb,semantic_edit,status\nps3,nhl14,BLUS31179-ROSTER,roster,v4,yes,yes,edit-roundtrip\n"
SURF_CSV = "platform,game,surface,state,reason\nps3,nhl14,players,editable,\nps3,nhl14,arenas,not-applicable,no arena table\n"


class GenerateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="wiki-gen-")
        root = Path(self.temp.name)
        self.data = root / "data"
        (self.data / "schema").mkdir(parents=True)
        (self.data / "schema" / "ps3-nhl14.json").write_text(json.dumps(SNAPSHOT), encoding="utf-8")
        (self.data / "save-matrix.csv").write_text(SAVE_CSV, encoding="utf-8")
        (self.data / "surface-capabilities.csv").write_text(SURF_CSV, encoding="utf-8")
        self.wiki = root / "wiki"
        self.wiki.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def test_check_reports_missing_then_write_then_clean(self):
        self.assertEqual(wiki_generate.main(["--check", "--data", str(self.data), "--wiki", str(self.wiki)]), 1)
        self.assertEqual(wiki_generate.main(["--write", "--data", str(self.data), "--wiki", str(self.wiki)]), 0)
        names = sorted(p.name for p in self.wiki.glob("*.md"))
        self.assertEqual(names, ["Save-Formats-by-Title.md", "Schema-Index.md", "Schema-NHL-14-PS3.md", "Surface-Capabilities-by-Title.md"])
        self.assertEqual(wiki_generate.main(["--check", "--data", str(self.data), "--wiki", str(self.wiki)]), 0)

    def test_pages_carry_banner_evidence_and_review_date(self):
        wiki_generate.main(["--write", "--data", str(self.data), "--wiki", str(self.wiki)])
        for p in self.wiki.glob("*.md"):
            text = p.read_text(encoding="utf-8")
            self.assertTrue(text.startswith(wiki_generate.BANNER), p.name)
            self.assertIn("**Evidence:**", text, p.name)
            self.assertIn(f"Last reviewed: {wiki_generate.REVIEW_DATE}", text, p.name)

    def test_schema_page_content(self):
        text = wiki_generate.render_schema(SNAPSHOT)
        self.assertIn("`ubPc (raw cPbu)`", text)
        self.assertIn("| team | `dXSB (BSXd)` | 8 | -1..254 | fk exhibitionteams |", text)
        self.assertIn("| game_id | `wBIz (zIBw)` | 14 | 0..16383 | key, not updated by the game |", text)

    def test_hand_written_page_blocks_write_and_edited_page_is_drift(self):
        (self.wiki / "Schema-Index.md").write_text("# mine\n", encoding="utf-8")
        self.assertEqual(wiki_generate.main(["--write", "--data", str(self.data), "--wiki", str(self.wiki)]), 2)
        self.assertEqual((self.wiki / "Schema-Index.md").read_text(encoding="utf-8"), "# mine\n")
        (self.wiki / "Schema-Index.md").unlink()
        wiki_generate.main(["--write", "--data", str(self.data), "--wiki", str(self.wiki)])
        page = self.wiki / "Schema-NHL-14-PS3.md"
        page.write_text(page.read_text(encoding="utf-8") + "\nedited\n", encoding="utf-8")
        self.assertEqual(wiki_generate.main(["--check", "--data", str(self.data), "--wiki", str(self.wiki)]), 1)

    def test_obsolete_owned_page_is_removed_on_write(self):
        (self.wiki / "Schema-Old.md").write_text(wiki_generate.BANNER + "\nold\n", encoding="utf-8")
        wiki_generate.main(["--write", "--data", str(self.data), "--wiki", str(self.wiki)])
        self.assertFalse((self.wiki / "Schema-Old.md").exists())


if __name__ == "__main__":
    unittest.main()
