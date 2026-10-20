"""Compile the native bridge and its Windows test fixture; never install it."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
import panic_math
import panic_geometry
import panic_prediction
import ptm_trajectory_math


def build(output, compiler):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).resolve().parent / 'runtime/input_bridge.c'
    asset_source = source.with_name('asset_binding.c')
    panic_source = source.with_name('panic_state.c')
    service_source = source.with_name('panic_services.c')
    geometry_source = source.with_name('panic_geometry.c')
    closest_native_source = source.with_name('panic_closest_native.c')
    update_source = source.with_name('panic_update.c')
    lanes_source = source.with_name('panic_lanes.c')
    prediction_native_source = source.with_name('panic_prediction_native.c')
    lifecycle_source = source.with_name('panic_lifecycle.c')
    transition_source = source.with_name('ptm_transition.c')
    ptm_native_source = source.with_name('ptm_native.c')
    trajectory_source = source.with_name('ptm_trajectory.c')
    offsets_source = source.with_name('ptm_offsets.c')
    integration_source = source.with_name('ptm_integration.c')
    ptm_lifecycle_source = source.with_name('ptm_lifecycle.c')
    ptm_blend_timing_source = source.with_name('ptm_blend_timing.c')
    ptm_feature_pose_source = source.with_name('ptm_feature_pose.c')
    ptm_blend_storage_source = source.with_name('ptm_blend_storage.c')
    ptm_blend_snapshot_source = source.with_name('ptm_blend_snapshot.c')
    ptm_blend_snapshot_native_source = source.with_name('ptm_blend_snapshot_native.c')
    ptm_velocity_output_source = source.with_name('ptm_velocity_output.c')
    ptm_history_pose_source = source.with_name('ptm_history_pose.c')
    ptm_previous_pose_source = source.with_name('ptm_previous_pose.c')
    ptm_velocity_query_source = source.with_name('ptm_velocity_query.c')
    ptm_velocity_native_source = source.with_name('ptm_velocity_native.c')
    ptm_query_scope_source = source.with_name('ptm_query_scope.c')
    ptm_blend_query_source = source.with_name('ptm_blend_query.c')
    ptm_blend_frame_source = source.with_name('ptm_blend_frame.c')
    ptm_blend_warp_source = source.with_name('ptm_blend_warp.c')
    image = checked_image(ElfImage, Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'), SOURCE_HASH)
    math_data, math_receipt = panic_math.relocate(image)
    geometry_assembly, geometry_receipt = panic_geometry.geometry(image)
    prediction_assembly, prediction_receipt = panic_prediction.generate(image)
    trajectory_assembly, trajectory_receipt = ptm_trajectory_math.generate(image)
    math_source = output / 'panic-direction.s'
    math_source.write_text(panic_math.assembly(math_data)+geometry_assembly+trajectory_assembly+prediction_assembly)
    common = [str(compiler), '-O2', '-Wall', '-Wextra', '-Werror', '-ffreestanding',
              '-fno-builtin', '-fno-stack-protector', '-ffp-contract=off']
    commands = [
        common + ['-fuse-ld=lld', '-shared', '-nostdlib', '-Wl,/noentry', str(source), str(asset_source), str(panic_source), str(service_source), str(geometry_source), str(closest_native_source), str(lanes_source), str(prediction_native_source), str(update_source), str(lifecycle_source), str(transition_source), str(ptm_native_source), str(trajectory_source), str(offsets_source), str(integration_source), str(ptm_lifecycle_source), str(ptm_blend_timing_source), str(ptm_feature_pose_source), str(ptm_blend_storage_source), str(ptm_blend_snapshot_source), str(ptm_blend_snapshot_native_source), str(ptm_velocity_output_source), str(ptm_history_pose_source), str(ptm_previous_pose_source), str(ptm_velocity_query_source), str(ptm_velocity_native_source), str(ptm_query_scope_source), str(ptm_blend_query_source), str(ptm_blend_frame_source), str(ptm_blend_warp_source), str(math_source),
                  '-o', str(output / 'input_bridge.dll')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(source), '-o', str(output / 'input_bridge.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(asset_source), '-o', str(output / 'asset_binding.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(panic_source), '-o', str(output / 'panic_state.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(service_source), '-o', str(output / 'panic_services.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(geometry_source), '-o', str(output / 'panic_geometry.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(closest_native_source), '-o', str(output / 'panic_closest_native.o')],
        [str(compiler), '--target=x86_64-unknown-freebsd', '-c', str(math_source),
                  '-o', str(output / 'panic_math.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(update_source), '-o', str(output / 'panic_update.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(lanes_source), '-o', str(output / 'panic_lanes.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(prediction_native_source), '-o', str(output / 'panic_prediction_native.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(lifecycle_source), '-o', str(output / 'panic_lifecycle.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(transition_source), '-o', str(output / 'ptm_transition.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_native_source), '-o', str(output / 'ptm_native.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(trajectory_source), '-o', str(output / 'ptm_trajectory.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(offsets_source), '-o', str(output / 'ptm_offsets.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(integration_source), '-o', str(output / 'ptm_integration.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_lifecycle_source), '-o', str(output / 'ptm_lifecycle.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_timing_source), '-o', str(output / 'ptm_blend_timing.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_feature_pose_source), '-o', str(output / 'ptm_feature_pose.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_storage_source), '-o', str(output / 'ptm_blend_storage.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_snapshot_source), '-o', str(output / 'ptm_blend_snapshot.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_snapshot_native_source), '-o', str(output / 'ptm_blend_snapshot_native.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_velocity_output_source), '-o', str(output / 'ptm_velocity_output.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_history_pose_source), '-o', str(output / 'ptm_history_pose.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_previous_pose_source), '-o', str(output / 'ptm_previous_pose.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_velocity_query_source), '-o', str(output / 'ptm_velocity_query.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_velocity_native_source), '-o', str(output / 'ptm_velocity_native.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_query_scope_source), '-o', str(output / 'ptm_query_scope.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_query_source), '-o', str(output / 'ptm_blend_query.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_frame_source), '-o', str(output / 'ptm_blend_frame.o')],
        common + ['--target=x86_64-unknown-freebsd', '-mno-red-zone', '-fPIC', '-c',
                  str(ptm_blend_warp_source), '-o', str(output / 'ptm_blend_warp.o')],
    ]
    for command in commands:
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
    receipt = dict(status='compiled_component_not_integrated_or_installable', installed=False,
        source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
        asset_binding_source_sha256=hashlib.sha256(asset_source.read_bytes()).hexdigest(),
        panic_state_source_sha256=hashlib.sha256(panic_source.read_bytes()).hexdigest(),
        panic_prediction=prediction_receipt,
        panic_prediction_native_source_sha256=hashlib.sha256(prediction_native_source.read_bytes()).hexdigest(),
        panic_lanes_source_sha256=hashlib.sha256(lanes_source.read_bytes()).hexdigest(),
        panic_update_source_sha256=hashlib.sha256(update_source.read_bytes()).hexdigest(),
        panic_lifecycle_source_sha256=hashlib.sha256(lifecycle_source.read_bytes()).hexdigest(),
        ptm_transition_source_sha256=hashlib.sha256(transition_source.read_bytes()).hexdigest(),
        ptm_native_source_sha256=hashlib.sha256(ptm_native_source.read_bytes()).hexdigest(),
        ptm_trajectory_source_sha256=hashlib.sha256(trajectory_source.read_bytes()).hexdigest(),
        ptm_trajectory_math=trajectory_receipt,
        ptm_lifecycle_source_sha256=hashlib.sha256(ptm_lifecycle_source.read_bytes()).hexdigest(),
        ptm_blend_snapshot_source_sha256=hashlib.sha256(ptm_blend_snapshot_source.read_bytes()).hexdigest(),
        ptm_blend_snapshot_native_source_sha256=hashlib.sha256(ptm_blend_snapshot_native_source.read_bytes()).hexdigest(),
        ptm_velocity_output_source_sha256=hashlib.sha256(ptm_velocity_output_source.read_bytes()).hexdigest(),
        ptm_history_pose_source_sha256=hashlib.sha256(ptm_history_pose_source.read_bytes()).hexdigest(),
        ptm_previous_pose_source_sha256=hashlib.sha256(ptm_previous_pose_source.read_bytes()).hexdigest(),
        ptm_velocity_query_source_sha256=hashlib.sha256(ptm_velocity_query_source.read_bytes()).hexdigest(),
        ptm_velocity_native_source_sha256=hashlib.sha256(ptm_velocity_native_source.read_bytes()).hexdigest(),
        ptm_query_scope_source_sha256=hashlib.sha256(ptm_query_scope_source.read_bytes()).hexdigest(),
        ptm_blend_frame_source_sha256=hashlib.sha256(ptm_blend_frame_source.read_bytes()).hexdigest(),
        ptm_blend_warp_source_sha256=hashlib.sha256(ptm_blend_warp_source.read_bytes()).hexdigest(),
        ptm_blend_query_source_sha256=hashlib.sha256(ptm_blend_query_source.read_bytes()).hexdigest(),
        ptm_blend_storage_source_sha256=hashlib.sha256(ptm_blend_storage_source.read_bytes()).hexdigest(),
        ptm_feature_pose_source_sha256=hashlib.sha256(ptm_feature_pose_source.read_bytes()).hexdigest(),
        ptm_blend_timing_source_sha256=hashlib.sha256(ptm_blend_timing_source.read_bytes()).hexdigest(),
        ptm_offsets_source_sha256=hashlib.sha256(offsets_source.read_bytes()).hexdigest(),
        ptm_integration_source_sha256=hashlib.sha256(integration_source.read_bytes()).hexdigest(),
        panic_services_source_sha256=hashlib.sha256(service_source.read_bytes()).hexdigest(),
        panic_geometry=geometry_receipt,
        panic_closest_native_source_sha256=hashlib.sha256(closest_native_source.read_bytes()).hexdigest(),
        panic_geometry_source_sha256=hashlib.sha256(geometry_source.read_bytes()).hexdigest(),
        panic_direction=dict(source_eboot_sha256=SOURCE_HASH, **math_receipt),
        compiler=subprocess.run([str(compiler), '--version'], check=True,
                                capture_output=True, text=True).stdout.strip(),
        commands=commands,
        outputs={name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                 for name in ['input_bridge.dll', 'input_bridge.o', 'asset_binding.o', 'panic_state.o', 'panic_services.o', 'panic_geometry.o', 'panic_closest_native.o', 'panic_math.o', 'panic_update.o', 'panic_lanes.o', 'panic_prediction_native.o', 'panic_lifecycle.o', 'ptm_transition.o', 'ptm_native.o', 'ptm_trajectory.o', 'ptm_offsets.o', 'ptm_integration.o', 'ptm_lifecycle.o', 'ptm_blend_timing.o', 'ptm_feature_pose.o', 'ptm_blend_snapshot.o', 'ptm_blend_snapshot_native.o', 'ptm_velocity_output.o', 'ptm_history_pose.o', 'ptm_previous_pose.o', 'ptm_velocity_query.o', 'ptm_velocity_native.o', 'ptm_query_scope.o', 'ptm_blend_query.o', 'ptm_blend_frame.o', 'ptm_blend_warp.o', 'ptm_blend_storage.o']},
        unresolved=['native class/vtable and relocation integration',
                    'input producer and transition scheduling parity',
                    'activation and live qualification of paired allocation/publication hook',
                    'tick updates and producer write hooks',
                    'panic-turn runtime implementation', 'PTM compatibility requirements'])
    (output / 'input-bridge-build.json').write_text(json.dumps(receipt, indent=2))
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path,
        default=Path('local/nhl27-eboot/compatibility/deke-trial/runtime-build'))
    parser.add_argument('--compiler', type=Path, default=Path(r'C:\Program Files\LLVM\bin\clang.exe'))
    args = parser.parse_args()
    print(build(args.output, args.compiler))
