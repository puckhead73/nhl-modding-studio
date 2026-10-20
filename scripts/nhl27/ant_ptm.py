"""NHL24 1.60 motion-matching memory-image records.

Layout evidence: factories/serializers 3cebd0/3cec80, 3ba6d0/3ba850,
3d5d40/3d5df0 and 3ebf00/3ebfb0. Source-only runtime semantics are
reported separately and MUST block linking until implemented or qualified.
"""
import struct
import math

from ant_interfaces import BOOL, CONDITION
from ant_ptm_parameters import LAYOUTS as PARAMETER_LAYOUTS, FIELDS as PARAMETER_FIELDS
from ptm_kd_qualification import qualified as kd_qualified

LAYOUTS = {
    'PTMMasterDatabaseAsset': (0xb3ff6eac, 80),
    'PTMDatabaseAsset': (0xa7129111, 504),
    'PTMPoseFeaturesRuntimeAsset': (0x7151cb46, 104),
    'PtmBlendAsset': (0x87cfbd8b, 80),
}
LAYOUTS.update(PARAMETER_LAYOUTS)
INLINE_TYPES = {'PTMDatabaseAsset'}
INITIALIZED_TYPES = {'PTMMasterDatabaseAsset'}
VEC = ('0x29c0831c', '0xa53f7f32', '0xeafbb36d', '0x3d0f7ed4')


def emit_ptm(kind, f, at, data, pointer, array):
    used, requirements = {'Name'}, []

    def value(name):
        used.add(name)
        return f[name]

    def scalar(offset, fmt, name):
        struct.pack_into('<' + fmt, data, at + offset, value(name))

    def ref(offset, name, interface=None):
        pointer(at + offset, value(name), interface)

    def records(site, values, stride):
        data.extend(bytes((-len(data)) % 16))
        start = array(site, site + 8, [None] * ((len(values) * stride + 7) // 8))
        struct.pack_into('<Q', data, site + 8, len(values))
        return start

    def numbers(site, values, fmt):
        start = records(site, values, struct.calcsize('<' + fmt))
        for i, item in enumerate(values):
            struct.pack_into('<' + fmt, data, start + i * struct.calcsize('<' + fmt), item)

    def words(site, values):
        start = records(site, values, 16)
        for i, item in enumerate(values):
            if set(item) != set(VEC) or any(type(item[k]) is not int or
                    not -(1 << 31) <= item[k] < (1 << 32) for k in VEC):
                raise ValueError('PTM packed vector must retain exact integer words')
            struct.pack_into('<4I', data, start + 16 * i, *(item[k] & 0xffffffff for k in VEC))

    def inactive(name, expected):
        if value(name) != expected:
            raise ValueError(f'Active source-only PTM field requires implementation: {name}')

    def unresolved(name, reason):
        requirements.append(dict(type=kind, field=name, source_value=value(name), reason=reason))

    if kind in PARAMETER_LAYOUTS:
        for name, (offset, fmt, interface) in PARAMETER_FIELDS[kind].items():
            if fmt == 'ptr':
                ref(offset, name, interface)
            elif fmt == 'ref_array':
                values = value(name)
                start = records(at + offset, values, 8)
                for i, item in enumerate(values):
                    pointer(start + i * 8, item, interface)
            elif fmt == 'empty_array':
                inactive(name, [])
            else:
                scalar(offset, fmt, name)
        # Source-only fields are not silently discarded. Null optional outputs
        # need no native storage; active/configuration differences remain a
        # mandatory integration requirement carried through the graph audit.
        null_fields = {
            'PTMMatchAsset': ('PrioritySelectedOutput',),
            'PTMMotionFilterParamsAsset': ('MotionFilterMirrored', 'MotionFilterExcludeMirrored'),
            'PTMWarpParamsAsset': ('MaxDesiredWarpPositionArcAngleOverride',
                'MaxDesiredWarpPositionArcDiffNearOverride', 'MaxDesiredWarpPositionArcDiffFarOverride'),
            'PTMLocomotionFilterParamsAsset': ('MaxSpeedToPlayStopOverride',
                'MaxConstantCycleSpeedDiffPercentOverride'),
            'PTMOutputParamsAsset': ('MoveAngleDiff', 'FaceDiff'),
            'PTMTimeScalingParamsAsset': ('MaxTimeScaleEventOverride', 'MinTimeScaleEventOverride'),
            'PTMBlendParamsAsset': ('MaxActiveBlendsOverride',),
        }
        extra_fields = {
            'PTMMatchAsset': ('UglyParams', 'BlendToTargetPoseTrajectory', 'KDAccuracyTestEnable'),
            'PTMWarpParamsAsset': ('MaxDesiredWarpPositionArcAngle', 'MaxDesiredWarpPositionArcDiffNear',
                'MaxDesiredWarpPositionArcDiffFar', 'PreserveAnimationFacing'),
            'PTMMotionWarpParamsAsset': ('MotionWarpTimeHorizonTicksDynamicShort', 'EnableStrideLogic'),
            'PTMLocomotionFilterParamsAsset': ('MaxSpeedToPlayStop', 'MaxConstantCycleSpeedDiffPercent'),
            'PTMTimeScalingParamsAsset': ('MaxTimeScaleEvent', 'MinTimeScaleEvent'),
        }
        for name in null_fields.get(kind, ()):
            inactive(name, None)
        for name in extra_fields.get(kind, ()):
            unresolved(name, 'Source-only PTM parameter requires consumer behavior qualification or runtime implementation')
    elif kind == 'PTMMasterDatabaseAsset':
        databases = value('Databases')
        array(at + 8, at + 16, databases)
        table = value('MotionTypeDatabaseTable')
        if any(i < -1 or i >= len(databases) for i in table):
            raise ValueError('Motion type table indexes an absent database')
        numbers(at + 24, table, 'i')
        ref(40, 'EvaluationActorProvider', 0xd1b595c1)
        ref(48, 'InterruptAnimationOnEnterOverride', BOOL)
        ref(56, 'InterruptAnimationOnExitOverride', BOOL)
        ref(64, 'PreviewScale', 0x67745308)
        for offset, name in enumerate(('InterruptAnimationOnEnter', 'InterruptAnimationOnExit',
                                       'FallbackToDefaultDatabase'), 72):
            scalar(offset, '?', name)
    elif kind == 'PTMDatabaseAsset':
        for offset, name in ((8, 'CompressedOffset'), (24, 'CompressedScale')):
            words(at + offset, value(name))
        for offset, name, interface in (
            (40, 'PriorityDatabase', CONDITION), (48, 'PriorityEnable', BOOL),
            (56, 'PTMFilter', CONDITION), (64, 'InterruptAnimationOnEnterOverride', BOOL),
            (72, 'InterruptAnimationOnExitOverride', BOOL),
            (80, 'InterruptAnimationOnLinkedCDBInvalidOverride', BOOL),
            (88, 'RandomValue', 0x0e0cfe7b), (96, 'LinkedCDB', 0x26954e01),
            (120, 'SideAccelvsTimeScaleCurve', 0xcfc9170f),
            (128, 'FwdBwdAccelvsTimeScaleCurve', 0xcfc9170f),
            (136, 'NumNNeighboursOverride', 0xa5edbe98), (144, 'WeightsRuntime', 0xb718768e),
            (168, 'MasterDatabase', 0xb3ff6eac)):
            ref(offset, name, interface)
        # These arrays are empty in all eleven source databases. Reject rather
        # than inventing the layout when importing a different source graph.
        for name in ('LinkedCDBEntryFrameRanges', 'EventLookup', 'PTMAABBImpls', 'PCAGridImpls'):
            inactive(name, [])
        clips = value('MotionClips')
        start = records(at + 176, clips, 8)
        for i, clip in enumerate(clips):
            pointer(start + 8 * i, clip, 0xb2b42593)
        pca = value('PCAData')
        if set(pca) != {'MeanVector', 'ProjMatrix', 'NumPoints', 'NumFeatures',
                        'NumPrincipalComponents', 'VarianceCoverage'}:
            raise ValueError('Unexpected PTM PCA layout')
        if len(pca['MeanVector']) != pca['NumFeatures'] or len(pca['ProjMatrix']) != (
                pca['NumFeatures'] * pca['NumPrincipalComponents']):
            raise ValueError('PTM PCA dimensions do not match backing arrays')
        if not all(math.isfinite(v) for v in pca['MeanVector'] + pca['ProjMatrix'] + [pca['VarianceCoverage']]):
            raise ValueError('PTM PCA contains non-finite numeric data')
        numbers(at + 192, pca['MeanVector'], 'f')
        numbers(at + 208, pca['ProjMatrix'], 'f')
        struct.pack_into('<IIIf', data, at + 224, *(pca[k] for k in (
            'NumPoints', 'NumFeatures', 'NumPrincipalComponents', 'VarianceCoverage')))
        name = value('DatabaseName')
        if '\0' in name:
            raise ValueError('Embedded NUL in PTM database name')
        raw = name.encode('utf-8') + b'\0'
        start = array(at + 240, None, [None] * ((len(raw) + 7) // 8))
        data[start:start + len(raw)] = raw
        scalar(248, 'Q', 'DatabaseId')
        for offset, name in ((256, 'PoseFeatureMirrorIndices'), (272, 'PoseFeatureComponentMirrorIndices')):
            numbers(at + offset, value(name), 'I')
        for offset, name in ((288, 'PoseFeatureComponentMirrorMultipliers'),
                             (304, 'PoseFeatureComponentScaleMultipliers')):
            words(at + offset, value(name))
        numbers(at + 328, value('PTMImplIdxHash'), 'B')
        kd = value('PTMKDImpls')
        start = records(at + 344, kd, 56)
        for i, item in enumerate(kd):
            if set(item) != {'Filter', 'KDIndexToClipIndex', 'KDTreeData', 'KDTreeDataNumBytes'}:
                raise ValueError('Unexpected PTM KD implementation record')
            p = start + i * 56
            filt = item['Filter']
            if set(filt) != {'ClipIdxFilter', 'IncludeFlags', 'ExcludeFlags', 'FilterOp'}:
                raise ValueError('Unexpected PTM motion filter record')
            struct.pack_into('<4I', data, p, *(filt[k] & 0xffffffff for k in (
                'ClipIdxFilter', 'IncludeFlags', 'ExcludeFlags', 'FilterOp')))
            numbers(p + 16, item['KDIndexToClipIndex'], 'I')
            words(p + 32, item['KDTreeData'])
            size = item['KDTreeDataNumBytes']
            if size != 16 * len(item['KDTreeData']):
                raise ValueError('PTM KD tree byte count differs from backing vectors')
            struct.pack_into('<I', data, p + 48, size)
        numbers(at + 400, value('EventTypeMap'), 'B')
        scalar_fields = (
            ('I', 'Unit'), ('f', 'PriorityCostThreshold'), ('I', 'FutureTimeHorizon'),
            ('I', 'PastTimeHorizon'), ('f', 'ClipTrimEndTicks'), ('f', 'ClipTrimEndPercent'),
            ('I', 'MaxRandomCandidates'), ('I', 'ClipFilterMode'), ('I', 'NumRefineSubdivs'),
            ('I', 'NumNNeighbours'), ('I', 'PCAGridBucketSize'), ('I', 'NumComponents'),
            ('f', 'FootVelocityThreshold'), ('I', 'StepSize'), ('I', 'MotionType'),
            ('I', 'PoseFeatureComponentMirrorMultipliersNumFloats'),
            ('I', 'PoseFeatureComponentScaleMultipliersNumFloats'))
        for i, (fmt, name) in enumerate(scalar_fields):
            scalar(416 + 4 * i, fmt, name)
        for i, name in enumerate(('UseCompressedFeatures', 'ExcludeNextNaturalFrames',
                'InterruptAnimationOnEnter', 'InterruptAnimationOnExit',
                'InterruptAnimationOnLinkedCDBInvalid', 'UseKNN', 'UseAABB',
                'EnableLegacyLocoFilters', 'PhaseQuotientMethod', 'DisableSecondSearch',
                'HasIdles', 'HasStarts', 'HasStops', 'HasCycles')):
            scalar(484 + i, '?', name)
        for name in ('MaxRandomCandidatesOverride', 'PTMImplIdxMap_OldAnt', 'EventTypeMap_OldAnt'):
            inactive(name, None)
        for name in ('FilterMotionInfoTagsByMotionTypesEnum', 'SortKNNResults',
                     'PCAGridIterateForward', 'ReconstructWithMetaData'):
            inactive(name, False)
        inactive('EarlyOutCostThreshold', 0.0)
        unresolved('MotionTypesEnumRef', 'Qualify source/target motion-type values and tag filtering semantics')
        unresolved('PoseFeatureReferenceSystemState', 'Qualify implicit target feature reference systems')
        if not kd_qualified(f):
            requirements.append(dict(type=kind, field='PTMKDImpls',
                reason='Packed KD query inputs differ from the source/native differential qualification'))
    elif kind == 'PTMPoseFeaturesRuntimeAsset':
        ref(8, 'PoseHistoryPointerAsset', 0x1b333d4e)
        schemas = value('Schema')
        start = records(at + 16, schemas, 8)
        for i, schema in enumerate(schemas):
            pointer(start + i * 8, schema, 0x19372c2c)
        offsets = value('FeatureComponentDataOffsets')
        if len(offsets) != len(schemas):
            raise ValueError('PTM feature offsets do not match schemas')
        numbers(at + 32, offsets, 'I')
        ref(48, 'SlotEnumerationGS', 0x2196d161)
        ref(56, 'PrimarySlot', 0x2fb5a845)
        ref(64, 'AnimatableId', CONDITION)
        for offset, fmt, name in ((72, 'I', 'NumFeatureComponents'),
                (76, 'I', 'NumFeatureComponentsPadded'), (80, 'I', 'NumFeatureComponentsVector'),
                (84, 'f', 'AlternativeTimeHorizonMin'), (88, 'f', 'AlternativeTimeHorizonStepSize'),
                (92, 'I', 'NumAlternativeTimeHorizons'), (96, 'I', 'NumCurrentFeatureComponents'),
                (100, '?', 'UseDebugColoring')):
            scalar(offset, fmt, name)
        for name in ('NumMetaDataComponents', 'NumCurrentMetaDataComponents', 'MetaDataFrameSize'):
            inactive(name, 0)
        inactive('MetaDataComponentDataOffsets', [0] * len(schemas))
        inactive('DisableParallelBuild', False)
        unresolved('TrajectoryJoint', 'Verify explicit source trajectory joint against target implicit joint')
    elif kind == 'PtmBlendAsset':
        scalar(8, 'f', 'BlendTime')
        scalar(12, 'f', 'LaunchAngle')
        for offset, name, interface in ((16, 'BlendInBlend', 0x90c15c64),
                (24, 'BlendInTimeOverride', 0x67745308), (40, 'BlendOutTimeOverride', 0x67745308),
                (48, 'BlendTimeGameState', 0x67745308), (56, 'MatchAsset', 0x5e84d979)):
            ref(offset, name, interface)
        scalar(64, 'f', 'BlendInTime')
        scalar(68, 'f', 'BlendOutTime')
        scalar(72, '?', 'UpdateMasterController')
        for name, expected in (('FutureFacing', None), ('FuturePosition', None),
                               ('MinBlendTime', 0.0), ('VelocityMode', 0), ('UseSlaveTrajectoryState', False)):
            inactive(name, expected)
        requirements.append(dict(type=kind, field='BlendOutBlend',
            reason='NHL24 update 0x369520 skips blend-out when asset+0x20 is null. '
                   'NHL27 update 0x2843c0 instead calls 0x27ffb0 at the remaining-time '
                   'threshold; that routine allocates 0x4a0 bytes and publishes transition '
                   'state at source PTM match+0xbbc0. This behavior needs a target implementation.'))
    else:
        raise ValueError(f'Unsupported PTM record: {kind}')
    if set(f) - used:
        raise ValueError(f'Unconsumed fields in {kind}: {sorted(set(f) - used)}')
    return requirements
