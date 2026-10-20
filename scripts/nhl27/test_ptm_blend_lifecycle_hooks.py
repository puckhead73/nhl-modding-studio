"""Verify allocation coverage and both distinct destructor paths in the target."""
from pathlib import Path
import struct
import unittest
from deke_runtime_evidence import checked_image,TARGET_HASH
from research import SelfImage
from trace import direct_call_candidates
import ptm_blend_lifecycle_hooks as hook


class BlendLifecycleEvidenceTests(unittest.TestCase):
    def test_all_constructor_paths_and_destructors(self):
        path=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not path.exists():self.skipTest('Private target required')
        image=checked_image(SelfImage,path,TARGET_HASH)
        callers=[]
        for seg in image.segments:
            data=image.read(seg['va'],seg['filesz'])
            if seg['flags']&1:
                callers.extend(direct_call_candidates(data,seg['va'],0x36bf60))
            else:
                self.assertNotIn(struct.pack('<Q',0x36bf60),data)
        self.assertEqual(sorted(callers),[0x36be8c,0x36bebc])
        self.assertEqual(struct.unpack('<QQ',image.read(0xa41bcf8,16)),(0x3693a0,0x369410))
        # The deleting destructor passes size zero, not the original allocation size.
        self.assertEqual(image.read(0x36946f,2),bytes.fromhex('31d2'))
        symbols={f'ptm_blend_lifecycle_{a:x}':0xc000000+i*0x1000 for i,(a,*_) in enumerate(hook.SITES)}
        patches=hook.patches(image,symbols)
        self.assertEqual(len(patches),5)
        self.assertEqual(patches[0],(0x36be40,bytes.fromhex('bed8020000'),bytes.fromhex('be30030000')))


if __name__=='__main__':unittest.main()
