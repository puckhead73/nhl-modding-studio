"""Export lifecycle helpers (stdlib only): a JSON-lines event stream and owned subprocess trees.

Events `nhl-head-export-event/1` (one object per line in <export>/events.jsonl, optionally on stdout): fields schema, seq
(monotonic from 1), ts, export_id, run_id, event, elapsed_s, plus stage {name, index} / status / subprocess / error / cleanup as
applicable. Names: export_started, stage_started, subprocess_started, subprocess_exited, stage_finished, export_completed,
export_failed, export_cancelled (terminal: at most one per run; the log refuses a second) and export_refused (stdout only; no
directory exists). A terminal event is CLAIMED (`EventLog.terminal`) before its line is written, so a second terminal can never be
attempted; whether the claimed line actually reached the log is `terminal_delivery` (`written` / `failed: ...` / `unknown` when the
I/O was interrupted) - a claim is not a delivery. The worker's `nhl-head-worker-event/1` contract is untouched; the field shapes follow
it where they overlap.

Owned processes (Windows): ownership is established BEFORE any child code runs. A Job Object with KILL_ON_JOB_CLOSE is created
first (failure -> the launch is refused); the child is created CREATE_SUSPENDED (hidden window, redirected log) and assigned to
the job while still suspended; only then is its primary thread resumed. Descendants (Blender -> native packer) inherit the job.
Assignment or resume failure terminates and reaps the never-run child and closes every handle; there is no unowned fallback.
`wait_tree()` confirms the whole job is empty (ActiveProcesses == 0) before a stage may advance, terminating lingering descendants
within a bound; `terminate_tree()` kills the job and confirms emptiness the same way. `release()` is the unconditional finaliser the
exporter runs whatever happened (normal exit, cancellation, an interrupt or accounting failure inside the drain, an event-delivery
error): unless emptiness was already confirmed it terminates the owned tree and attempts bounded confirmation, and it ALWAYS closes
the job handle (kill-on-close then kills anything still owned even if confirmation failed); its report says what was measured, never
what was assumed. The job handle dies with this process, so an abrupt exporter death kills the owned tree as well (no receipt is
written in that case). POSIX: a new session is started and the process group is killed; descendant emptiness cannot be confirmed there
(`descendants: 'unverified'`), which is reported, not hidden. Nothing here kills by executable name or touches unrelated processes."""
import ctypes
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

EVENT_SCHEMA = 'nhl-head-export-event/1'
CANCEL_FILE = 'CANCEL'
CLEANUP_TIMEOUT_S = 10.0
STILL_ACTIVE = 259


class Cancelled(Exception):
    """A cancellation request (CANCEL file or KeyboardInterrupt) observed before completion."""


class OwnershipError(OSError):
    """Process-tree ownership could not be established; the child never ran (or was terminated before running)."""


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%S')


class EventLog:
    def __init__(self, path, export_id, run_id, to_stdout=False):
        self.path = Path(path); self.export_id = export_id; self.run_id = run_id; self.to_stdout = to_stdout
        self.seq = 0; self.t0 = time.time()
        self.terminal = None            # terminal event name once CLAIMED (before its write): the log refuses any later event
        self.terminal_delivery = None   # 'attempted' -> 'written' | 'failed: <error>' | 'unknown' (interrupted mid-I/O)
        self.stream = self.path.open('a', encoding='utf-8')

    @property
    def terminal_written(self):
        """True only when the claimed terminal line was written, flushed and fsynced to events.jsonl."""
        return self.terminal_delivery == 'written'

    def emit(self, event, **fields):
        if self.terminal is not None: raise RuntimeError(f'event {event} after terminal event {self.terminal}')
        self.seq += 1
        ev = {'schema': EVENT_SCHEMA, 'seq': self.seq, 'ts': now(), 'export_id': self.export_id, 'run_id': self.run_id, 'event': event}
        ev.update({k: v for k, v in fields.items() if v is not None}); ev['elapsed_s'] = round(time.time() - self.t0, 3)
        line = json.dumps(ev, ensure_ascii=False, default=str)
        terminal = event.startswith('export_') and event != 'export_started'
        if terminal: self.terminal = event; self.terminal_delivery = 'attempted'   # claimed before writing: never two terminals
        try:
            self.stream.write(line + '\n'); self.stream.flush(); os.fsync(self.stream.fileno())
        except BaseException as exc:
            if terminal: self.terminal_delivery = 'unknown' if isinstance(exc, KeyboardInterrupt) else f'failed: {type(exc).__name__}: {exc}'
            raise
        if terminal: self.terminal_delivery = 'written'
        if self.to_stdout: print(line, flush=True)
        return ev

    def close(self):
        try: self.stream.close()
        except OSError: pass


def stdout_event(event, **fields):
    """A single stdout-only event (used for preflight refusals, where no directory exists yet)."""
    ev = {'schema': EVENT_SCHEMA, 'seq': 1, 'ts': now(), 'export_id': None, 'run_id': None, 'event': event, **fields, 'elapsed_s': 0.0}
    print(json.dumps(ev, ensure_ascii=False, default=str), flush=True)
    return ev


# ---- Windows Job Objects and suspended launch -----------------------------------------------------------------------
if os.name == 'nt':
    import ctypes.wintypes as wt
    _k32 = ctypes.WinDLL('kernel32', use_last_error=True)
    _JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x2000
    _JobObjectExtendedLimitInformation, _JobObjectBasicAccountingInformation = 9, 1
    _PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    _TH32CS_SNAPTHREAD, _THREAD_SUSPEND_RESUME = 0x4, 0x2
    _CREATE_SUSPENDED = 0x4
    _INVALID_HANDLE_VALUE = wt.HANDLE(-1).value

    class _IO_COUNTERS(ctypes.Structure):
        _fields_ = [(n, ctypes.c_ulonglong) for n in ('ReadOperationCount', 'WriteOperationCount', 'OtherOperationCount', 'ReadTransferCount', 'WriteTransferCount', 'OtherTransferCount')]

    class _BASIC_LIMIT(ctypes.Structure):
        _fields_ = [('PerProcessUserTimeLimit', wt.LARGE_INTEGER), ('PerJobUserTimeLimit', wt.LARGE_INTEGER), ('LimitFlags', wt.DWORD),
                    ('MinimumWorkingSetSize', ctypes.c_size_t), ('MaximumWorkingSetSize', ctypes.c_size_t), ('ActiveProcessLimit', wt.DWORD),
                    ('Affinity', ctypes.c_size_t), ('PriorityClass', wt.DWORD), ('SchedulingClass', wt.DWORD)]

    class _EXTENDED_LIMIT(ctypes.Structure):
        _fields_ = [('BasicLimitInformation', _BASIC_LIMIT), ('IoInfo', _IO_COUNTERS), ('ProcessMemoryLimit', ctypes.c_size_t),
                    ('JobMemoryLimit', ctypes.c_size_t), ('PeakProcessMemoryUsed', ctypes.c_size_t), ('PeakJobMemoryUsed', ctypes.c_size_t)]

    class _ACCOUNTING(ctypes.Structure):
        _fields_ = [('TotalUserTime', wt.LARGE_INTEGER), ('TotalKernelTime', wt.LARGE_INTEGER), ('ThisPeriodTotalUserTime', wt.LARGE_INTEGER),
                    ('ThisPeriodTotalKernelTime', wt.LARGE_INTEGER), ('TotalPageFaultCount', wt.DWORD), ('TotalProcesses', wt.DWORD),
                    ('ActiveProcesses', wt.DWORD), ('TotalTerminatedProcesses', wt.DWORD)]

    class _THREADENTRY32(ctypes.Structure):
        _fields_ = [('dwSize', wt.DWORD), ('cntUsage', wt.DWORD), ('th32ThreadID', wt.DWORD), ('th32OwnerProcessID', wt.DWORD),
                    ('tpBasePri', wt.LONG), ('tpDeltaPri', wt.LONG), ('dwFlags', wt.DWORD)]

    _k32.CreateJobObjectW.restype = wt.HANDLE; _k32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wt.LPCWSTR]
    _k32.SetInformationJobObject.restype = wt.BOOL; _k32.SetInformationJobObject.argtypes = [wt.HANDLE, ctypes.c_int, ctypes.c_void_p, wt.DWORD]
    _k32.QueryInformationJobObject.restype = wt.BOOL; _k32.QueryInformationJobObject.argtypes = [wt.HANDLE, ctypes.c_int, ctypes.c_void_p, wt.DWORD, ctypes.POINTER(wt.DWORD)]
    _k32.AssignProcessToJobObject.restype = wt.BOOL; _k32.AssignProcessToJobObject.argtypes = [wt.HANDLE, wt.HANDLE]
    _k32.TerminateJobObject.restype = wt.BOOL; _k32.TerminateJobObject.argtypes = [wt.HANDLE, wt.UINT]
    _k32.CloseHandle.restype = wt.BOOL; _k32.CloseHandle.argtypes = [wt.HANDLE]
    _k32.OpenProcess.restype = wt.HANDLE; _k32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
    _k32.GetExitCodeProcess.restype = wt.BOOL; _k32.GetExitCodeProcess.argtypes = [wt.HANDLE, ctypes.POINTER(wt.DWORD)]
    _k32.CreateToolhelp32Snapshot.restype = wt.HANDLE; _k32.CreateToolhelp32Snapshot.argtypes = [wt.DWORD, wt.DWORD]
    _k32.Thread32First.restype = wt.BOOL; _k32.Thread32First.argtypes = [wt.HANDLE, ctypes.POINTER(_THREADENTRY32)]
    _k32.Thread32Next.restype = wt.BOOL; _k32.Thread32Next.argtypes = [wt.HANDLE, ctypes.POINTER(_THREADENTRY32)]
    _k32.OpenThread.restype = wt.HANDLE; _k32.OpenThread.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
    _k32.ResumeThread.restype = wt.DWORD; _k32.ResumeThread.argtypes = [wt.HANDLE]

    def create_kill_on_close_job():
        job = _k32.CreateJobObjectW(None, None)
        if not job: raise OwnershipError(ctypes.get_last_error(), 'CreateJobObjectW failed')
        info = _EXTENDED_LIMIT(); info.BasicLimitInformation.LimitFlags = _JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not _k32.SetInformationJobObject(job, _JobObjectExtendedLimitInformation, ctypes.byref(info), ctypes.sizeof(info)):
            err = ctypes.get_last_error(); _k32.CloseHandle(job); raise OwnershipError(err, 'SetInformationJobObject failed')
        return job

    def assign_to_job(job, process_handle):
        if not _k32.AssignProcessToJobObject(job, process_handle):
            raise OwnershipError(ctypes.get_last_error(), 'AssignProcessToJobObject failed')

    def job_active_processes(job):
        acc = _ACCOUNTING()
        if not _k32.QueryInformationJobObject(job, _JobObjectBasicAccountingInformation, ctypes.byref(acc), ctypes.sizeof(acc), None):
            raise OSError(ctypes.get_last_error(), 'QueryInformationJobObject failed')
        return int(acc.ActiveProcesses)

    def resume_process_threads(pid):
        """Resume every thread of a CREATE_SUSPENDED process (it has exactly one until it runs). Returns the count resumed."""
        snap = _k32.CreateToolhelp32Snapshot(_TH32CS_SNAPTHREAD, 0)
        if snap == _INVALID_HANDLE_VALUE or not snap: raise OwnershipError(ctypes.get_last_error(), 'CreateToolhelp32Snapshot failed')
        resumed = 0
        try:
            entry = _THREADENTRY32(); entry.dwSize = ctypes.sizeof(_THREADENTRY32)
            ok = _k32.Thread32First(snap, ctypes.byref(entry))
            while ok:
                if entry.th32OwnerProcessID == pid:
                    th = _k32.OpenThread(_THREAD_SUSPEND_RESUME, False, entry.th32ThreadID)
                    if not th: raise OwnershipError(ctypes.get_last_error(), 'OpenThread failed')
                    try:
                        if _k32.ResumeThread(th) == 0xFFFFFFFF: raise OwnershipError(ctypes.get_last_error(), 'ResumeThread failed')
                    finally: _k32.CloseHandle(th)
                    resumed += 1
                ok = _k32.Thread32Next(snap, ctypes.byref(entry))
        finally: _k32.CloseHandle(snap)
        if resumed == 0: raise OwnershipError(0, f'no suspended thread found for pid {pid}')
        return resumed

    def process_alive(pid):
        """True while the process object reports STILL_ACTIVE (query-limited access; never signals or kills)."""
        h = _k32.OpenProcess(_PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
        if not h: return False
        try:
            code = wt.DWORD()
            if not _k32.GetExitCodeProcess(h, ctypes.byref(code)): return False
            return code.value == STILL_ACTIVE
        finally: _k32.CloseHandle(h)
else:
    def process_alive(pid):
        try: os.kill(int(pid), 0); return True
        except ProcessLookupError: return False
        except PermissionError: return True


class OwnedProcess:
    """A subprocess whose descendant tree this exporter owns from before its first instruction."""
    def __init__(self, command, stdout, cwd):
        self.command = [str(c) for c in command]; self.job = None; self.proc = None; self.ownership = None
        self.confirmed_empty = False   # set only by a wait_tree / terminate_tree that measured ActiveProcesses == 0
        self.release_report = None     # release() is idempotent; the first report stands
        kwargs = {'stdout': stdout, 'stderr': subprocess.STDOUT, 'cwd': str(cwd)}
        try:
            if os.name == 'nt':
                self.job = create_kill_on_close_job()          # 1. own the container first; failure refuses the launch
                kwargs['creationflags'] = getattr(subprocess, 'CREATE_NO_WINDOW', 0) | _CREATE_SUSPENDED
                self.proc = subprocess.Popen(self.command, **kwargs)   # 2. child exists but has not executed an instruction
                assign_to_job(self.job, int(self.proc._handle))       # 3. ownership
                resume_process_threads(self.proc.pid)                 # 4. only now may it run
                self.ownership = {'mechanism': 'windows_job_object', 'tree_owned': True, 'kill_on_close': True, 'launched_suspended': True}
            else:
                kwargs['start_new_session'] = True
                self.proc = subprocess.Popen(self.command, **kwargs)
                self.ownership = {'mechanism': 'posix_session', 'tree_owned': True, 'descendants': 'unverified'}
        except BaseException:
            self._abort_construction(); raise
        self.started = time.time()

    def _abort_construction(self):
        """Spawn/assign/resume failure or an interrupt during construction: nothing owned may keep running or leak."""
        if self.proc is not None:
            try: self.proc.kill()
            except OSError: pass
            try: self.proc.wait(timeout=CLEANUP_TIMEOUT_S)
            except subprocess.TimeoutExpired: pass
        if self.job is not None:
            try: _k32.TerminateJobObject(self.job, 1)
            except OSError: pass
            _k32.CloseHandle(self.job); self.job = None

    @property
    def pid(self): return self.proc.pid

    def poll(self): return self.proc.poll()

    def wait(self, timeout=None): return self.proc.wait(timeout=timeout)

    def _drain_job(self, timeout, t):
        """Wait until the job reports no active process; returns the last count observed."""
        active = job_active_processes(self.job)
        while active and time.time() - t < timeout:
            time.sleep(0.05); active = job_active_processes(self.job)
        return active

    def wait_tree(self, timeout=CLEANUP_TIMEOUT_S):
        """After the immediate child exited: confirm no owned descendant is still running. Lingering descendants are terminated
        (bounded); the report says whether emptiness was confirmed. Callers must not advance on `confirmed: False`."""
        t = time.time(); report = {'pid': self.proc.pid, 'returncode': self.proc.returncode, 'confirmed': False}
        if self.job is not None:
            active = self._drain_job(0.5, t)   # job accounting lags the child's own exit by a moment: short grace before "lingering"
            if active:
                report['lingering_descendants'] = active; _k32.TerminateJobObject(self.job, 1)
                active = self._drain_job(timeout, t)
                report['lingering_terminated'] = active == 0
            report['job_active_processes'] = active; report['confirmed'] = active == 0
        else:
            report.update({'descendants': 'unverified', 'confirmed': False, 'note': 'no job object on this platform'})
        report['seconds'] = round(time.time() - t, 3)
        if report['confirmed']: self.confirmed_empty = True
        return report

    def terminate_tree(self, timeout=CLEANUP_TIMEOUT_S):
        """Kill the owned tree and wait for it to be gone. `confirmed` is True only when the immediate child has exited AND the
        job reports zero active processes within `timeout` (Windows); POSIX cannot confirm descendants."""
        t = time.time(); report = {'pid': self.proc.pid, **self.ownership, 'timeout_s': timeout, 'confirmed': False}
        try:
            if self.job is not None: _k32.TerminateJobObject(self.job, 1)
            elif os.name != 'nt':
                try: os.killpg(os.getpgid(self.proc.pid), signal.SIGKILL)
                except ProcessLookupError: pass
            if self.proc.poll() is None:
                try: self.proc.kill()
                except OSError: pass
            try: self.proc.wait(timeout=timeout); report['child_exited'] = True; report['returncode'] = self.proc.returncode
            except subprocess.TimeoutExpired: report['child_exited'] = False
            if self.job is not None:
                active = self._drain_job(timeout, t)
                report['job_active_processes'] = active
                report['confirmed'] = bool(report.get('child_exited') and active == 0)
            else:
                report['confirmed'] = False; report['descendants'] = 'unverified'
        except OSError as exc:
            report['error'] = f'{type(exc).__name__}: {exc}'
        report['seconds'] = round(time.time() - t, 3)
        if report['confirmed']: self.confirmed_empty = True
        return report

    def release(self, timeout=CLEANUP_TIMEOUT_S):
        """Unconditional finalisation of the owned tree; idempotent. Unless emptiness was already MEASURED (`confirmed_empty`), the
        tree is terminated and bounded confirmation attempted (`terminated: True`); an exited immediate child alone proves nothing
        about descendants. Every owned handle is closed on every path - including an interrupt inside the confirmation wait, when
        kill-on-close still terminates whatever is owned - so an open job can never outlive this call. The report says whether
        emptiness was confirmed; callers must not claim more than it does."""
        if self.release_report is not None: return self.release_report
        try:
            if self.confirmed_empty:
                report = {'pid': self.proc.pid, 'returncode': self.proc.returncode, 'confirmed': True, 'terminated': False,
                          'note': 'owned tree already confirmed empty'}
            else:
                report = self.terminate_tree(timeout); report['terminated'] = True
        finally:
            self.close()
        report['handles_closed'] = True
        self.release_report = report
        return report

    def close(self):
        if self.job is not None:
            _k32.CloseHandle(self.job); self.job = None


def cancel_requested(directory):
    return (Path(directory) / CANCEL_FILE).exists()
