"""Release controller action ownership before native actor-index unassignment.

Source 46d66c0 resets the action feature before 46d6590, even when the actor's
controller index is already negative. Target 2a33e60 unrolls the same 12-record
loop (controller stride f0 vs source b0). Hook its calls, not every unassignment
caller: unrelated callers have not been shown to reset the feature early.

The 46d1060 branch at 46d1562 performs the same release loop, inlined and
unrolled in target 2a2a1b0. Target 2a33660 additionally inlines 46d4b30's full
controller reset; the feature must be cleared before its 2a336a0 unassignment.

The special-state reset 46f88c0 is inlined at target 2b9b7db. Its state-2
write and selected-actor comparison lead to 2b9b874 -> 2a33e60, so the same
12 RELEASE_SITES cover it. It does not need a second set of reset hooks.
"""
import struct
import input_frame_hook

ORIGINAL = 0x2a2cc20
RELEASE_SITES = (0x2a33e82, 0x2a33ebf, 0x2a33efc, 0x2a33f39,
         0x2a33f76, 0x2a33fb3, 0x2a33ff0, 0x2a3402d,
         0x2a3406a, 0x2a340a7, 0x2a340e4, 0x2a3411d)
INLINE_RELEASE_SITES = (0x2a2a2fa, 0x2a2a337, 0x2a2a374, 0x2a2a3b1,
         0x2a2a3ee, 0x2a2a42b, 0x2a2a468, 0x2a2a4a5,
         0x2a2a4e2, 0x2a2a51f, 0x2a2a55c, 0x2a2a599)
FULL_RESET_SITES = (0x2a336a0, 0x2a2ab51, 0x2a2c0cb, 0x2a2c21a, 0x2a2cb31, 0x2a37094)
# Both source 46d9480 branches reset: first via 46d6480 at 46d9582,
# second inline at 46d97af. Following the callee is essential here.
HANDOFF_RESET_SITES = (0x2a2eac2, 0x2a2ed7a)
# Source 46e1d00 timeout release resets at 46e1ed5 before 46e1edd.
# Target keeps its native inactivity predicate but gains that reset boundary.
TIMEOUT_RESET_SITES = (0x2a4ea3b,)
# Source 46db1e0 is inlined in native possession acquisition 2818fe0.
# Only its final auto-selection branch resets the old actor's action state;
# the two earlier goalie-selection branches deliberately remain unchanged.
AUTOSELECT_RESET_SITES = (0x281a1e0,)
SITES = (RELEASE_SITES + INLINE_RELEASE_SITES + FULL_RESET_SITES
         + HANDOFF_RESET_SITES + TIMEOUT_RESET_SITES + AUTOSELECT_RESET_SITES)


def expected(address):
    return b'\xe8' + struct.pack('<i', ORIGINAL-address-5)


def assembly(helper='nhl24_input_assign_actor_guest', base_symbol=None, original=None):
    return '\n'.join(input_frame_hook.assembly(helper, f'input_release_{address:x}',
        base_symbol, original, address, ORIGINAL, ('mov rsi, rdi', 'mov edx, -1'))
        for address in SITES)
