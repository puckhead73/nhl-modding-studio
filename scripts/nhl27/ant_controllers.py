"""NHL24 1.60 native controller/sequence records from audited serializers.

These are relocatable data records, not executable replacements. Source-only
features are accepted only in the explicitly supported inactive configurations.
"""
import struct
from ant_interfaces import BOOL, DEACTIVATE, BUFFERED, CONDITION

LAYOUTS = {
    'PTMWeights': (0xb718768e, 64),
    'AlternativeFeatureSchemaAsset': (0x302ef52f, 16),
    'PrevPoseBlendAsset': (0x91bd9d2c, 24),
    'StateFlowNodeControllerAsset': (0x184a86dd, 208),
    'GateNodeTransitionsInfo': (0xc2cd0bad, 32),
    'TransitionAsset': (0x8b7dc2e8, 96),
    'SequenceContainerAsset': (0x2f94b830, 56),
    'ActorControllerAsset': (0x82234926, 104),
    'ActorControllerInitParams': (0x257cd87d, 64),
    'DekeControllerAsset': (0x8c99ad91, 64),
    'DekeControllerBlenderAsset': (0x494a6c2e, 40),
    'DekeCoordAsset': (0xdcf6e651, 24),
    'PlaybackRateControllerAsset': (0x06959dee, 80),
    'LoopingControllerAsset': (0xc9e7e2f5, 64),
    'BlendCurveAsset': (0x8e3f8005, 24),
    'AccelBlendAsset': (0xd788243c, 24),
    'BranchOutPointTag': (0xa3e652c5, 80),
    'EvalBranchOutPointTag': (0xf7fcabb9, 128),
    'LoosePuckDekeLaunchTag': (0x791c9951, 40),
    'BranchOutTagCollectionAsset': (0xde6d31ff, 24),
    'EnumerationEnumeratorPair': (0x89f48d19, 32),
    'MotionClip': (0xb2b42593, 160),
    'JointState': (0x04e28100, 40),
    'SyncPointBlendSpaceControllerAsset': (0x34370ae1, 160),
    'SyncPointControllerAsset': (0xde384593, 72),
    'InitParamsND': (0x966f4b40, 40),
    'ChooserControllerAsset': (0xc50eba5f, 88),
    'EnumChooserEntryAsset': (0xa2c74181, 24),
    'EnumChooserPolicyAsset': (0x8b4ad0d4, 24),
    'BezierBlendAsset': (0x78a06220, 96),
    'BlendSpaceAsset': (0x47ffae51, 192),
    'BoolAsset': (0x33e57bd6, 104),
    'FloatAsset': (0x028883b0, 88),
    'ScenarioLocatorParamsAsset': (0x1b1c4c1f, 24),
    'ScenarioStatusBoolAsset': (0x805604a0, 32),
    'AntEnumerationAsset': (0x92de1d8f, 48),
    'EnumerationValueAsset': (0x2fb5a845, 32),
    'GameStateEnumerationAsset': (0x2196d161, 104),
    'GameStateEnumerationBitArrayAsset': (0x1d52f677, 40),
    'GameStateEnumerationBitArrayIntAsset': (0x56c76ee3, 24),
    'GameStateEnumerationBitArrayItemAsset': (0x94141c94, 40),
    'DebugCategoryAsset': (0xe9329a24, 16),
    'PoseHistoryPointerAsset': (0x1b333d4e, 32),
    'VelocityFeatureSchemaAsset': (0xf4f207b3, 112),
    'PositionFeatureSchemaAsset': (0xd4e91eac, 112),
}
INLINE_TYPES = {'JointState', 'GateNodeTransitionsInfo'}
VEC = ('0x29c0831c', '0xa53f7f32', '0xeafbb36d', '0x3d0f7ed4')


def emit_controller(kind, f, at, data, pointer, array):
    used = {'Name'}

    def value(name):
        used.add(name)
        return f[name]

    def scalar(offset, fmt, name):
        v = value(name)
        if fmt == 'I' and isinstance(v, int) and -(1 << 31) <= v < 0:
            v &= 0xffffffff
        struct.pack_into('<' + fmt, data, at + offset, v)

    def ref(offset, name):
        pointer(at + offset, value(name))

    def refs(offset, name):
        return array(at + offset, at + offset + 8, value(name))

    def records(site, count_site, values, stride):
        # Vector-bearing records require 16-byte aligned backing storage. Byte
        # arrays retain their logical count despite rounded allocation padding.
        data.extend(bytes((-len(data)) % 16))
        start = array(site, count_site, [None] * ((len(values) * stride + 7) // 8))
        struct.pack_into('<Q', data, count_site, len(values))
        return start

    def byte_array(site, count_site, values):
        start = records(site, count_site, values, 1)
        if values:
            data[start:start + len(values)] = bytes(values)

    def conditions(site, values):
        start = records(site, site + 8, values, 40)
        for i, condition in enumerate(values):
            if set(condition) != {'BoolAsset', 'DeactivateAsset', 'BufferedAsset', 'ConditionAsset', 'Required'}:
                raise ValueError('Unexpected transition condition fields')
            for offset, name, interface in ((0, 'BoolAsset', BOOL), (8, 'DeactivateAsset', DEACTIVATE),
                                           (16, 'BufferedAsset', BUFFERED), (24, 'ConditionAsset', CONDITION)):
                pointer(start + i * 40 + offset, condition[name], interface)
            struct.pack_into('<?', data, start + i * 40 + 32, condition['Required'])

    def string(offset, name):
        text = value(name)
        if '\0' in text:
            raise ValueError('Embedded NUL in native string')
        raw = text.encode('utf-8') + b'\0'
        p = array(at + offset, None, [None] * ((len(raw) + 7) // 8))
        data[p:p + len(raw)] = raw

    def vector(offset, name, components):
        v = value(name)
        if set(v) != set(VEC[:components]):
            raise ValueError('Unsupported vector component layout')
        struct.pack_into('<' + 'f' * components, data, at + offset, *(v[k] for k in VEC[:components]))

    if kind in ('SequenceContainerAsset', 'ActorControllerAsset', 'DekeControllerAsset',
                'DekeControllerBlenderAsset', 'PlaybackRateControllerAsset', 'LoopingControllerAsset',
                'SyncPointBlendSpaceControllerAsset', 'SyncPointControllerAsset', 'ChooserControllerAsset',
                'BlendSpaceAsset', 'StateFlowNodeControllerAsset'):
        ref(16, 'TagCollectionSetAsset')
    if kind == 'SequenceContainerAsset':
        refs(32, 'Actors')
        ref(48, 'TrackSelectionGameStateEnumeration')
    elif kind == 'ActorControllerAsset':
        ref(48, 'TrackSelectionEnumerator')
        ref(72, 'JointState')
        ref(80, 'DefaultInitParams')
        scalar(88, 'f', 'LoopPoint')
        scalar(92, 'f', 'Length')
        scalar(96, 'I', 'ActorContentFlags')
        scalar(100, '?', 'PassTagsToChildren')
        scalar(101, '?', 'SnapToStartLocation')
        tracks = value('Tracks')
        start = records(at + 56, at + 64, tracks, 40)
        for index, track in enumerate(tracks):
            if set(track) != {'Target', 'Warpers', 'Anims'}:
                raise ValueError('Unrecognized actor track fields')
            t = start + index * 40
            pointer(t, track['Target'])
            wstart = records(t + 8, t + 16, track['Warpers'], 104)
            for wi, w in enumerate(track['Warpers']):
                wp = wstart + wi * 104
                names = ['StartTime', 'Duration', 'BlendType', 'SmartWarperBlendSize',
                         'TranslateX', 'TranslateY', 'TranslateZ', 'Rotation',
                         'FacingAngle', 'FacingBegin', 'FacingEnd', 'IsFacingWarper',
                         'YTranslateFootEffectors', 'FullBodyYTranslation', 'IsSmartWarper']
                struct.pack_into('<HHII7f4?', data, wp, *(w[n] for n in names))
                if w['SmartWarperVels']:
                    raise ValueError('Nonempty smart warper velocities need native vector qualification')
                names += ['SmartWarperVels']
                for off, name in zip(range(64, 104, 8), ['YTranslateMainJointID',
                        'YTranslateLeftFootJointID', 'YTranslateRightFootJointID', 'WarpSignal', 'RotationSignal']):
                    pointer(wp + off, w[name])
                    names.append(name)
                if set(w) != set(names):
                    raise ValueError('Unrecognized warper fields')
            ast = records(t + 24, t + 32, track['Anims'], 40)
            for ai, anim in enumerate(track['Anims']):
                ap = ast + ai * 40
                for off, name in ((0, 'Asset'), (8, 'BlendAsset'), (16, 'InitParams')):
                    pointer(ap + off, anim[name])
                shorts = [anim[n] for n in ('Flags', 'StartTime', 'StartInTime', 'EndInTime')]
                if any(not -32768 <= v <= 65535 for v in shorts):
                    raise ValueError('Animation timing/flags exceed native 16-bit storage')
                struct.pack_into('<f4H', data, ap + 24, anim['Scale'], *(v & 0xffff for v in shorts))
                if set(anim) != {'Asset', 'BlendAsset', 'InitParams', 'Scale', 'Flags',
                                 'StartTime', 'StartInTime', 'EndInTime'}:
                    raise ValueError('Unrecognized sequence animation fields')
    elif kind == 'ActorControllerInitParams':
        vector(16, 'Orientation', 4)
        for off, name in ((32, 'PositionX'), (36, 'PositionY'), (40, 'PositionZ'),
                          (44, 'Scale'), (52, 'StartTick'), (56, 'FacingAngle')):
            scalar(off, 'f', name)
        scalar(48, 'I', 'Mirrored')
        scalar(60, '?', 'OrientationValid')
    elif kind == 'DekeControllerAsset':
        refs(24, 'DekeCoords')
        ref(40, 'DekeControllerSyncPointBlendSpaceRef')
        ref(48, 'DekeControllerAPI')
        scalar(56, 'I', 'DekeControllerBlenderPriority')
    elif kind == 'DekeControllerBlenderAsset':
        ref(24, 'SubjectControllerRef')
        ref(32, 'DekeControllerBlenderAPI')
    elif kind == 'DekeCoordAsset':
        for off, name in ((8, 'RAPosMag'), (12, 'RAPosAng'), (16, 'Mapping')):
            scalar(off, 'f', name)
        scalar(20, 'I', 'LPDNode')
    elif kind == 'PlaybackRateControllerAsset':
        for name, expected in [('ScaleType', 0), ('DurationGameState', None), ('ContinuousPhaseTime', False)]:
            if value(name) != expected:
                raise ValueError(f'Unsupported active playback-rate feature: {name}')
        # Duration is not used in source ScaleType=0 (time-scale) mode.
        value('Duration')
        for off, name in ((32, 'SubjectControllerRef'), (40, 'TimeScaleGameState'),
                          (48, 'SubjectTicksOutputGS'), (56, 'InitialTicksOffsetOverride')):
            ref(off, name)
        for off, name in ((64, 'TimeScale'), (68, 'MinScale'), (72, 'MaxScale'), (76, 'InitialTicksOffset')):
            scalar(off, 'f', name)
    elif kind == 'LoopingControllerAsset':
        ref(32, 'SubjectControllerAsset')
        ref(40, 'SubjectControllerInit')
        for off, name in ((48, 'StartTime'), (52, 'LoopStartTime'), (56, 'LoopEndTime')):
            scalar(off, 'f', name)
        scalar(60, '?', 'IsLoopControl')
    elif kind in ('BlendCurveAsset', 'AccelBlendAsset', 'BezierBlendAsset'):
        scalar(8, 'f', 'BlendTime')
        scalar(12, 'f', 'LaunchAngle')
        if kind == 'BlendCurveAsset':
            scalar(16, 'I', 'Curve')
            scalar(20, '?', 'ShouldSyncPhase')
        elif kind == 'AccelBlendAsset':
            scalar(16, '?', 'PreserveLaunchMoveAngle')
            scalar(17, '?', 'UseSlaveDeltaTrajectory')
        else:
            samples = value('BezierSamples')
            if len(samples) != 20:
                raise ValueError('Native Bezier sample count must be exactly twenty')
            struct.pack_into('<20f', data, at + 16, *samples)
    elif kind in ('BranchOutPointTag', 'EvalBranchOutPointTag', 'LoosePuckDekeLaunchTag'):
        scalar(8, 'f', 'StartLinkOffset')
        scalar(12, 'H', 'LinkFlags')
        scalar(14, 'B', 'StartLinkUpdates')
        scalar(16, 'f', 'EndLinkOffset')
        if kind == 'LoosePuckDekeLaunchTag':
            scalar(24, 'I', 'CharacterID')
            scalar(32, 'I', 'LPDLaunchType')
        else:
            for off, name in ((24, 'BlendAsset'), (32, 'BranchType'), (40, 'TargetAsset'), (48, 'TargetInitParams')):
                ref(off, name)
            scalar(56, 'f', 'DestinationPhase')
            scalar(60, '?', 'Active')
            scalar(64, 'I', 'ActivationPoint')
            scalar(68, 'f', 'StartTuningOffset')
            scalar(72, 'f', 'EndTuningOffset')
            if kind == 'EvalBranchOutPointTag':
                vector(96, 'PosPlayer', 3)
                for off, name in ((112, 'Speed'), (116, 'MoveAngle'), (120, 'BodyAngle')):
                    scalar(off, 'f', name)
    elif kind == 'BranchOutTagCollectionAsset':
        refs(8, 'Tags')
    elif kind == 'EnumerationEnumeratorPair':
        ref(16, 'GameStateEnumeration')
        ref(24, 'Enumerator')
    elif kind == 'MotionClip':
        ref(8, 'ClipAsset')
        scalar(16, 'Q', 'ClipAssetId')
        frames = value('FrameInfo')
        start = records(at + 24, at + 32, frames, 16)
        for i, frame in enumerate(frames):
            if set(frame) != {'FrameType', 'ClipTick', 'TimeScale', 'TrajTick', 'FootStatus'}:
                raise ValueError('Unsupported motion-frame fields')
            struct.pack_into('<IffHB', data, start + i * 16,
                *(frame[n] for n in ('FrameType', 'ClipTick', 'TimeScale', 'TrajTick', 'FootStatus')))
        if value('Event') or value('MetaDataBlob'):
            raise ValueError('Motion event/metadata conversion is not qualified')
        features = value('FeaturesBlob')
        start = records(at + 56, at + 64, features, 16)
        for i, vec in enumerate(features):
            if set(vec) != set(VEC):
                raise ValueError('Unsupported motion-feature vector layout')
            components = [vec[k] for k in VEC]
            if any(not isinstance(v, int) or not -(1 << 31) <= v < (1 << 32) for v in components):
                raise ValueError('Feature component is not a 32-bit word')
            struct.pack_into('<4I', data, start + i * 16, *(v & 0xffffffff for v in components))
        byte_array(at + 72, at + 80, value('PhaseQuotient'))
        refs(88, 'TrajectoryStates')
        refs(104, 'Slots')
        slots = value('PerSlotData')
        start = records(at + 120, at + 128, slots, 16)
        for i, slot in enumerate(slots):
            if set(slot) != {'Phase'}:
                raise ValueError('Unsupported per-slot motion fields')
            byte_array(start + i * 16, start + i * 16 + 8, slot['Phase'])
        scalar(136, 'I', 'FeaturesBlobNumInts')
        scalar(140, 'I', 'MirrorType')
        scalar(144, 'f', 'NumClipTicks')
        scalar(148, 'f', 'NumTicks')
        scalar(152, 'I', 'FrameTypes')
        scalar(156, '?', 'IsLooping')
        scalar(157, '?', 'IsUsed')
    elif kind == 'JointState':
        joints = value('JointInfos')
        start = records(at + 16, at + 24, joints, 64)
        for i, joint in enumerate(joints):
            p = start + i * 64
            for off, name in ((0, 'Bases'), (16, 'Scales')):
                if set(joint[name]) != set(VEC):
                    raise ValueError('Unsupported trajectory vector layout')
                struct.pack_into('<4f', data, p + off, *(joint[name][k] for k in VEC))
            byte_array(p + 32, p + 40, joint['Values'])
            pointer(p + 48, joint['JointIdAsset'])
            struct.pack_into('<IHH', data, p + 56,
                             joint['BakedChannels'], joint['StartTick'], joint['EndTick'])
            if set(joint) != {'Bases', 'Scales', 'Values', 'JointIdAsset', 'BakedChannels', 'StartTick', 'EndTick'}:
                raise ValueError('Unsupported trajectory joint fields')
        scalar(32, 'f', 'RecordDeltaT')
        scalar(36, '?', 'HasTrajAtIndex0')
        scalar(37, '?', 'ClampTime')
    elif kind == 'SyncPointBlendSpaceControllerAsset':
        for off, name in zip(range(32, 96, 8), ['SignalXRef', 'SignalYRef', 'CurrentXRef',
                'CurrentYRef', 'MasterControllerRef', 'MasterControllerInitParams',
                'SubjectBlendRef', 'SubjectControllerInitParams']):
            ref(off, name)
        refs(96, 'SyncPointOutputs')
        for off, name in zip(range(112, 148, 4), ['MasterLength', 'SignalXMaxInc',
                'SignalXMaxDec', 'SignalYMaxInc', 'SignalYMaxDec', 'BlendSpaceRegionXMin',
                'BlendSpaceRegionXMax', 'BlendSpaceRegionYMin', 'BlendSpaceRegionYMax']):
            scalar(off, 'f', name)
        for off, name in zip(range(148, 153), ['IsWrapX', 'IsWrapY', 'FreezeInputsOnBlendOut',
                'ForceSyncPointsToFullFrames', 'SupportsCycles']):
            scalar(off, '?', name)
    elif kind == 'SyncPointControllerAsset':
        points = value('SyncPoints')
        start = records(at + 32, at + 40, points, 4)
        if points:
            struct.pack_into('<' + 'f' * len(points), data, start, *points)
        ref(48, 'SubjectControllerInitParamsRef')
        ref(56, 'SubjectControllerRef')
        scalar(64, 'I', 'NumCycles')
        scalar(68, '?', 'MirrorSubject')
        scalar(69, '?', 'SupportsCycles')
    elif kind == 'InitParamsND':
        scalar(8, 'f', 'StartTick')
        scalar(16, 'I', 'NumItems')
        for off, name in zip(range(20, 36, 4), ['Item0', 'Item1', 'Item2', 'Item3']):
            scalar(off, 'f', name)
    elif kind == 'ChooserControllerAsset':
        ref(40, 'PolicyAsset')
        refs(48, 'Entries')
        ref(64, 'CurrentChoice')
        ref(72, 'BlendAsset')
        for off, name in zip(range(80, 88), ['ChooseOnReset', 'ChooseEveryUpdate', 'ChooseFirstUpdate',
                'Looping', 'ChooseOnLoop', 'PhaseMatchPrevious', 'LoopPreviousController', 'AllowRechooseOnBlendOut']):
            scalar(off, '?', name)
    elif kind == 'EnumChooserEntryAsset':
        ref(8, 'ControllerAsset')
        ref(16, 'EnumerationValue')
    elif kind == 'EnumChooserPolicyAsset':
        ref(8, 'EnumerationAsset')
        ref(16, 'GameStateAsset')
    elif kind == 'BlendSpaceAsset':
        diagram = value('VoronoiDiagram')
        def vec2(p, vec):
            if set(vec) != set(VEC[:2]):
                raise ValueError('Unsupported Voronoi vector components')
            struct.pack_into('<2f', data, p, *(vec[k] for k in VEC[:2]))
        for off, name in ((32, 'PosMin'), (48, 'PosMax'), (64, 'PosCentroid')):
            vec2(at + off, diagram[name])
        items = diagram['Items']
        start = records(at + 80, at + 88, items, 32)
        for i, item in enumerate(items):
            p = start + i * 32
            vec2(p, item['PosItem'])
            pointer(p + 16, item['Asset'])
            struct.pack_into('<2?', data, p + 24, item['XAxis'], item['Snap'])
            if set(item) != {'PosItem', 'Asset', 'XAxis', 'Snap'}:
                raise ValueError('Unsupported Voronoi item fields')
        triangles = diagram['Triangles']
        start = records(at + 96, at + 104, triangles, 128)
        for i, tri in enumerate(triangles):
            p = start + i * 128
            vector_names = [f'{prefix}_X{j:03d}' for prefix in ('PosVerts', 'VecEdgeNormal') for j in range(3)]
            for off, name in zip(range(0, 96, 16), vector_names):
                vec2(p + off, tri[name])
            indices = [tri[f'V_X{j:03d}'] for j in range(3)]
            if any(not 0 <= v < len(items) for v in indices):
                raise ValueError('Voronoi triangle references an absent item')
            struct.pack_into('<3I2f', data, p + 96, *indices, tri['Area'], tri['AreaInv'])
            if set(tri) != set(vector_names + ['V_X000', 'V_X001', 'V_X002', 'Area', 'AreaInv']):
                raise ValueError('Unsupported Voronoi triangle fields')
        edges = diagram['Edges']
        start = records(at + 112, at + 120, edges, 80)
        for i, edge in enumerate(edges):
            p = start + i * 80
            for off, name in zip(range(0, 64, 16), ['PosBegin', 'PosEnd', 'VecEdge', 'VecEdgeNormal']):
                vec2(p + off, edge[name])
            if not -1 <= edge['Triangle'] < len(triangles):
                raise ValueError('Voronoi edge references an absent triangle')
            struct.pack_into('<f?', data, p + 64, edge['Length'], edge['IsDegenerate'])
            struct.pack_into('<i', data, p + 72, edge['Triangle'])
            if set(edge) != {'PosBegin', 'PosEnd', 'VecEdge', 'VecEdgeNormal', 'Length', 'IsDegenerate', 'Triangle'}:
                raise ValueError('Unsupported Voronoi edge fields')
        grids = diagram['GridItems']
        start = records(at + 128, at + 136, grids, 16)
        for i, grid in enumerate(grids):
            if set(grid) != {'TriangleIndices'}:
                raise ValueError('Unsupported Voronoi grid fields')
            indices = grid['TriangleIndices']
            if any(not 0 <= v < len(triangles) for v in indices):
                raise ValueError('Voronoi grid references an absent triangle')
            p = records(start + i * 16, start + i * 16 + 8, indices, 4)
            if indices:
                struct.pack_into('<' + 'I' * len(indices), data, p, *indices)
        struct.pack_into('<2f3I', data, at + 144,
            *(diagram[n] for n in ('XGridStep', 'YGridStep', 'Rows', 'Columns', 'GridMode')))
        if set(diagram) != {'PosMin', 'PosMax', 'PosCentroid', 'Items', 'Triangles', 'Edges',
                           'GridItems', 'XGridStep', 'YGridStep', 'Rows', 'Columns', 'GridMode'}:
            raise ValueError('Unsupported Voronoi diagram fields')
        for off, name in ((176, 'ValueType'), (180, 'BlendMode'), (184, 'ClipMode')):
            scalar(off, 'I', name)
        scalar(188, 'f', 'SnapTolerance')
    elif kind == 'ScenarioStatusBoolAsset':
        # Factory 25d2b0; loader 25d360; registration 23f7a2..23f8fc.
        # The Boolean interface occupies +8, followed by four uint32 fields.
        for offset, name in ((16, 'StatusFlagsTrue'), (20, 'StatusFlagsFalse'),
                             (24, 'MatchModeTrue'), (28, 'MatchModeFalse')):
            scalar(offset, 'I', name)
    elif kind == 'ScenarioLocatorParamsAsset':
        # Native factory 245540 stores flags at +10; secondary params vtable
        # is at +8. The original launch-bank board locator has this layout.
        scalar(16, 'I', 'LocatorFlags')
    elif kind in ('BoolAsset', 'FloatAsset', 'GameStateEnumerationAsset'):
        # Initial value-asset layout from target factories, independently present
        # in serialized target Bool/enum records. Runtime-owned vtables stay zero.
        is_bool = kind == 'BoolAsset'
        is_float = kind == 'FloatAsset'
        expected = dict(LayoutHash=1 if is_bool else 10 if is_float else 6, ElementSize=1 if is_bool else 4,
            Offset=0, Name=1, Count=1, Flags=0, ElementAlign=1 if is_bool else 4,
            **{'0xaacc1dc': 1, 'LayoutPtr': 0})
        if value('0x6516a1e3') != expected:
            raise ValueError('Unsupported game-state entry layout')
        scalar(12, '?', '0xb1e00a80')
        struct.pack_into('<5I2H', data, at + 24, expected['LayoutHash'], expected['ElementSize'],
                         0, 1, 1, expected['ElementAlign'], 1)
        # Float factory 8e8e60 allocates 0x58 and stores the default at +0x50;
        # Bool/enum have an additional interface and use +0x58 instead.
        scalar(80 if is_float else 88, '?' if is_bool else 'f' if is_float else 'I', '0x3593b156')
        if is_bool:
            # Source runtime descriptor ecad810 -> e441810 names these hashes;
            # runtime member offsets differ from EBX offsets, so resolve by hash.
            # Target serializer 8e8a40 writes the corresponding flags at 96..98.
            for offset, name in ((96, '0xd950d3a4'), (97, '0xc0a0ea68'), (98, '0xb166b560')):
                scalar(offset, '?', name)
        elif not is_float:
            ref(96, '0xcbfd0e8f')
    elif kind == 'AntEnumerationAsset':
        refs(8, 'EnumerationValues')
        string(24, 'RuntimeName')
        scalar(32, 'I', 'NoneValue')
        scalar(36, 'I', 'HashId')
        scalar(40, '?', 'ZeroBasedSequential')
    elif kind == 'EnumerationValueAsset':
        ref(8, 'EnumerationAssetRef')
        string(16, 'RuntimeName')
        scalar(24, 'I', 'IntValue')
    elif kind == 'GameStateEnumerationBitArrayAsset':
        scalar(12, '?', '0xb1e00a80')
        ref(24, '0xcbfd0e8f')
        scalar(32, 'B', '0xad5c6098')
    elif kind == 'GameStateEnumerationBitArrayIntAsset':
        ref(16, 'GSEnumBitArray')
    elif kind == 'GameStateEnumerationBitArrayItemAsset':
        ref(24, 'EnumerationGameStateRef')
        scalar(32, 'I', 'ItemValue')
        scalar(36, '?', 'DeactivateAfterTransition')
    elif kind == 'TransitionAsset':
        # Native serializer aae820 uses four distinct interfaces for each
        # condition. Keeping four copies of the object base is invalid.
        for off, name in ((8, 'From'), (16, '0x2b6786f7'), (24, 'BranchWindowType'),
                          (32, 'BranchInWindowType'), (40, 'DestinationPhaseSignal'),
                          (48, 'Blend'), (72, 'TagCollectionSetAsset')):
            ref(off, name)
        conditions(at + 56, value('TransitionConditions'))
        for offset, fmt, name in ((80, 'f', 'DestinationPhase'), (84, 'f', 'DestinationTickOffset'),
                                  (88, 'i', 'BranchAtEndOffset'), (92, 'H', 'PhaseMode'), (94, 'H', 'Flags')):
            scalar(offset, fmt, name)
    elif kind == 'StateFlowNodeControllerAsset':
        for offset, name in ((24, 'StateFlowController'), (32, 'StateOpList'), (136, 'SubjectAsset'),
                             (144, 'SubjectControllerAsset'), (152, 'SubjectControllerInit'),
                             (192, 'GateNodeTransitionsInfo')):
            ref(offset, name)
        for offset, name, interface in ((40, 'ActiveBoolAssets', BOOL), (56, 'ActiveFloatAssets', 0x67745308),
                                        (72, 'ActiveDeactivationAssets', DEACTIVATE), (88, 'AllConditions', BOOL)):
            values = value(name)
            start = records(at + offset, at + offset + 8, values, 8)
            for i, item in enumerate(values):
                pointer(start + i * 8, item, interface)
        conditions(at + 104, value('EntryConditions'))
        refs(120, 'Transitions')
        refs(160, 'BranchTypeAssets')
        flattened = value('FlattenedTransitions')
        start = records(at + 176, at + 184, flattened, 64)
        for i, item in enumerate(flattened):
            expected = {'TransitionNode', 'Transition', 'GateNodeParams', 'TrueConditionIndices',
                        'FalseConditionIndices', 'IsNested', 'GateNodeTypeIdx', 'GateNodePuppetSiblingIdx',
                        'FirstChildIdx', 'SiblingIdx', 'BranchTypeIdx'}
            if set(item) != expected:
                raise ValueError('Unexpected flattened transition fields')
            p = start + i * 64
            for offset, name in ((0, 'TransitionNode'), (8, 'Transition'), (16, 'GateNodeParams')):
                pointer(p + offset, item[name])
            for offset, name in ((24, 'TrueConditionIndices'), (40, 'FalseConditionIndices')):
                values = item[name]
                if any(index < 0 or index >= len(f['AllConditions']) for index in values):
                    raise ValueError('Flattened transition condition index outside condition table')
                dest = records(p + offset, p + offset + 8, values, 2)
                for j, index in enumerate(values):
                    struct.pack_into('<H', data, dest + j * 2, index)
            struct.pack_into('<?5B', data, p + 56, *(item[n] for n in ('IsNested', 'GateNodeTypeIdx',
                'GateNodePuppetSiblingIdx', 'FirstChildIdx', 'SiblingIdx', 'BranchTypeIdx')))
        scalar(200, 'H', 'NodeType')
        scalar(202, 'H', 'EnumValue')
    elif kind == 'GateNodeTransitionsInfo':
        byte_array(at + 8, at + 16, value('FirstPuppetTransitionIndices'))
        scalar(24, 'I', 'GateNodeTypeCount')
    elif kind == 'AlternativeFeatureSchemaAsset':
        # Factory 3acbe0 allocates two interface vtables; serializer 3acc80 has
        # no fields. The loader restores the vtables from the registered type.
        pass
    elif kind == 'PrevPoseBlendAsset':
        # Factory 3ebb90: 24 bytes; base BlendAsset fields and serializer 3ebc40.
        scalar(8, 'f', 'BlendTime')
        scalar(12, 'f', 'LaunchAngle')
        ref(16, 'SubjectBlend')
    elif kind == 'PTMWeights':
        ref(8, 'KeepCurrentAnimBiasOverride')
        weights = value('FeatureComponentWeights')
        start = records(at + 16, at + 24, weights, 16)
        for i, vector_words in enumerate(weights):
            if set(vector_words) != set(VEC):
                raise ValueError('Unexpected PTM weight vector layout')
            words = [vector_words[k] for k in VEC]
            if any(not isinstance(w, int) or not -(1 << 31) <= w < (1 << 32) for w in words):
                raise ValueError('PTM weight is not an exact 32-bit word')
            struct.pack_into('<4I', data, start + i * 16, *(w & 0xffffffff for w in words))
        ref(32, 'PoseFeaturesRuntime')
        scalar(40, 'f', 'KeepCurrentAnimBias')
        scalar(44, 'f', 'RandomBias')
        scalar(48, 'I', 'FeatureComponentWeightsNumFloats')
        scalar(52, 'I', 'CostMetric')
        scalar(56, '?', 'EnableLegacyTrajFacingWeights')
        if not 0 <= f['FeatureComponentWeightsNumFloats'] <= 4 * len(weights):
            raise ValueError('PTM weight component count exceeds backing vectors')
        overrides = value('FeatureComponentWeightsOverride')
        if overrides and len(overrides) != f['FeatureComponentWeightsNumFloats']:
            raise ValueError('PTM override count differs from feature component count')
        if value('RandomBiasOverride') is not None or any(item is not None for item in overrides):
            raise ValueError('Active source-only PTM weight overrides require runtime support')
        if value('FailingDataminerName'):
            raise ValueError('Source PTM weights carry a dataminer failure')
        value('TweakAsset')
        if value('_native_tweak_defaults_verified') is not True:
            raise ValueError('PTM tool overrides have not been resolved and verified inactive')
    elif kind == 'DebugCategoryAsset':
        scalar(8, 'I', 'CategoryId')
        value('CategoryName')  # Source editor label; native type retains only ID.
    elif kind == 'PoseHistoryPointerAsset':
        scalar(12, '?', '0xb1e00a80')
        if value('0x3593b156') != 0:
            raise ValueError('Non-null source pose-history pointer default is unsupported')
    elif kind in ('VelocityFeatureSchemaAsset', 'PositionFeatureSchemaAsset'):
        for off, name in ((16, 'Joint'), (24, 'GameStateDriver'), (32, 'UpdateFromGameState'),
                          (48, 'SlotValue'), (56, 'ReferenceJoint'), (64, 'ReferenceSlotValue')):
            ref(off, name)
        if value('OverrideFilter') is not None:
            raise ValueError('Active source pose-feature filter override requires runtime support')
        stripped = value('StrippedComponents')
        if len(stripped) != 3:
            raise ValueError('Pose-feature component mask must contain three Booleans')
        struct.pack_into('<3?', data, at + 40, *stripped)
        scalar(72, 'I', 'FeatureSource')
        vector(80, 'DebugColor', 4)
        scalar(96, 'I', 'PoseTime')
        scalar(100, '?', 'Connect')
    else:
        raise ValueError(f'Unsupported controller record: {kind}')
    if set(f) - used:
        raise ValueError(f'Unconsumed fields in {kind}: {sorted(set(f) - used)}')
