"""Measured PS5 pointer/layout regressions; no game data is checked in."""
import struct
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ps5_head_export import ps5_mesh, associated_textures


def fixture(layout=0xbc, inline=True):
    data = bytearray(0x400)
    put = lambda at, fmt, *values: struct.pack_into("<" + fmt, data, at, *values)
    put(4, "II", layout, 0x170)
    put(0xac, "H", 1)
    put(0x30, "Q", 0xc0)
    lod, section = 0xd0, 0x190
    put(lod + 8, "I", 1)
    put(lod + 12, "Q", section - 16)
    put(lod + 0x54, "III", 0x21, 6, 60)
    is_nhl27 = layout in (0xb8, 0xac)
    guid_at = lod + (0x70 if is_nhl27 else 0x74)
    if not inline:
        data[guid_at:guid_at + 16] = bytes(range(16))
    put(section + 8, "Q", 0x310 - 16)
    put(section + 0x20, "IIII", 1, 0, 0, 3)
    declaration = section + (0x70 if is_nhl27 else 0x64)
    data[declaration:declaration + 8] = bytes([1, 3, 0, 0, 0x21, 2, 1, 0])
    data[declaration + 0x60] = 2
    data[0x310:0x319] = b"mat_head\0"
    geometry = struct.pack("<15f3H", 0, 0, 0, 1, 0, 0, 0, 1, 0,
                           0, 0, 1, 0, 0, 1, 0, 1, 2)
    metadata = struct.pack("<4I", len(data), 0, len(geometry), 0).hex()
    return bytes(data) + (geometry if inline else b""), metadata, geometry


class Ps5MeshTests(unittest.TestCase):
    def test_shared_maps_exclude_other_players_common_folders(self):
        family = "content/character/skater/shared/head/licensed/example"
        textures = {
            (family + "/textures/common/head_color", "a"): ("own.dds", {}),
            ("content/character/common/textures/eye_color", "b"): ("shared.dds", {}),
            ("content/character/skater/shared/head/licensed/other/textures/common/head_color", "c"): ("wrong.dds", {}),
        }
        self.assertEqual(associated_textures(family + "/components/head_mesh", textures),
                         ["own.dds", "shared.dds"])

    def test_inline_and_streamed_geometry_match_for_all_measured_layouts(self):
        for layout in (0xbc, 0xb8, 0xb0, 0xac):
            data, meta, geometry = fixture(layout)
            inline = ps5_mesh(data, {}, None, meta)
            streamed, _, _ = fixture(layout, inline=False)
            with patch("ps5_head_export.read", return_value=geometry):
                external = ps5_mesh(streamed, {bytes(range(16)).hex(): {}}, None)
            self.assertEqual(inline[0][2], external[0][2])
            self.assertEqual(inline[0][3], ["mat_head"])

    def test_inline_bounds_and_index_bounds_are_enforced(self):
        data, meta, _ = fixture()
        with self.assertRaisesRegex(ValueError, "bounds"):
            ps5_mesh(data[:-1], {}, None, meta)
        damaged = bytearray(data)
        struct.pack_into("<H", damaged, len(damaged) - 2, 3)
        with self.assertRaisesRegex(ValueError, "Index outside"):
            ps5_mesh(bytes(damaged), {}, None, meta)
        with self.assertRaisesRegex(ValueError, "metadata"):
            ps5_mesh(data, {}, None)


if __name__ == "__main__":
    unittest.main()
