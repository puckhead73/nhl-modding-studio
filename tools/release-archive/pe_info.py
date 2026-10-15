"""Read what a Windows executable needs at run time (receipt 46): its imported DLLs, whether it embeds a
manifest resource, and its target subsystem version. Standard library only; the file is read, never run.

    python tools/release-archive/pe_info.py <file.exe> [...]
"""
import json
import struct
import sys


class Pe:
    def __init__(self, path):
        with open(path, "rb") as source:
            self.b = source.read()
        b = self.b
        if b[:2] != b"MZ":
            raise ValueError(f"{path}: not an MZ executable")
        pe = struct.unpack_from("<I", b, 0x3C)[0]
        if b[pe:pe + 4] != b"PE\0\0":
            raise ValueError(f"{path}: no PE signature")
        self.machine, nsec, _, _, _, optsz, _ = struct.unpack_from("<HHIIIHH", b, pe + 4)
        opt = pe + 24
        magic = struct.unpack_from("<H", b, opt)[0]
        if magic != 0x20B:
            raise ValueError(f"{path}: not PE32+ (magic {magic:#x})")
        self.subsystem_version = struct.unpack_from("<HH", b, opt + 48)
        self.subsystem = struct.unpack_from("<H", b, opt + 68)[0]
        ndirs = struct.unpack_from("<I", b, opt + 108)[0]
        self.dirs = [struct.unpack_from("<II", b, opt + 112 + 8 * i) for i in range(ndirs)]
        self.sections = []
        s = opt + optsz
        for i in range(nsec):
            name, vsize, va, rsize, rptr = struct.unpack_from("<8sIIII", b, s + 40 * i)
            self.sections.append((name.rstrip(b"\0").decode("ascii", "replace"), va, max(vsize, rsize), rptr, rsize))

    def off(self, rva):
        for _, va, size, rptr, rsize in self.sections:
            if va <= rva < va + size:
                if rva - va >= rsize:
                    raise ValueError(f"rva {rva:#x} outside the raw data of its section")
                return rptr + rva - va
        raise ValueError(f"rva {rva:#x} in no section")

    def cstr(self, rva):
        o = self.off(rva)
        return self.b[o:self.b.index(b"\0", o)].decode("ascii")

    def imports(self):
        out = []
        rva, size = self.dirs[1] if len(self.dirs) > 1 else (0, 0)
        if rva:
            o = self.off(rva)
            while True:
                name_rva = struct.unpack_from("<I", self.b, o + 12)[0]
                if not name_rva:
                    break
                out.append(self.cstr(name_rva))
                o += 20
        rva, size = self.dirs[13] if len(self.dirs) > 13 else (0, 0)
        delay = []
        if rva:
            o = self.off(rva)
            while True:
                name_rva = struct.unpack_from("<I", self.b, o + 4)[0]
                if not name_rva:
                    break
                delay.append(self.cstr(name_rva))
                o += 32
        return out, delay

    def resource_types(self):
        rva, _ = self.dirs[2] if len(self.dirs) > 2 else (0, 0)
        if not rva:
            return []
        o = self.off(rva)
        named, ids = struct.unpack_from("<HH", self.b, o + 12)
        types = []
        for i in range(named + ids):
            name, _ = struct.unpack_from("<II", self.b, o + 16 + 8 * i)
            if not name & 0x80000000:
                types.append(name)
        return types


def info(path):
    p = Pe(path)
    imports, delay = p.imports()
    return {
        "file": path,
        "subsystem": {2: "windows (GUI)", 3: "console"}.get(p.subsystem, p.subsystem),
        "subsystem_version": "%d.%d" % p.subsystem_version,
        "imports": imports,
        "delay_imports": delay,
        "manifest_resource": 24 in p.resource_types(),
    }


# Windows system DLLs: present on every supported Windows (the Universal CRT is part of Windows 10 / 11).
SYSTEM_PREFIXES = ("api-ms-win-", "ext-ms-win-")
SYSTEM_DLLS = {d.lower() for d in (
    "kernel32.dll", "user32.dll", "gdi32.dll", "advapi32.dll", "shell32.dll", "ole32.dll", "oleaut32.dll", "comctl32.dll",
    "comdlg32.dll", "shlwapi.dll", "ws2_32.dll", "bcrypt.dll", "bcryptprimitives.dll", "crypt32.dll", "ntdll.dll",
    "userenv.dll", "uxtheme.dll", "dwmapi.dll", "version.dll", "winmm.dll", "secur32.dll", "ncrypt.dll", "iphlpapi.dll",
    "propsys.dll", "setupapi.dll", "dbghelp.dll", "imm32.dll", "msimg32.dll", "powrprof.dll", "psapi.dll", "rpcrt4.dll",
    "shcore.dll", "winspool.drv", "wininet.dll", "winhttp.dll", "d3d11.dll", "dxgi.dll", "d2d1.dll", "dwrite.dll",
    "windowscodecs.dll", "urlmon.dll", "oleacc.dll", "msvcrt.dll", "normaliz.dll", "cfgmgr32.dll", "hid.dll",
    "wtsapi32.dll", "dnsapi.dll", "mpr.dll", "netapi32.dll", "mswsock.dll", "credui.dll", "wldap32.dll", "combase.dll",
    "api-ms-win-core-synch-l1-2-0.dll",
)}


def non_system(dlls):
    return sorted(d for d in dlls if not d.lower().startswith(SYSTEM_PREFIXES) and d.lower() not in SYSTEM_DLLS)


if __name__ == "__main__":
    for f in sys.argv[1:]:
        i = info(f)
        i["non_system_imports"] = non_system(i["imports"] + i["delay_imports"])
        print(json.dumps(i, indent=2))
