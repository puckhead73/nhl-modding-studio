"""Stage wiring for the CC5 -> NHL atlas-A donor (NHL21 Ekblad accepted recipe) `donor_fit` route.

Each stage is a small function that reads what the previous stage wrote to disk (so a job can be inspected or
resumed by hand), calls the numeric modules, and returns outputs/summary/loss entries for its receipt. Route-specific
policies (e.g. the NHL19 upper-lining retraction) are explicit policy blocks, never global offsets; a job that names an
unsupported policy is refused before any conversion runs.
"""
import json
from pathlib import Path
import numpy as np
from . import exchange as X, cc5_source as C, uv_fit as F, masks as M, classify as CL, mouth_islands as MI, collision_repair as CR, eye_place as EP, measure as ME, candidate as CA, geometry as G, preview as PV
from .neck_transition import neck_transition
from .job import JobError, verify_ref

HERE = Path(__file__).resolve().parents[2]
BLENDER_EXTRACT = HERE / 'experiments/claude/worker/blender_extract_cc5.py'; BLENDER_BAKE = HERE / 'experiments/claude/worker/blender_bake.py'
RENDER_VIEWS = HERE / 'experiments/claude/render/render_fixture_views.py'; RENDER_TEXTURED = HERE / 'experiments/claude/render/render_textured.py'
SUPPORTED_ROUTE_POLICIES = {'none', 'nhl19_upper_lining_retraction'}   # nhl19 policy is declared but only the NHL19 driver implements it (not this slice)
NECK_DEFAULTS = dict(keep_rings=2, band_top=1.775, fit_top=1.79, max_delta=0.03, shape_rings=10, outlier_mm=8.0, fold_relax_iters=3, seam_consistent_sets=True, interior_follows_fit=True, ear_blend=True, lip_rings=2, snap_mm=None, relax_rings=2, ray_from='fit', lip_pin_mm=0.5, lip_pin_rings=None, relax_keep_best=True, interior_mode='fit', lining_blend_rings=3)


def _w(path, obj):
    Path(path).write_text(json.dumps(obj)); return path


def _wi(path, obj):
    Path(path).write_text(json.dumps(obj, indent=1)); return path


def stage_validate(job, R):
    def fn(d, log):
        ins = {}
        for grp in ('source', 'destination', 'calibration'):
            for k, v in job[grp].items():
                if isinstance(v, dict) and 'path' in v: ins[f'{grp}.{k}'] = verify_ref(f'{grp}.{k}', v)
        tools = __import__('src.conversion.job', fromlist=['tool_identity']).tool_identity(job)
        sha = __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file
        tools['worker_code'] = {str(p.relative_to(HERE)).replace('\\', '/'): sha(p) for p in sorted(list((HERE / 'src/conversion').glob('*.py')) + list((HERE / 'experiments/claude/worker').glob('*.py')) + [RENDER_VIEWS, RENDER_TEXTURED])}
        ex = X.load(job['destination']['exchange']['path']); sig = X.signature(ex)
        exp = job['destination'].get('expected_donor_layout') or {}
        ok, problems = X.validate_donor_layout(ex, exp)
        if job['destination'].get('profile_id') and ex.get('profile_id') != job['destination']['profile_id']: problems.append(f"exchange profile_id {ex.get('profile_id')} != job {job['destination']['profile_id']}")
        prof_path = job['destination'].get('profile', {}).get('path')
        if prof_path:
            prof = json.loads(Path(prof_path).read_text())
            if prof.get('donor', {}).get('sha256') and prof['donor']['sha256'] != ex.get('donor_sha256'): problems.append('profile donor sha256 != exchange donor_sha256')
            for sm in prof.get('layout', {}).get('submeshes', []):
                s = sig['submeshes'].get(sm['name'])
                if s is None: problems.append(f"profile submesh {sm['name']} absent from exchange")
                elif s['triangles'] != sm['triangles']: problems.append(f"profile {sm['name']} triangles {sm['triangles']} != exchange {s['triangles']}")
        rp = job['policies'].get('route_specific', {}).get('id', 'none')
        if rp not in SUPPORTED_ROUTE_POLICIES: problems.append(f'route_specific policy {rp} unsupported')
        if rp == 'nhl19_upper_lining_retraction': problems.append('route_specific policy nhl19_upper_lining_retraction is declared for the NHL19 route only; this NHL21 slice refuses it')
        if job['policies'].get('atlas_policy', M.ATLAS_POLICY_NHL_HEAD_A['id']) != M.ATLAS_POLICY_NHL_HEAD_A['id']: problems.append(f"atlas_policy {job['policies'].get('atlas_policy')} unsupported (only {M.ATLAS_POLICY_NHL_HEAD_A['id']})")
        if problems: raise JobError('input validation failed: ' + '; '.join(problems))
        _wi(d / 'donor_signature.json', sig); _wi(d / 'tools.json', tools); _wi(d / 'inputs.json', ins)
        return {'outputs': {'donor_signature': d / 'donor_signature.json', 'tools': d / 'tools.json'}, 'summary': {'inputs_verified': len(ins), 'donor_layout_ok': ok, 'tools': tools, 'exchange_profile_id': ex.get('profile_id')}}
    return fn


def stage_extract(job, R):
    def fn(d, log):
        src = job['source']; sg = src.get('source_geometry')
        if sg and 'path' in sg:
            info = verify_ref('source.source_geometry', sg); s = C.load(info['path']); ok, problems = C.validate(s, expected_signature=src.get('expected_signature'))
            if not ok: raise JobError('supplied source_geometry invalid: ' + '; '.join(problems))
            Path(d / 'source_geometry.json').write_bytes(Path(info['path']).read_bytes())
            _wi(d / 'extract_receipt.json', {'mode': 'supplied_and_verified', 'source': info, 'signature': C.signature(s)})
            return {'outputs': {'source_geometry': d / 'source_geometry.json'}, 'summary': {'mode': 'supplied_and_verified', 'signature': C.signature(s)}, 'losses': [{'stage': 'extract_source', 'component': 'source', 'field': 'rig', 'action': 'drop', 'count': 1, 'units': 'binding', 'evidence': 'measured', 'severity': 'none', 'note': 'head-only export carries a rigid one-bone binding only'}]}
        R.blender(BLENDER_EXTRACT, [src['fbx']['path'], d / 'extracted'], d)
        s = C.load(d / 'extracted' / 'source_geometry.json'); ok, problems = C.validate(s, expected_signature=src.get('expected_signature'))
        if not ok: raise JobError('extracted source_geometry invalid: ' + '; '.join(problems))
        Path(d / 'source_geometry.json').write_bytes(Path(d / 'extracted' / 'source_geometry.json').read_bytes())
        rec = json.loads((d / 'extracted' / 'extract_receipt.json').read_text()); rec['signature'] = C.signature(s); _wi(d / 'extract_receipt.json', rec)
        return {'outputs': {'source_geometry': d / 'source_geometry.json'}, 'summary': {'mode': 'blender_extract', 'signature': C.signature(s), 'blender': rec['blender']}, 'losses': [{'stage': 'extract_source', 'component': 'source', 'field': 'rig', 'action': 'drop', 'count': 1, 'units': 'binding', 'evidence': 'measured', 'severity': 'none', 'note': 'head-only export carries a rigid one-bone binding only'}]}
    return fn


def stage_fit(job, R, prev):
    def fn(d, log):
        ex = X.load(job['destination']['exchange']['path']); src = C.load(prev['source_geometry'])
        led = json.loads(Path(job['calibration']['tps_ledger']['path']).read_text())
        P, T, U = X.head_arrays(ex, 'original_positions'); comps = [c['vertex_ids'] for c in X.head(ex)['connected_components']]
        S0, ST, SU = C.head_arrays(src); SE = np.array(C.mesh(src, C.EYE)['positions']); DE = G.native_to_blender(X.submesh(ex, X.EYES)['original_positions'])
        params = {**F.DEFAULTS, **job['policies'].get('fit', {})}
        fit, mode, ledger, report, mk, prov = F.fit_head(P, T, U, comps, S0, ST, SU, SE, DE, led['pairs_pixels'], led['coefficients'], **params)
        ledger['tps_provenance'] = {'from': job['calibration']['tps_ledger']['path'], 'sha256': __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file(job['calibration']['tps_ledger']['path']), 'justification': job['calibration'].get('justification')}
        _w(d / 'native_mesh.json', X.with_positions(ex, X.HEAD, fit)); _wi(d / 'ledger.json', ledger); _w(d / 'masks.json', prov); _wi(d / 'fit_report.json', report); np.save(d / 'mode.npy', mode)
        losses = [{'stage': 'register_fit', 'component': 'head', 'field': 'correspondence', 'action': 'infer', 'count': int(report['modes'][k]), 'units': 'records', 'evidence': 'measured', 'severity': 'documented', 'note': f'mode {k}'} for k in ('uncovered', 'radial', 'blend', 'band', 'oral', 'lash')]
        losses.append({'stage': 'register_fit', 'component': 'head', 'field': 'confidence', 'action': 'preserve_opaque', 'count': None, 'units': None, 'evidence': 'unknown', 'severity': 'documented', 'note': 'per-record correspondence confidence is null (no estimator); method labels only'})
        return {'outputs': {'native_mesh': d / 'native_mesh.json', 'ledger': d / 'ledger.json', 'masks': d / 'masks.json', 'fit_report': d / 'fit_report.json', 'mode': d / 'mode.npy'}, 'summary': {k: report[k] for k in ('modes', 'shift_m', 'covered_fit_to_cc5_mm', 'max_move_vs_donor_mm', 'triangle_quality_vs_donor', 'seconds')}, 'losses': losses}
    return fn


def stage_neck(job, R, prev):
    def fn(d, log):
        nat = X.load(prev['native_mesh']); src = C.load(prev['source_geometry']); ledger = json.loads(Path(prev['ledger']).read_text())
        orig, T, U = X.head_arrays(nat, 'original_positions'); fit, _, _ = X.head_arrays(nat, 'positions')
        S0, ST, _ = C.head_arrays(src); srcp = S0 + np.array(ledger['source_shift'])
        mk = M.derive(orig, T, U, [c['vertex_ids'] for c in X.head(nat)['connected_components']])
        params = {**NECK_DEFAULTS, **job['policies'].get('neck', {})}
        result, mode, report = neck_transition(orig, fit, T, srcp, ST, oral_ids=ledger['oral_ids'], ear_ids=mk['ear_ids'], **params)
        report = {'params': params, **report, 'max_change_vs_fit_mm': float(np.linalg.norm(result - fit, axis=1).max() * 1000)}
        _w(d / 'native_mesh.json', X.with_positions(nat, X.HEAD, result)); _wi(d / 'neck_report.json', report); np.save(d / 'mode.npy', mode)
        return {'outputs': {'native_mesh': d / 'native_mesh.json', 'neck_report': d / 'neck_report.json', 'mode': d / 'mode.npy'}, 'summary': {k: report.get(k) for k in ('frozen_vertices', 'band_candidates', 'confident_spatial', 'lip_pinned_vertices', 'demoted_by_fold_relaxation', 'new_collapsed_triangles', 'orientation_flips', 'max_change_vs_fit_mm', 'frozen_max_error', 'seam_max_gap')},
                'losses': [{'stage': 'neck_transition', 'component': 'head', 'field': 'positions', 'action': 'infer', 'count': int(report['band_candidates']), 'units': 'records', 'evidence': 'measured', 'severity': 'documented', 'note': 'neck band rebuilt from source surface rays + harmonic fill; attachment loop + 2 rings exact'}]}
    return fn


def _classify(nat_fit_path, nat_after_path, ledger, label):
    fitj = X.load(nat_fit_path); aftj = X.load(nat_after_path)
    donor, T, U = X.head_arrays(fitj, 'original_positions'); fit, _, _ = X.head_arrays(fitj, 'positions'); after, _, _ = X.head_arrays(aftj, 'positions')
    mk = M.derive(donor, T, U, [c['vertex_ids'] for c in X.head(fitj)['connected_components']])
    return CL.classify(donor, fit, after, T, U, ledger['oral_ids'], mk['lash'], mk['ear_core']), mk, donor, T


def stage_classify(job, R, prev, name='classify_after_neck'):
    def fn(d, log):
        ledger = json.loads(Path(prev['ledger']).read_text())
        res, mk, donor, T = _classify(prev['fit_native_mesh'], prev['native_mesh'], ledger, name)
        isl = MI.islands(donor, T, ledger['oral_ids'], mk['outer_faces'])
        _w(d / 'flip_classes.json', {'summary': res['summary'], 'records': res['records']}); _w(d / 'flagged_faces.json', res['lists']); _w(d / 'mouth_islands.json', isl)
        trig = CL.collision_triggers(res['records']); _wi(d / 'collision_triggers.json', {'rule': 'fit-introduced skin faces that self-intersect after the neck stage (classification-derived, no vertex ids)', 'faces': trig})
        s = res['summary']
        return {'outputs': {'flip_classes': d / 'flip_classes.json', 'flagged_faces': d / 'flagged_faces.json', 'mouth_islands': d / 'mouth_islands.json', 'collision_triggers': d / 'collision_triggers.json'},
                'summary': {'classes': s['classes'], 'introduced_by_shell': s['introduced_by_shell'], 'self_intersections_by_class': s['self_intersections_by_class'], 'new_pairs_vs_fit': s['new_pairs_vs_fit'], 'after_skin_defect_faces': s['after_skin_defects']['faces'], 'collision_triggers': trig, 'mouth_islands': {k: len(v) for k, v in isl.items()}, 'coverage_limits': s['coverage_limits']}}
    return fn


def stage_repair(job, R, prev):
    def fn(d, log):
        aftj = X.load(prev['native_mesh']); ledger = json.loads(Path(prev['ledger']).read_text()); src = C.load(prev['source_geometry'])
        donor, T, U = X.head_arrays(aftj, 'original_positions'); P, _, _ = X.head_arrays(aftj, 'positions')
        isl = json.loads(Path(prev['mouth_islands']).read_text()); trig = json.loads(Path(prev['collision_triggers']).read_text())['faces']
        S0, ST, _ = C.head_arrays(src); soup = G.TriangleSoup(S0 + np.array(ledger['source_shift']), ST)
        params = {**CR.DEFAULTS, **job['policies'].get('collision_repair', {})}
        if params.get('mode', 'fixed') == 'escalate':
            res, log_ = CR.smooth_repair_escalating(donor, P, T, ledger['oral_ids'], MI.lining_faces(isl), trig, soup, rings_schedule=params['rings_schedule'], passes=params['passes'], alpha=params['alpha'])
        elif params.get('mode', 'fixed') == 'fixed':
            res, log_ = CR.smooth_repair(donor, P, T, ledger['oral_ids'], MI.lining_faces(isl), trig, soup, rings=params['rings'], passes=params['passes'], alpha=params['alpha'], force=bool(params.get('force', False)))
        else: raise JobError(f"collision_repair mode {params.get('mode')} unsupported (fixed | escalate)")
        _w(d / 'native_mesh.json', X.with_positions(aftj, X.HEAD, res)); _wi(d / 'repair_report.json', log_)
        losses = [] if log_['changed_count'] == 0 else [{'stage': 'collision_repair', 'component': 'head', 'field': 'positions', 'action': 'infer', 'count': log_['changed_count'], 'units': 'records', 'evidence': 'measured', 'severity': 'documented', 'note': f"local Laplacian smoothing, max {log_['max_move_mm']} mm, no source projection"}]
        return {'outputs': {'native_mesh': d / 'native_mesh.json', 'repair_report': d / 'repair_report.json'}, 'summary': {k: log_.get(k) for k in ('result', 'triggered_faces', 'changed_count', 'max_move_mm', 'gate_accepted', 'gate', 'source_error_skin_records_mm')}, 'losses': losses}
    return fn


def stage_eyes(job, R, prev):
    def fn(d, log):
        nat = X.load(prev['native_mesh']); src = C.load(prev['source_geometry']); ledger = json.loads(Path(prev['ledger']).read_text())
        pol = job['policies'].get('eyes', {'operation': 'component_similarity_to_cc5_sphere'})
        e = X.submesh(nat, X.EYES); Pe = G.native_to_blender(e['original_positions']); comps = [c['vertex_ids'] for c in e['connected_components']]
        Q = np.array(C.mesh(src, C.EYE)['positions']) + np.array(ledger['source_shift']); mats = C.eye_material_ids(src)
        diag = ME.eye_diagnostic(Pe, comps, Q, mats)
        if pol.get('operation') == 'native_unchanged':
            Pn = Pe.copy(); rep = {'method': 'eyes kept native (policy)', 'components': []}
        elif pol.get('operation') == 'component_similarity_to_cc5_sphere':
            Pn, rep = EP.place_eyes(Pe, comps, Q, mats)
        else: raise JobError(f"eye policy {pol.get('operation')} unsupported")
        ok, ver = EP.verify_similarity(Pe, Pn, comps)
        if not ok: raise JobError('eye placement is not a per-component positive uniform scale + translation')
        rep['verification'] = ver; rep['diagnostic'] = diag; rep['pivot_provenance'] = 'CC5 sphere centre; native rig eye pivot unknown (opaque rig block)'
        _w(d / 'native_mesh.json', X.with_positions(nat, X.EYES, Pn)); _wi(d / 'eyes_report.json', rep)
        return {'outputs': {'native_mesh': d / 'native_mesh.json', 'eyes_report': d / 'eyes_report.json'}, 'summary': {'operation': pol.get('operation'), 'components': [{k: c[k] for k in ('side', 'uniform_scale', 'translation_mm')} for c in rep['components']], 'diagnostic_offsets_mm': {s: v['centre_offset_norm_mm'] for s, v in diag.items()}},
                'losses': [{'stage': 'eye_place', 'component': 'eyes', 'field': 'pivot', 'action': 'preserve_opaque', 'count': len(comps), 'units': 'components', 'evidence': 'unknown', 'severity': 'documented', 'note': 'native eye pivots/animation opaque; placement uses CC5 sphere centres'}]}
    return fn


def stage_bake(job, R, prev):
    def fn(d, log):
        pol = job['policies'].get('bake', {}); cfg = {'fitted_native_mesh': str(prev['native_mesh']), 'source_geometry': str(prev['source_geometry']), 'ledger': str(prev['ledger']), 'cc5_diffuse': job['source']['diffuse']['path'], 'native_diffuse': job['destination']['native_diffuse_png']['path'], 'out_dir': str(d / 'bake'), 'size': int(pol.get('size', 1024))}
        _wi(d / 'bake_stage.json', cfg); R.blender(BLENDER_BAKE, [d / 'bake_stage.json'], d)
        rep = json.loads((d / 'bake' / 'bake_report.json').read_text())
        losses = [{'stage': 'bake', 'component': 'atlas', 'field': 'texels', 'action': 'infer', 'count': rep['F_texels_native_fallback'], 'units': 'texels', 'evidence': 'measured', 'severity': 'documented', 'note': 'native fallback where CC5 has no surface (chest/nape)'},
                  {'stage': 'bake', 'component': 'atlas', 'field': 'texels', 'action': 'infer', 'count': rep['A_hole_texels'], 'units': 'texels', 'evidence': 'measured', 'severity': 'documented', 'note': 'TPS holes replaced by surface correspondence'},
                  {'stage': 'bake', 'component': 'atlas', 'field': 'resolution', 'action': 'quantize', 'count': rep['size'], 'units': 'px', 'evidence': 'measured', 'severity': 'documented', 'note': 'CC5 4096 diffuse resampled to the native 1024 atlas; BC1 + mips are the packer\'s (destination policy), not established here'},
                  {'stage': 'bake', 'component': 'atlas', 'field': 'normal_map', 'action': 'drop', 'count': 1, 'units': 'map', 'evidence': 'measured', 'severity': 'documented', 'note': 'no normal-map bake; native normal texture untouched'}]
        return {'outputs': {'atlas': d / 'bake' / 'bake_G_boundary.png', 'bake_report': d / 'bake' / 'bake_report.json'}, 'summary': {k: rep[k] for k in ('used_texels', 'A_hole_texels', 'B_nearest_distance_mm', 'F_texels_native_fallback', 'F_feathered', 'region_coverage', 'candidate_atlas_sha256', 'total_seconds')}, 'losses': losses}
    return fn


def stage_measure(job, R, prev):
    def fn(d, log):
        ex = X.load(job['destination']['exchange']['path']); nat = X.load(prev['native_mesh']); src = C.load(prev['source_geometry']); ledger = json.loads(Path(prev['ledger']).read_text())
        donor, T, U = X.head_arrays(nat, 'original_positions'); after, _, _ = X.head_arrays(nat, 'positions')
        mk = M.derive(donor, T, U, [c['vertex_ids'] for c in X.head(nat)['connected_components']]); S0, ST, _ = C.head_arrays(src); srcp = S0 + np.array(ledger['source_shift'])
        e = X.submesh(nat, X.EYES); Pe = G.native_to_blender(e['original_positions']); Pn = G.native_to_blender(e['positions']); comps = [c['vertex_ids'] for c in e['connected_components']]
        out = {'jaw_region_distance': ME.jaw_region_distance(donor, after, T, U, srcp, ST, mk['ear_core']), 'skin_source_distance': ME.source_distance(after, mk['on_skin'] & ~mk['is_frozen'] & ~mk['lash'], srcp, ST),
               'affected_region': ME.affected_region(donor, after, mk['dup']), 'lid_screen': ME.lid_screen(donor, after, T, U, Pe, Pn, comps, mk['lash']), 'frame': 'Blender metres, Z up, -Y forward, +X = character left; rest pose only'}
        _wi(d / 'measurements.json', out)
        return {'outputs': {'measurements': d / 'measurements.json'}, 'summary': {'jaw_median_mm_L_R': [out['jaw_region_distance']['character_left']['fit_to_source_mm']['median'], out['jaw_region_distance']['character_right']['fit_to_source_mm']['median']], 'skin_source_distance': out['skin_source_distance'], 'affected_region': {k: out['affected_region'][k] for k in ('records_moved', 'max_move_mm', 'seam_max_gap_m')}, 'lid_screen_fitted_vs_placed_median_mm': {s: v['fitted_head_vs_placed_eye']['median_mm'] for s, v in out['lid_screen']['per_side'].items()}}}
    return fn


def stage_render(job, R, prev):
    def fn(d, log):
        R.blender(RENDER_VIEWS, [prev['native_mesh'], prev['source_geometry'], prev['ledger'], d / 'views', 'candidate'], d)
        R.blender(RENDER_TEXTURED, [prev['native_mesh'], prev['atlas'], d / 'textured', 'candidate'], d)
        views = sorted(p.name for p in (d / 'views').glob('*.png')); tex = sorted(p.name for p in (d / 'textured').glob('*.png'))
        return {'outputs': {'views_manifest': d / 'views' / 'render_manifest.json'}, 'summary': {'views': views, 'textured': tex, 'note': 'Blender Workbench clay/textured views; not the game shader; native eyes hidden hair'}}
    return fn


def stage_freeze(job, R, prev, stage_receipts):
    def fn(d, log):
        ex = X.load(job['destination']['exchange']['path']); cand = X.load(prev['native_mesh'])
        checks, problems, per = CA.validate_candidate(cand, ex, eyes_may_move=job['policies'].get('eyes', {}).get('operation', 'component_similarity_to_cc5_sphere') != 'native_unchanged')
        atlas = CA.atlas_check(prev['atlas']); checks['atlas_is_1024_square'] = {'ok': atlas['ok'], 'detail': atlas}
        if atlas['ok'] is False: problems.append('atlas_is_1024_square')
        # references (optional): compare against accepted artifacts when the job names them (parity evidence, not acceptance)
        parity = {}
        for k, ref in (job.get('references') or {}).items():
            p = Path(ref['path'])
            if not p.exists(): parity[k] = {'missing': True}; continue
            if k.endswith('native_mesh'):
                r = X.load(p); accp, _, _ = X.head_arrays(r, 'positions'); cp, _, _ = X.head_arrays(cand, 'positions'); dh = np.linalg.norm(accp - cp, axis=1)
                ae = G.native_to_blender(X.submesh(r, X.EYES)['positions']); ce = G.native_to_blender(X.submesh(cand, X.EYES)['positions']); de = np.linalg.norm(ae - ce, axis=1)
                acc32 = accp.astype(np.float32); c32 = cp.astype(np.float32)
                parity[k] = {'head_max_abs_diff_m': float(dh.max()), 'head_records_differing': int((dh > 0).sum()), 'head_float32_equal': bool(np.array_equal(acc32, c32)), 'eyes_max_abs_diff_m': float(de.max()), 'eyes_records_differing': int((de > 0).sum()), 'reference_sha256': __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file(p)}
            elif k.endswith('atlas'):
                parity[k] = {'reference_sha256': __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file(p), 'candidate_sha256': __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file(prev['atlas'])}
                parity[k]['byte_identical'] = parity[k]['reference_sha256'] == parity[k]['candidate_sha256']
                try:
                    from PIL import Image
                    a = np.asarray(Image.open(p).convert('RGB'), int); b = np.asarray(Image.open(prev['atlas']).convert('RGB'), int); diff = np.abs(a - b).max(2)
                    parity[k].update({'pixels_differing': int((diff > 0).sum()), 'max_channel_diff': int(diff.max())})
                except ImportError: parity[k]['pixel_compare'] = 'PIL unavailable'
        import shutil
        shutil.copyfile(prev['native_mesh'], d / 'native_mesh.json'); shutil.copyfile(prev['atlas'], d / 'atlas_1024.png')
        sha = __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file
        artifacts = {'native_mesh_json': str(d / 'native_mesh.json'), 'native_mesh_sha256': sha(d / 'native_mesh.json'), 'atlas_png': str(d / 'atlas_1024.png'), 'atlas_sha256': sha(d / 'atlas_1024.png')}
        inputs = {}
        for grp in ('source', 'destination', 'calibration'):
            for k, v in job[grp].items():
                if isinstance(v, dict) and 'path' in v: inputs[f'{grp}_{k}'] = {'path': v['path'], 'sha256': sha(v['path'])}
        inputs['worker_source_geometry'] = {'path': str(prev['source_geometry']), 'sha256': sha(prev['source_geometry'])}
        losses = [l for r in stage_receipts for l in (r.get('losses') or [])]
        losses.append({'stage': 'freeze', 'component': 'head', 'field': 'normals/tangents', 'action': 'infer', 'count': per[X.HEAD]['vertices'], 'units': 'records', 'evidence': 'documented-prior', 'severity': 'documented', 'note': 'head shading regenerated by the native packer (Codex); native normals/tangents in the exchange are reference data'})
        losses.append({'stage': 'freeze', 'component': 'cards/haircap', 'field': 'all', 'action': 'preserve_opaque', 'count': 2, 'units': 'submeshes', 'evidence': 'measured', 'severity': 'none', 'note': 'untouched (validated byte-equal)'})
        rep = {r['name']: r.get('summary') for r in stage_receipts}
        cls = rep.get('classify_after_repair') or rep.get('classify_after_neck') or {}
        # acceptance policy (phase 7/9 decision rule): fit-introduced skin faces that still intersect (collision triggers)
        # or any new skin/skin intersecting pair after the conversion block experimental packing; inherited flags stay explicit
        if cls.get('collision_triggers'): problems.append(f"collision_triggers_remaining: {cls['collision_triggers']} (fit-introduced skin faces intersecting skin/lining; repair gate: {(rep.get('collision_repair') or {}).get('result')})")
        if (cls.get('new_pairs_vs_fit') or {}).get('skin_skin'): problems.append(f"new_skin_skin_pairs_vs_fit: {cls['new_pairs_vs_fit']['skin_skin_faces']}")
        if not cls: problems.append('no classification stage summary available')
        status = 'ready_for_experimental_packing' if not problems else 'blocked'
        validation = {'fixture': job['job_id'], 'checks': checks, 'problems': problems, 'per_submesh': per, 'parity_vs_references': parity, 'inputs': inputs, 'outputs': artifacts}
        _wi(d / 'validation.json', validation)
        gates = {'G0': 'pass (inputs hashed and layout-validated by the worker)', 'G1': 'Codex domain (native round-trip not run here)',
                 'G2': f"partial static: classification {cls.get('classes')} introduced_by_shell {cls.get('introduced_by_shell')} new skin/skin pairs {cls.get('new_pairs_vs_fit', {}).get('skin_skin')}; measurements in stage measure; coplanar/adjacent contacts unknown; renders are Blender Workbench",
                 'G3': 'open (no pose tests; opaque rig; eye pivots unknown)', 'G4': 'Codex domain', 'G5': 'open (no game test; a new run never inherits prior acceptance)'}
        hand = CA.handoff(job['job_id'], job['job_id'], job['source'].get('label'), job['destination'].get('label'), status, problems, artifacts, inputs, str(d / 'validation.json'),
                          {'stage_receipts': [str(Path(r['dir']) / 'receipt.json') for r in stage_receipts], 'renders': rep.get('render')}, cls, {'frozen_candidate': per.get(X.EYES, {}).get('components'), 'diagnostic': (rep.get('eye_place') or {}).get('diagnostic_offsets_mm')},
                          {'atlas_policy': job['policies'].get('atlas_policy', M.ATLAS_POLICY_NHL_HEAD_A['id']), 'masks_json': str(prev['masks'])}, 'Worker-produced conversion candidate (fitted head, placed eyes, baked 1024 atlas) in the Codex exchange schema; NOT a native package; nothing game-verified.',
                          ['Pack head + eye positions; regenerate head shading; encode atlas to BC1 + 11 mips; reversible test packet; user game test.'],
                          ['Residual classification flags explicit (see stage receipts)', 'coplanar/adjacent contact coverage unknown', 'eye pivots unknown', 'no pose tests', 'atlas not compressed here'], gates,
                          loss_report={'schema': 'nhl-head-loss/1', 'entries': losses}, provenance={'job': str(R.out / 'job.json'), 'tools': rep.get('validate', {}).get('tools')})
        _wi(d / 'handoff.json', hand); _wi(R.out / 'handoff.json', hand); _wi(R.out / 'loss_report.json', hand['loss_report'])
        return {'outputs': {'native_mesh': d / 'native_mesh.json', 'atlas': d / 'atlas_1024.png', 'validation': d / 'validation.json', 'handoff': d / 'handoff.json'}, 'summary': {'status': status, 'problems': problems, 'checks_ok': sum(1 for v in checks.values() if v['ok']), 'checks_total': len(checks), 'parity_vs_references': parity}, 'handoff': hand}
    return fn


def stage_preview(job, R, prev, handoff):
    """Application preview: self-contained GLB (candidate exchange geometry + embedded baked atlas) next to the
    handoff. Diagnostic-only when the candidate is blocked; never changes readiness. R.meta['preview'] feeds the
    ui_manifest written by Runner.finish()."""
    def fn(d, log):
        R.meta['preview'] = {'status': 'failed', 'path': str(R.out / 'preview.glb'), 'error': 'preview stage did not complete'}
        cand = X.load(handoff['artifacts']['native_mesh_json']); atlas = handoff['artifacts']['atlas_png']
        info = PV.write_glb(cand, atlas, d / 'preview.glb', job['job_id'], handoff['status'], extras={'native_mesh_sha256': handoff['artifacts']['native_mesh_sha256'], 'atlas_sha256': handoff['artifacts']['atlas_sha256']})
        ok, problems, _ = PV.verify_glb(d / 'preview.glb', cand, atlas)
        if not ok: raise RuntimeError('preview verification failed: ' + '; '.join(problems))
        import shutil; shutil.copyfile(d / 'preview.glb', R.out / 'preview.glb')
        sha = __import__('src.conversion.job', fromlist=['sha256_file']).sha256_file
        info['sha256'] = sha(R.out / 'preview.glb'); info['verified_against'] = {'native_mesh_sha256': handoff['artifacts']['native_mesh_sha256'], 'atlas_sha256': handoff['artifacts']['atlas_sha256']}
        _wi(d / 'preview_info.json', info)
        R.meta['preview'] = {'status': 'ok', 'path': str(R.out / 'preview.glb'), 'info': {k: info[k] for k in ('schema', 'bytes', 'sha256', 'nodes', 'approximations', 'frame', 'uv', 'diagnostic_only', 'verified_against')}}
        return {'outputs': {'preview_glb': R.out / 'preview.glb', 'preview_info': d / 'preview_info.json'}, 'summary': {'bytes': info['bytes'], 'nodes': [(n['name'], n['role'], n['default_visible']) for n in info['nodes']], 'diagnostic_only': info['diagnostic_only'], 'approximations': info['approximations']},
                'losses': [{'stage': 'preview', 'component': 'preview', 'field': 'materials', 'action': 'infer', 'count': len(info['nodes']), 'units': 'nodes', 'evidence': 'documented', 'severity': 'preview_only', 'note': 'preview materials/normals are approximations; not the game shader; the packed native output is unaffected'}]}
    return fn
