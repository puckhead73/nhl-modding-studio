"""Context database records, audited against NHL24 serializers 8903d0/8942e0.

Packed entries come from original EBX bytes, never JSON floating-point values.
The source's extra query slots must be inactive before using the target layout.
"""
import struct

LAYOUTS = {'ContextDatabase': (0xfaba13e8, 264), 'PriorityDatabase': (0xedd94816, 304),
           'ContextDatabaseAsset': (0x26954e01, 88), 'PriorityDatabaseAsset': (0x00b0a6a5, 88)}
INLINE_TYPES = {'ContextDatabase', 'PriorityDatabase'}


def emit_database(kind, f, at, data, pointer, array):
    used = {'Name'}

    def value(name):
        used.add(name)
        return f[name]

    def storage(site, count, stride):
        data.extend(bytes((-len(data)) % 16))
        start = array(site, site + 8, [None] * ((count * stride + 7) // 8))
        struct.pack_into('<Q', data, site + 8, count)
        return start

    def packed(offset, name, fmt):
        values = value(name)
        size = struct.calcsize('<' + fmt)
        start = storage(at + offset, len(values), size)
        for index, item in enumerate(values):
            struct.pack_into('<' + fmt, data, start + index * size, item)

    def strings(offset, name):
        values = value(name)
        start = storage(at + offset, len(values), 8)
        for index, text in enumerate(values):
            if '\0' in text:
                raise ValueError('Embedded NUL in database label')
            raw = text.encode('utf-8') + b'\0'
            p = array(start + index * 8, None, [None] * ((len(raw) + 7) // 8))
            data[p:p + len(raw)] = raw

    if value('HasLongBitmaps'):
        raise ValueError('Long source context bitmaps require a runtime adapter')
    if kind.endswith('Asset'):
        pointer(at + 24, value('DebugCategory'))
        pointer(at + 32, value('ContextDB'))
        pointer(at + 40, value('MirrorBool'), 0xbcc2c469)
        array(at + 48, at + 56, value('HACK_ownedAssets'))
        infos = value('PerValueAssetInfo')
        start = storage(at + 64, len(infos), 24)
        for index, info in enumerate(infos):
            if set(info) != {'ValueAsset', 'RemapAsset', 'FieldIndex', 'FieldInputSourceIndex', 'AutoShiftEnumsForBitmap'}:
                raise ValueError('Unexpected context per-value fields')
            p = start + index * 24
            pointer(p, info['ValueAsset'])
            pointer(p + 8, info['RemapAsset'])
            struct.pack_into('<HB?', data, p + 16, info['FieldIndex'], info['FieldInputSourceIndex'], info['AutoShiftEnumsForBitmap'])
        struct.pack_into('<I', data, at + 80, value('_native_debug_category_id'))
    else:
        pointer(at + 8, value('CDBAsset'))
        array(at + 16, at + 24, value('Assets'))
        packed(32, 'QueryToField', 'H')
        classes = value('Classes')
        start = storage(at + 48, len(classes), 16)
        for index, item in enumerate(classes):
            if set(item) != {'ClassGuid', 'FieldMapIndices', 'MetadataStartIdx', 'MetadataEndIdx'} or len(item['FieldMapIndices']) != 10:
                raise ValueError('Unsupported context class descriptor')
            struct.pack_into('<I12B', data, start + index * 16, item['ClassGuid'],
                             *item['FieldMapIndices'], item['MetadataStartIdx'], item['MetadataEndIdx'])
        fields = value('FieldMap')
        names = ('FieldType', 'FieldId', 'QueryIdx', 'DatabaseIdx', 'Int0', 'Int1', 'Int2', 'Float0', 'Float1')
        start = storage(at + 64, len(fields), 48)
        for index, item in enumerate(fields):
            if set(item) != set(names):
                raise ValueError('Unsupported context field-map descriptor')
            head = [item[n] for n in names[:4]]
            if any(not -(1 << 31) <= v < (1 << 32) for v in head):
                raise ValueError('Context field-map integer exceeds 32-bit storage')
            struct.pack_into('<4I3q2f', data, start + index * 48,
                             *(v & 0xffffffff for v in head), *(item[n] for n in names[4:]))
        packed(80, 'FieldInputSourceIndices', 'I')
        weights = value('WeightedFieldsInfo')
        if set(weights) != {'IsValid', 'Penalties', 'PackedPenalties', 'WeightedInputSourcesFlags'}:
            raise ValueError('Unsupported context weighted-field descriptor')
        struct.pack_into('<?', data, at + 96, weights['IsValid'])
        for offset, key in ((104, 'Penalties'), (120, 'PackedPenalties')):
            raw = bytes(weights[key])
            start = storage(at + offset, len(raw), 1)
            if raw:
                data[start:start + len(raw)] = raw
        struct.pack_into('<I', data, at + 136, weights['WeightedInputSourcesFlags'])
        passes = value('QueryPassInfo')
        start = storage(at + 144, len(passes), 108)
        for index, item in enumerate(passes):
            if set(item) != {'DbCompareIdxBase', 'QryCompareIdxBase', 'QryCompareIdxEnd'}:
                raise ValueError('Unsupported context query-pass descriptor')
            ends = item['QryCompareIdxEnd']
            if len(ends) != 32:
                raise ValueError('Unexpected source context comparison table length')
            # NHL27 inserts BITMAP_LONG at 4 and its MATCH_ENTRY variant at 9.
            # Consequently BOOL_EQUAL moves from 18 to 20 and ENUM_EQUAL_8
            # from 19 to 21. Original NHL24 bank/toe-drag records independently
            # confirm those destination slots; copying the first 25 is wrong.
            supported = {0, 1, 2, 3, 5, 6, 7, 8, 20, 21, 22, 23, 24}
            previous = item['QryCompareIdxBase']
            for operation, end in enumerate(ends):
                if end < previous:
                    raise ValueError('Context query comparison ranges are not monotonic')
                if end != previous and operation not in supported:
                    raise ValueError(f'Unqualified source context comparison operation {operation}')
                previous = end
            mapped = [end for operation, end in enumerate(ends[:27]) if operation not in (4, 9)]
            struct.pack_into('<27I', data, start + index * 108,
                             item['DbCompareIdxBase'], item['QryCompareIdxBase'], *mapped)
        raw_arrays = value('_native_raw_arrays')
        for offset, name in ((160, 'EntriesV'), (264, 'PrioritiesV')):
            if name == 'PrioritiesV' and kind != 'PriorityDatabase':
                continue
            values = value(name)
            raw = raw_arrays[name]
            if raw['stride'] != 16 or raw['count'] != len(values) or len(raw['data']) != 16 * len(values):
                raise ValueError('Packed database array size mismatch')
            start = storage(at + offset, len(values), 16)
            if values:
                data[start:start + len(raw['data'])] = raw['data']
        packed(176, 'DefaultQuery', 'B')
        strings(192, 'ClassNames')
        strings(208, 'FieldNames')
        counts = [value(n) for n in ('NumClasses', 'NumFields', 'NumEntries')]
        if counts[:2] != [len(classes), len(fields)]:
            raise ValueError('Context class/field counts disagree with arrays')
        # The source has no auxiliary rows. Native original banks use total =
        # regular + auxiliary; all source rows are retained as regular entries.
        counts.extend((0, counts[2]))
        counts.extend(value(n) for n in ('NumBytesPerEntry', 'NumBytesPerQuery',
                      'NumBytesForQueryDetailsHandler', 'FieldInputSourceUsageFlags'))
        struct.pack_into('<9I', data, at + 224, *counts)
        if kind == 'PriorityDatabase':
            packed(280, 'DefaultIndices', 'i')
            struct.pack_into('<I', data, at + 296, value('NumLayers'))
    if set(f) - used:
        raise ValueError(f'Unconsumed fields in {kind}: {sorted(set(f) - used)}')
