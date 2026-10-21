"""Ensure eye shading preservation cannot accept arbitrary eye deformations."""
import importlib.util
from pathlib import Path
import unittest
import numpy as np

path=Path(__file__).resolve().parents[2]/'experiments/codex/nhl19_package/shading_proof.py'
spec=importlib.util.spec_from_file_location('shading_proof',path)
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
proof=module.positive_uniform_scale_translation


class EyeShadingProofTests(unittest.TestCase):
    def setUp(self):
        rng=np.random.default_rng(19)
        self.a=(rng.normal(size=(66,3))*.015+[.03,1.84,.1]).astype(np.float32)

    def test_positive_uniform_scale_translation_preserves_directions(self):
        result=proof(self.a,(self.a.astype(float)*.953+[.001,-.027,.005]).astype(np.float32))
        self.assertTrue(result['accepted'])
        self.assertLessEqual(result['max_absolute_residual'],result['float32_tolerance'])

    def test_nonuniform_scale_is_not_shading_invariant(self):
        self.assertFalse(proof(self.a,self.a*[.95,1.02,.95])['accepted'])

    def test_single_vertex_deformation_is_not_shading_invariant(self):
        b=self.a.copy();b[0,0]+=.0001
        self.assertFalse(proof(self.a,b)['accepted'])

    def test_reflection_is_not_positive_uniform_scale(self):
        self.assertFalse(proof(self.a,-self.a)['accepted'])

    def test_rotation_requires_rotating_shading(self):
        theta=.02;c,s=np.cos(theta),np.sin(theta)
        rotation=np.array([[c,0,s],[0,1,0],[-s,0,c]])
        self.assertFalse(proof(self.a,self.a@rotation)['accepted'])


if __name__=='__main__':unittest.main()
