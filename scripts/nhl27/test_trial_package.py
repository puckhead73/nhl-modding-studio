"""Stored CAS framing boundaries, independent of proprietary assets."""
import unittest
from compatibility import frames
from deke_trial_package import stored_frames


class PackageTests(unittest.TestCase):
    def test_stored_block_boundaries(self):
        for n in (1,65535,65536,65537,131072):
            data=(bytes(range(256))*((n+255)//256))[:n]
            blocks,decoded=frames(stored_frames(data))
            self.assertEqual(decoded,data)
            self.assertEqual(len(blocks),(n+65535)//65536)
            self.assertTrue(all(x['codec']==0 for x in blocks))


if __name__=='__main__':unittest.main()
