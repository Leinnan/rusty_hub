"""Check release PE resources and MSI tables without installing anything."""
import ctypes
import pathlib
import sys
import xml.etree.ElementTree as ET


def verify_exe(path):
    # Inspect the import table without loading the executable's dependencies.
    import struct
    pe = path.read_bytes()
    pe_offset = struct.unpack_from("<I", pe, 0x3C)[0]
    assert struct.unpack_from("<H", pe, pe_offset + 4)[0] == 0x8664, "Expected an x64 executable"
    section_count = struct.unpack_from("<H", pe, pe_offset + 6)[0]
    optional_size = struct.unpack_from("<H", pe, pe_offset + 20)[0]
    optional = pe_offset + 24
    assert struct.unpack_from("<H", pe, optional)[0] == 0x20B, "Expected an x64 PE"
    imports_rva = struct.unpack_from("<I", pe, optional + 112 + 8)[0]
    sections = optional + optional_size

    def file_offset(rva):
        for i in range(section_count):
            section = sections + 40 * i
            virtual_size, address, raw_size, raw_offset = struct.unpack_from("<4I", pe, section + 8)
            if address <= rva < address + max(virtual_size, raw_size):
                return raw_offset + rva - address
        raise ValueError(f"Unmapped RVA {rva:#x}")

    imports = []
    descriptor = file_offset(imports_rva)
    while True:
        fields = struct.unpack_from("<5I", pe, descriptor)
        if not any(fields):
            break
        name = file_offset(fields[3])
        imports.append(pe[name:pe.index(b"\0", name)].decode("ascii").lower())
        descriptor += 20
    assert not any(name.startswith(("vcruntime", "msvcp", "msvcr")) for name in imports), "Visual C++ runtime must be linked statically"
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.LoadLibraryExW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.c_uint]
    kernel.LoadLibraryExW.restype = ctypes.c_void_p
    kernel.FindResourceW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
    kernel.FindResourceW.restype = ctypes.c_void_p
    kernel.SizeofResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    kernel.SizeofResource.restype = ctypes.c_uint
    kernel.LoadResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    kernel.LoadResource.restype = ctypes.c_void_p
    kernel.LockResource.argtypes = [ctypes.c_void_p]
    kernel.LockResource.restype = ctypes.c_void_p
    kernel.FreeLibrary.argtypes = [ctypes.c_void_p]
    module = kernel.LoadLibraryExW(str(path.resolve()), None, 2)
    if not module:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        for kind, name in [(14, "icon"), (16, "version"), (24, "manifest")]:
            resource = kernel.FindResourceW(module, 1, kind)
            if not resource:
                raise RuntimeError(f"Missing {name} resource")
            if kind == 24:
                size = kernel.SizeofResource(module, resource)
                data = kernel.LockResource(kernel.LoadResource(module, resource))
                manifest = ET.fromstring(ctypes.string_at(data, size).rstrip(b"\0"))
                identities = manifest.findall(".//{urn:schemas-microsoft-com:asm.v1}assemblyIdentity")
                assert any(i.get("name") == "Microsoft.Windows.Common-Controls" and i.get("version") == "6.0.0.0" for i in identities)
                level = manifest.find(".//{urn:schemas-microsoft-com:asm.v3}requestedExecutionLevel")
                assert level is not None and level.get("level") == "asInvoker"
    finally:
        kernel.FreeLibrary(module)


def verify_msi(path):
    msi = ctypes.WinDLL("msi", use_last_error=True)
    handle_type = ctypes.c_uint
    msi.MsiOpenDatabaseW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.POINTER(handle_type)]
    msi.MsiDatabaseOpenViewW.argtypes = [handle_type, ctypes.c_wchar_p, ctypes.POINTER(handle_type)]
    msi.MsiViewExecute.argtypes = [handle_type, handle_type]
    msi.MsiViewFetch.argtypes = [handle_type, ctypes.POINTER(handle_type)]
    msi.MsiRecordGetStringW.argtypes = [handle_type, ctypes.c_uint, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_uint)]
    msi.MsiCloseHandle.argtypes = [handle_type]

    def check(code):
        if code:
            raise OSError(code, "Windows Installer database query failed")

    database = handle_type()
    check(msi.MsiOpenDatabaseW(str(path.resolve()), None, ctypes.byref(database)))

    def rows(query, columns):
        view = handle_type()
        check(msi.MsiDatabaseOpenViewW(database, query, ctypes.byref(view)))
        result = []
        try:
            check(msi.MsiViewExecute(view, 0))
            while True:
                record = handle_type()
                status = msi.MsiViewFetch(view, ctypes.byref(record))
                if status == 259:  # ERROR_NO_MORE_ITEMS
                    break
                check(status)
                try:
                    values = []
                    for column in range(1, columns + 1):
                        buffer = ctypes.create_unicode_buffer(32768)
                        size = ctypes.c_uint(len(buffer))
                        check(msi.MsiRecordGetStringW(record, column, buffer, ctypes.byref(size)))
                        values.append(buffer.value)
                    result.append(tuple(values))
                finally:
                    msi.MsiCloseHandle(record)
        finally:
            msi.MsiCloseHandle(view)
        return result

    properties = dict(rows("SELECT `Property`, `Value` FROM `Property`", 2))
    assert properties.get("ALLUSERS", "") == "", "MSI must be per-user"
    assert properties["UpgradeCode"] == "{2FB4AA5B-B8FB-4F57-AFAF-48412C048F13}"
    registry = rows("SELECT `Root`, `Key`, `Name` FROM `Registry`", 3)
    assert registry and all(root == "1" for root, _, _ in registry), "Only HKCU writes are permitted"
    assert not any(key.startswith("Software\\Classes") for _, key, _ in registry), "Must not claim file types"
    assert any(name == "BinaryInstalled" for _, _, name in registry)
    actions = dict(rows("SELECT `Action`, `Target` FROM `CustomAction`", 2))
    assert "Close Rusty Hub" in actions["RequireClosedApp"]
    close_apps = rows("SELECT `Target`, `Attributes`, `Property` FROM `Wix4CloseApplication`", 3)
    assert close_apps == [("rusty_hub.exe", "0", "RUSTY_HUB_RUNNING")], close_apps
    assert rows("SELECT `Shortcut` FROM `Shortcut`", 1), "Start menu shortcut missing"
    msi.MsiCloseHandle(database)


if __name__ == "__main__":
    exe, msi = map(pathlib.Path, sys.argv[1:])
    verify_exe(exe)
    verify_msi(msi)
    print("Verified executable resources and per-user MSI tables.")
