"""Bounded proof that positive uniform scale + translation preserves shading directions."""
import numpy as np


def positive_uniform_scale_translation(original, fitted):
    a=np.asarray(original,dtype=np.float32).astype(float)
    b=np.asarray(fitted,dtype=np.float32).astype(float)
    if a.shape!=b.shape or a.ndim!=2 or a.shape[1]!=3 or not np.isfinite(a).all() or not np.isfinite(b).all():
        return {'accepted':False,'reason':'invalid shape or nonfinite positions'}
    ac=a-a.mean(axis=0);bc=b-b.mean(axis=0)
    den=float((ac*ac).sum())
    if den<=1e-20:return {'accepted':False,'reason':'insufficient position variation'}
    scale=float((ac*bc).sum()/den)
    translation=b.mean(axis=0)-scale*a.mean(axis=0)
    predicted=scale*a+translation
    # Two float32 ULPs at this component's largest absolute coordinate bound
    # affine-fit/position-rounding residuals. This is a numerical acceptance
    # bound, not permission for visible nonuniform deformation.
    magnitude=np.float32(max(float(np.abs(b).max()),float(np.abs(predicted).max()),1e-30))
    tolerance=2*float(np.spacing(magnitude))
    residual=float(np.abs(predicted-b).max())
    return {'accepted':bool(scale>0 and residual<=tolerance),'scale':scale,
            'translation':translation.tolist(),'max_absolute_residual':residual,
            'float32_tolerance':tolerance,'tolerance_rule':'two float32 ULPs at maximum component coordinate',
            'max_absolute_residual_mm':residual*1000,'tolerance_mm':tolerance*1000}
