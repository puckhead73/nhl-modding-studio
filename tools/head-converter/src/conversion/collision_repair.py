"""Collision-triggered coupled local Laplacian smoothing (numpy only).

Port of `experiments/claude/mouth/local_untangle_nhl21.py --mode smooth --seeds triggered` (the accepted Fedorov/Ekblad
correction; exact no-op on accepted Turgeon). Trigger faces come from the classification (`classify.collision_triggers`):
fit-introduced skin faces that intersect skin or lining after the neck stage. With no trigger the stage is a no-op.
Seeds = records of the trigger faces; neighbourhood = `rings` graph rings (seam groups as units, oral ids excluded);
`passes` Laplacian averaging passes of positions with weight `alpha`; float32 rounding of moved records only; NO
projection back to the source. Gate: no new skin/skin pair, no new skin/interior pair, no remaining skin/skin pair,
no collapsed face (intersection test limits: coplanar/adjacent contacts not detected). If the gate fails the head is
returned unchanged unless force=True; the receipt always says which.
"""
import numpy as np
from . import geometry as G, face_quality as FQ

DEFAULTS = {'mode': 'fixed', 'rings': 3, 'passes': 2, 'alpha': 0.5, 'rings_schedule': [1, 2, 3]}


def smooth_repair_escalating(donor, P, T, oral_ids, lining_faces, trigger_faces, source_soup=None, *, rings_schedule=(1, 2, 3), passes=2, alpha=0.5):
    """Deterministic ring escalation: run `smooth_repair` for each ring count in `rings_schedule` (ascending) and keep the
    FIRST result that passes the gate; if none passes the head is returned unchanged. Measured motivation: the accepted
    Fedorov correction needed 3 rings (1 and 2 rings left crossings, phase 9 ring selection) while Demitra's four chin
    crossings clear at 1 ring and re-pierce the lining at 2-4 rings (overnight-2026-09-12 sweep). Every attempt is logged."""
    attempts = []
    for r in rings_schedule:
        res, log = smooth_repair(donor, P, T, oral_ids, lining_faces, trigger_faces, source_soup, rings=int(r), passes=passes, alpha=alpha, force=False)
        attempts.append({k: log.get(k) for k in ('params', 'result', 'changed_count', 'max_move_mm', 'gate_accepted', 'gate')})
        if log['changed_count'] == 0 and log['result'].startswith('NO-OP'):
            log['mode'] = 'escalate'; log['attempts'] = attempts; return res, log
        if log['gate_accepted']:
            log['mode'] = 'escalate'; log['chosen_rings'] = int(r); log['attempts'] = attempts; return res, log
    log['mode'] = 'escalate'; log['chosen_rings'] = None; log['attempts'] = attempts; log['result'] = f'gate failed for every ring count in {list(rings_schedule)}: head returned unchanged'
    return P.copy(), log


def smooth_repair(donor, P, T, oral_ids, lining_faces, trigger_faces, source_soup=None, *, rings=3, passes=2, alpha=0.5, force=False):
    """donor: untouched donor head; P: head after the neck stage (Blender frame). Returns (positions, report)."""
    n = len(P); T = np.asarray(T); dup = G.seam_groups(donor); adj = G.adjacency(n, T, dup)
    oral = np.zeros(n, bool); oral[np.asarray(list(oral_ids), int)] = True
    outer = FQ.outer_shell_faces(donor, T); skinf = outer & ~oral[T].any(1)
    lining_faces = np.asarray(lining_faces, int); lining_v = np.zeros(n, bool)
    if len(lining_faces): lining_v[np.unique(T[lining_faces])] = True
    lining_v &= ~oral; skin_v = np.zeros(n, bool); skin_v[np.unique(T[skinf])] = True; lining_v &= ~skin_v
    trig = np.array(sorted(int(f) for f in trigger_faces), int)
    log = {'params': {'rings': rings, 'passes': passes, 'alpha': alpha, 'seeds': 'triggered', 'force': force}, 'triggered_faces': trig.tolist()}
    if len(trig) == 0:
        log.update({'result': 'NO-OP: no fit-introduced colliding skin faces', 'changed_ids': [], 'changed_count': 0, 'max_move_mm': 0.0, 'gate_accepted': True, 'gate': None, 'source_error_skin_records_mm': None,
                    'seam_max_gap_m': max((float(np.linalg.norm(P[ids] - P[ids[0]], axis=1).max()) for ids in dup), default=0.0), 'coverage_limits': 'trigger derivation uses exact non-coplanar triangle intersections on non-adjacent faces; coplanar/adjacent contacts unknown'})
        return P.copy(), log
    res = P.copy(); all_faces = np.arange(len(T))

    def pairs_of(positions, faces, targets):
        Tall = np.vstack([T[faces], T[targets]]); prs = FQ.self_intersections(positions, Tall, candidates=np.arange(len(faces)), merge_groups=dup)
        return [(faces[i], targets[j - len(faces)]) for i, j in prs if j >= len(faces)]

    def pairset(positions, faces):
        return set((min(x, y), max(x, y)) for x, y in pairs_of(positions, faces, all_faces) if x != y)

    seed_faces = [int(f) for f in trig]; seeds = sorted({int(v) for f in seed_faces for v in T[f]})
    sm = set(G.grow(seeds, adj, rings).tolist()) if rings else set(seeds)
    for ids in dup:
        if any(i in sm for i in ids): sm.update(int(i) for i in ids)
    sm = np.array(sorted(v for v in sm if not oral[v]))
    base = res.copy(); watch0 = np.flatnonzero(np.isin(T, sm).any(1))
    for _ in range(passes):
        nxt = res.copy()
        for v in sm: nxt[v] = (1 - alpha) * res[v] + alpha * res[sorted(adj[v])].mean(0)
        res = nxt
        for ids in dup: res[ids] = res[ids].mean(0)
    moved_mask = np.linalg.norm(res - base, axis=1) > 1e-12
    res[moved_mask] = res[moved_mask].astype(np.float32).astype(np.float64)
    watch = np.unique(np.r_[watch0, np.flatnonzero(np.isin(T, np.flatnonzero(moved_mask)).any(1))])
    final_pairs = pairset(res, watch); base_pairs = pairset(base, watch); donor_pairs = pairset(donor, watch)
    newp = sorted(final_pairs - base_pairs); gone = sorted(base_pairs - final_pairs); q = G.triangle_quality(base, res, T)
    new_ss = [(x, y) for x, y in newp if skinf[x] and skinf[y]]; new_sl = [(x, y) for x, y in newp if skinf[x] != skinf[y]]
    rem_ss = [(x, y) for x, y in final_pairs if skinf[x] and skinf[y]]
    rem_sl = [(x, y) for x, y in final_pairs if (x, y) not in donor_pairs and (skinf[x] != skinf[y])]
    base_sl = [(x, y) for x, y in base_pairs if (x, y) not in donor_pairs and (skinf[x] != skinf[y])]
    changed = np.flatnonzero(moved_mask); sk = [v for v in changed if skin_v[v]]
    src_err = None
    if source_soup is not None and sk:
        err_b = [float(source_soup.nearest(base[v])[1] * 1000) for v in sk]; err_a = [float(source_soup.nearest(res[v])[1] * 1000) for v in sk]
        src_err = {'n': len(sk), 'before_median': round(float(np.median(err_b)), 3), 'after_median': round(float(np.median(err_a)), 3), 'before_max': round(max(err_b), 3), 'after_max': round(max(err_a), 3)}
    accepted = (not new_ss) and (not new_sl) and (not rem_ss) and q['new_collapsed_triangles'] == 0
    log.update({'seed_faces': seed_faces, 'records_smoothed': int(len(sm)), 'changed_ids': [int(v) for v in changed], 'changed_count': int(len(changed)),
                'max_move_mm': round(float(np.linalg.norm(res - base, axis=1).max() * 1000), 3), 'median_move_mm': round(float(np.median(np.linalg.norm(res - base, axis=1)[changed]) * 1000), 3) if len(changed) else 0.0,
                'changed_by_class': {'skin': int(skin_v[changed].sum()), 'lining': int(lining_v[changed].sum()), 'oral': int(oral[changed].sum())},
                'gate': {'new_skin_skin': [[int(x), int(y)] for x, y in new_ss], 'new_skin_interior': [[int(x), int(y)] for x, y in new_sl], 'remaining_skin_skin': [[int(x), int(y)] for x, y in rem_ss],
                         'introduced_skin_lining_before': len(base_sl), 'introduced_skin_lining_after': [[int(x), int(y)] for x, y in rem_sl],
                         'interior_interior_new_documented': int(sum(1 for x, y in newp if not skinf[x] and not skinf[y])), 'interior_interior_gone': int(sum(1 for x, y in gone if not skinf[x] and not skinf[y]))},
                'pairs_gone': [[int(x), int(y)] for x, y in gone], 'pairs_new': [[int(x), int(y), ('skin' if skinf[x] else 'interior') + '/' + ('skin' if skinf[y] else 'interior')] for x, y in newp],
                'flips': q['orientation_flips'], 'collapsed': q['new_collapsed_triangles'], 'source_error_skin_records_mm': src_err, 'gate_accepted': bool(accepted),
                'seam_max_gap_m': max((float(np.linalg.norm(res[ids] - res[ids[0]], axis=1).max()) for ids in dup), default=0.0),
                'coverage_limits': 'pair tests are exact non-coplanar triangle intersections on non-adjacent faces; coplanar/adjacent contacts unknown'})
    if not accepted and not force:
        log['result'] = 'gate failed: head returned unchanged'; return base, log
    log['result'] = 'corrected' if accepted else 'corrected (gate failed, forced)'
    return res, log
