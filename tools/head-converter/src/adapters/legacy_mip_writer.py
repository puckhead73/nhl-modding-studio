"""Packed-mip-chain writer for one Legacy texlib slot (numpy only). Operates on COMPRESSED block streams: it never
decodes or re-encodes pixels, so a round trip of a slot's own levels reproduces the slot's bytes exactly at every
block address, and bytes outside the level addresses (tile padding, unused blocks) are never touched.

  slot_level_addresses(layout, bpb)      per level: absolute (offset, len) pairs inside the slot; proves coverage/overlap
  extract_levels(slot, layout, bpb)      original compressed blocks of every level (linear row-major streams)
  write_levels(slot, layout, bpb, blocks) place every level's blocks at their addresses in a copy of `slot`
  round_trip(slot, layout, bpb)          extract -> write into (a) a copy of the slot, (b) a zero slot; report exact
                                         equality of (a) and the byte set (b) cannot reproduce (padding outside levels)
  synthetic_injection(layout, bpb, n)    distinct per-level/per-block patterns written to a zero slot and read back:
                                         proves the writer writes each intended level, nothing else, with no overlap
  patch_texlib(raw, tex, layout, blocks) texlib bytes with only that texture's slot changed (whitelist = the slot)
"""
import numpy as np
from . import legacy_native as LN, legacy_texlib as TL


def slot_level_addresses(layout, bpb):
    return {l['level']: TL.level_block_addresses(l, bpb) for l in layout['levels']}


def coverage_report(layout, bpb, slot_bytes):
    addr = slot_level_addresses(layout, bpb); sets = {k: set(a for a, n in v) for k, v in addr.items()}
    overlaps = [(a, b, len(sets[a] & sets[b])) for a in sets for b in sets if a < b and sets[a] & sets[b]]
    covered = sum(len(v) * bpb for v in addr.values()); mx = max(a + n for v in addr.values() for a, n in v)
    return {'levels': len(addr), 'blocks_per_level': {k: len(v) for k, v in addr.items()}, 'bytes_covered_by_levels': covered, 'slot_bytes': slot_bytes, 'bytes_outside_levels': slot_bytes - covered, 'max_address_end': mx, 'within_slot': mx <= slot_bytes, 'overlapping_level_pairs': overlaps}


def extract_levels(slot, layout, bpb):
    return [TL.extract_level_blocks(slot, l, bpb) for l in layout['levels']]


def write_levels(slot, layout, bpb, blocks):
    if len(blocks) != len(layout['levels']): raise ValueError(f'{len(layout["levels"])} levels expected, {len(blocks)} given')
    out = bytearray(slot)
    for l, b in zip(layout['levels'], blocks): TL.write_level_blocks(out, l, bpb, b)
    return bytes(out)


def round_trip(slot, layout, bpb):
    """(a) extract every level's original blocks and write them back into a copy of the slot: must be byte-identical;
    (b) write them into a zero slot: bytes that differ from the original are exactly the nonzero bytes outside any
    level address (padding/opaque) - listed with offsets, never silently zeroed by a caller."""
    blocks = extract_levels(slot, layout, bpb); back = write_levels(slot, layout, bpb, blocks); zero = write_levels(bytes(len(slot)), layout, bpb, blocks)
    a = np.frombuffer(slot, np.uint8); z = np.frombuffer(zero, np.uint8); diff = np.flatnonzero(a != z)
    return {'copy_round_trip_exact': back == bytes(slot), 'zero_round_trip_exact': len(diff) == 0, 'bytes_not_reproducible_from_levels': int(len(diff)), 'first_offsets': diff[:32].tolist(), 'cause': 'nonzero bytes outside every level block address (tile padding / unused blocks); a writer must carry them from the source slot, which write_levels does' if len(diff) else None, 'level_bytes': [len(b) for b in blocks]}


def synthetic_injection(layout, bpb, slot_bytes, seed=1):
    rng = np.random.default_rng(seed); blocks = []
    for l in layout['levels']:
        w, h = l['dim']; n = max(w // 4, 1) * max(h // 4, 1); pat = bytearray(rng.integers(1, 256, n * bpb, dtype=np.uint8).tobytes())
        for k in range(n): pat[k * bpb] = (l['level'] + 1) & 0xFF; pat[k * bpb + 1] = (k % 255) + 1      # level tag + block index, never zero
        blocks.append(bytes(pat))
    written = write_levels(bytes(slot_bytes), layout, bpb, blocks); back = extract_levels(written, layout, bpb)
    nz = int(np.count_nonzero(np.frombuffer(written, np.uint8))); expected = sum(len(b) for b in blocks)
    return {'every_level_reads_back_identically': all(x == y for x, y in zip(blocks, back)), 'nonzero_bytes_written': nz, 'expected_level_bytes': expected, 'no_bytes_written_outside_levels': nz == expected, 'per_level_ok': [x == y for x, y in zip(blocks, back)]}


def patch_texlib(raw, tex, layout, blocks):
    """New texlib bytes where only `tex`'s slot changes: the slot is copied from `raw` (padding preserved) and every
    level's blocks are written at their addresses. Returns (bytes, {changed_bytes, changed_outside_slot, changed_outside_level_addresses})."""
    s0, n = tex['slot_start'], tex['slot_bytes']; slot = raw[s0:s0 + n]; new_slot = write_levels(slot, layout, tex['bytes_per_block'], blocks)
    out = raw[:s0] + new_slot + raw[s0 + n:]; a = np.frombuffer(raw, np.uint8); b = np.frombuffer(out, np.uint8); changed = a != b
    allowed = np.zeros(len(raw), bool)
    for v in slot_level_addresses(layout, tex['bytes_per_block']).values():
        for off, ln in v: allowed[s0 + off:s0 + off + ln] = True
    return out, {'changed_bytes': int(changed.sum()), 'changed_outside_slot': int(changed[:s0].sum() + changed[s0 + n:].sum()), 'changed_outside_level_addresses': int((changed & ~allowed).sum()), 'same_length': len(out) == len(raw)}
