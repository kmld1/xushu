"""Phase 7A installed-build E2E orchestration.

Installs the isolated NSIS package into a directory containing spaces and
Chinese characters, drives the installed application through WebView2 CDP,
verifies data persistence/upgrade/sidecar/uninstall behavior, and checks that
the normal user database is never touched.

The installer passed here must already be built with the isolated identifier
`dev.xushu.e2e.phase7a` and `appDirectoriesOverride` pointing at --data-dir.
"""

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import lzma
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / "target" / "phase7a"
SCENARIO = ROOT / "scripts" / "phase7a-installed-e2e.mjs"
NORMAL_IDENTIFIER = "dev.xushu.desktop"
ISOLATED_IDENTIFIER = "dev.xushu.e2e.phase7a"
PRODUCT_NAME = "XUSHU E2E Phase 7A"
CDP_PORT = 9333
LOG_CANARIES = (
    "XUSHU_AUDIT_SYNTHETIC_PROMPT_7A1",
    "XUSHU_AUDIT_SYNTHETIC_TRANSCRIPT_7A1",
    "XUSHU_AUDIT_SYNTHETIC_TOOL_INPUT_7A1",
    "XUSHU_AUDIT_SYNTHETIC_TOOL_RESPONSE_7A1",
    "XUSHU_AUDIT_SYNTHETIC_TOKEN_7A1",
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def fingerprint(database):
    result = []
    for suffix in ("", "-wal", "-shm"):
        path = database.with_name(database.name + suffix)
        try:
            result.append(digest(path))
        except FileNotFoundError:
            result.append(None)
    return result


def ensure_port_free(port):
    for _ in range(100):
        with socket.socket() as listener:
            try:
                listener.bind(("127.0.0.1", port))
                return
            except OSError:
                time.sleep(0.1)
    raise RuntimeError(f"CDP port {port} is still in use")


def require_fresh_directory(path):
    require(WORK in path.resolve().parents, f"fixture must be under {WORK}")
    require(not path.exists(), f"fixture already exists: {path}")


def require_isolated_installer(installer, data_dir, product_name, version):
    # The Tauri x64 NSIS packages use a solid LZMA stream. Inspect the
    # embedded config before any installer process or fixture mutation.
    require(installer.stat().st_size <= 16 * 1024 * 1024, "unexpected installer size")
    contents = installer.read_bytes()
    marker = bytes.fromhex("efbeadde4e756c6c736f6674496e7374")
    offset = contents.find(marker)
    require(contents.startswith(b"MZ") and offset >= 4, "expected an NSIS executable")
    offset += 24
    require(contents[offset] == 0x5D, "expected the Tauri NSIS solid LZMA format")
    dictionary_size = struct.unpack_from("<I", contents, offset + 1)[0]
    require(0 < dictionary_size <= 16 * 1024 * 1024, "unsupported LZMA dictionary size")
    decoder = lzma.LZMADecompressor(
        format=lzma.FORMAT_RAW,
        filters=[{"id": lzma.FILTER_LZMA1, "dict_size": dictionary_size, "lc": 3, "lp": 0, "pb": 2}],
    )
    payload = decoder.decompress(contents[offset + 5 :], max_length=32 * 1024 * 1024)
    require(decoder.eof, "cannot read the complete installer payload")
    config = f"{product_name}{version}{ISOLATED_IDENTIFIER}main".encode("utf-8")
    position = 0
    matches = []
    while position < len(payload):
        size = struct.unpack_from("<I", payload, position)[0]
        start = position + 4
        position = start + size
        require(position <= len(payload), "incomplete installer payload entry")
        entry = payload[start:position]
        if entry.startswith(b"MZ") and config in entry:
            require(str(data_dir).encode("utf-8") in entry, "embedded E2E data directory differs")
            matches.append(hashlib.sha256(entry).hexdigest())
    require(len(matches) == 1, "installer lacks the expected isolated application payload")
    return matches[0]


def run(args, **kwargs):
    completed = subprocess.run(args, **kwargs)
    require(completed.returncode == 0, f"command failed ({completed.returncode}): {args}")
    return completed


def node_scenario(port, mode, repo, git_executable):
    run(
        [
            "node",
            str(SCENARIO),
            str(port),
            mode,
            str(repo),
            git_executable,
            str(WORK),
        ],
        cwd=ROOT,
        timeout=420,
    )


def find_main_executable(install_dir):
    candidates = [
        path
        for path in install_dir.glob("*.exe")
        if "uninstall" not in path.name.lower() and "xushu-hook" not in path.name.lower()
    ]
    require(len(candidates) == 1, f"expected one main executable in {install_dir}, found {candidates}")
    return candidates[0]


def find_uninstaller(install_dir):
    candidates = [path for path in install_dir.glob("*.exe") if "uninstall" in path.name.lower()]
    require(candidates, f"uninstaller missing in {install_dir}")
    return candidates[0]


def launch(exe, data_dir, port):
    ensure_port_free(port)
    data_dir.mkdir(parents=True, exist_ok=True)
    env = {
        **os.environ,
        "XUSHU_E2E_DATA_DIR": str(data_dir),
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS": f"--remote-debugging-port={port}",
    }
    child = subprocess.Popen(
        [str(exe)],
        cwd=exe.parent,
        env=env,
        creationflags=subprocess.CREATE_NO_WINDOW,
    )
    try:
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.time() + 60
        cdp_ready = False
        while time.time() < deadline:
            require(child.poll() is None, "installed application exited before startup")
            if not cdp_ready:
                try:
                    with opener.open(f"http://127.0.0.1:{port}/json/list", timeout=0.5) as response:
                        cdp_ready = response.status == 200
                except OSError:
                    pass
            data_ready = (
                (data_dir / "xushu.db").is_file()
                and (data_dir / "hook-inbox").is_dir()
                and (data_dir / "EBWebView").is_dir()
            )
            if cdp_ready and data_ready:
                return child
            time.sleep(0.1)
        raise RuntimeError("installed application did not become ready with its isolated data directory")
    except BaseException:
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=10)
        raise


def wait_exit(child, timeout=30):
    try:
        code = child.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        child.terminate()
        child.wait(timeout=10)
        raise RuntimeError("installed application did not exit after window close")
    require(code == 0, f"installed application exited with {code}")


def user32_api():
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowThreadProcessId.restype = wintypes.DWORD
    user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    user32.PostMessageW.restype = wintypes.BOOL
    user32.EnumWindows.argtypes = [ctypes.c_void_p, wintypes.LPARAM]
    user32.EnumWindows.restype = wintypes.BOOL
    user32.EnumChildWindows.argtypes = [wintypes.HWND, ctypes.c_void_p, wintypes.LPARAM]
    user32.EnumChildWindows.restype = wintypes.BOOL
    user32.IsWindowVisible.argtypes = [wintypes.HWND]
    user32.IsWindowVisible.restype = wintypes.BOOL
    user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.GetClassNameW.restype = ctypes.c_int
    user32.SetWindowPos.argtypes = [
        wintypes.HWND,
        wintypes.HWND,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        wintypes.UINT,
    ]
    user32.SetWindowPos.restype = wintypes.BOOL
    user32.SendMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    user32.SendMessageW.restype = ctypes.c_ssize_t
    user32.GetDpiForWindow.argtypes = [wintypes.HWND]
    user32.GetDpiForWindow.restype = wintypes.UINT
    user32.GetSystemMetrics.argtypes = [ctypes.c_int]
    user32.GetSystemMetrics.restype = ctypes.c_int
    user32.IsIconic.argtypes = [wintypes.HWND]
    user32.IsIconic.restype = wintypes.BOOL
    user32.ShowWindow.argtypes = [wintypes.HWND, ctypes.c_int]
    user32.ShowWindow.restype = wintypes.BOOL
    return user32


def close_window(pid):
    user32 = user32_api()
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    @callback_type
    def visit(window, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(window, ctypes.byref(owner))
        if owner.value == pid:
            user32.PostMessageW(window, 0x0010, 0, 0)
        return True

    user32.EnumWindows(visit, 0)


def find_window(pid):
    user32 = user32_api()
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    found = []

    @callback_type
    def visit(window, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(window, ctypes.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(window):
            found.append(window)
        return True

    user32.EnumWindows(visit, 0)
    require(found, f"no visible window for pid {pid}")
    return found[0]


def find_overlay(main_window):
    user32 = user32_api()
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    found = []

    @callback_type
    def visit(window, _):
        buffer = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(window, buffer, 256)
        if buffer.value == "XushuSnapOverlay":
            found.append(window)
        return True

    for _ in range(50):
        user32.EnumChildWindows(main_window, visit, 0)
        if found:
            return found[0]
        time.sleep(0.1)
    raise RuntimeError("Snap overlay child window was not created")


def window_probe(exe, port, repo, git_executable):
    child = launch(exe, DATA, port)
    try:
        hwnd = find_window(child.pid)
        user32 = user32_api()
        user32.SetWindowPos(hwnd, 0, 40, 40, 500, 480, 0x0004)
        time.sleep(0.8)

        overlay = find_overlay(hwnd)
        hit_test = user32.SendMessageW(overlay, 0x0084, 0, 0)  # WM_NCHITTEST
        require(hit_test == 9, f"Snap overlay did not return HTMAXBUTTON: {hit_test}")

        dpi = user32.GetDpiForWindow(hwnd)
        monitor_count = user32.GetSystemMetrics(80)  # SM_CMONITORS

        node_scenario(port, "window", repo, git_executable)
        time.sleep(0.5)
        require(user32.IsIconic(hwnd), "minimize did not minimize the installed window")
        user32.ShowWindow(hwnd, 9)  # SW_RESTORE
        time.sleep(0.5)
        node_scenario(port, "close", repo, git_executable)
        wait_exit(child)
        return {"dpi": dpi, "monitorCount": monitor_count, "snapHitTest": hit_test}
    finally:
        if child.poll() is None:
            close_window(child.pid)
            child.wait(timeout=10)


def install(installer, install_dir, clean, expected_digest):
    install_dir.parent.mkdir(parents=True, exist_ok=True)
    if clean:
        require_fresh_directory(install_dir)
    else:
        require(install_dir.is_dir(), "upgrade requested without an existing installation")
    # NSIS /D must be the final argument and must not be quoted. Passing a
    # list makes subprocess quote the path because it contains spaces, which
    # NSIS would treat literally; pass the raw command line instead.
    run(f'"{installer}" /S /D={install_dir}', timeout=240)
    deadline = time.time() + 60
    while time.time() < deadline:
        exes = list(install_dir.glob("*.exe")) if install_dir.exists() else []
        if exes:
            break
        time.sleep(0.2)
    require(install_dir.is_dir(), "installer did not create the requested install directory")
    exe = find_main_executable(install_dir)
    require(
        digest(exe) == expected_digest,
        "installed main executable differs from the checked isolated payload",
    )
    return exe


def uninstall(install_dir):
    uninstaller = find_uninstaller(install_dir)
    run([str(uninstaller), "/S"], timeout=240)
    deadline = time.time() + 60
    while time.time() < deadline and install_dir.exists():
        remaining = list(install_dir.glob("*")) if install_dir.exists() else []
        if not remaining:
            break
        time.sleep(0.25)
    require(not install_dir.exists() or not any(install_dir.glob("*")), "uninstaller left program files behind")


def file_version(exe):
    escaped = str(exe).replace("'", "''")
    command = (
        f"$v=(Get-Item -LiteralPath '{escaped}').VersionInfo; "
        "Write-Output ($v.ProductVersion + '|' + $v.FileVersion)"
    )
    completed = run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", command],
        capture_output=True,
        text=True,
        timeout=60,
    )
    return completed.stdout.strip()


def read_settings(fixture):
    path = fixture / ".claude" / "settings.local.json"
    require(path.is_file(), "installed hooks did not write .claude/settings.local.json")
    document = json.loads(path.read_text(encoding="utf-8"))
    for groups in document.get("hooks", {}).values():
        for group in groups:
            for handler in group.get("hooks", []):
                if handler.get("args", [None])[0] == "--xushu-managed":
                    return handler
    raise RuntimeError("managed hook handler missing from settings.local.json")


def run_sidecar(handler, fixture, session_id):
    payload = json.dumps(
        {
            "session_id": session_id,
            "transcript_path": str(fixture / f"{LOG_CANARIES[1]}.jsonl"),
            "cwd": str(fixture),
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "prompt": LOG_CANARIES[0],
            "tool_input": {"command": LOG_CANARIES[2], "api_key": LOG_CANARIES[4]},
            "tool_response": LOG_CANARIES[3],
            "tool_use_id": f"toolu-phase7a-{session_id}",
            "prompt_id": f"prompt-phase7a-{session_id}",
        }
    )
    run(
        [handler["command"], *handler["args"]],
        input=payload.encode("utf-8"),
        timeout=60,
    )


def read_log():
    path = DATA / "logs" / "xushu.log"
    require(path.is_file(), "bounded diagnostic log was not created")
    text = path.read_text(encoding="utf-8", errors="replace")
    require(len(text.encode("utf-8")) <= 2 * 1024 * 1024, "diagnostic log exceeded its bound")
    return text


def unclean_exit_probe(exe):
    app = launch(exe, DATA, CDP_PORT)
    time.sleep(1.0)
    app.kill()
    app.wait(timeout=10)
    app = launch(exe, DATA, CDP_PORT)
    time.sleep(1.5)
    log = read_log()
    require("previous_run_unclean_exit" in log, "unclean exit was not detected on restart")
    node_scenario(CDP_PORT, "close", DATA / "fixture project" / "repo", shutil.which("git"))
    wait_exit(app)
    return {"previousRunDetected": True}


def install_evidence(path):
    return {
        "path": str(path),
        "sha256": digest(path),
        "size": path.stat().st_size,
    }


def main():
    global DATA, WORK
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installer", type=Path, required=True)
    parser.add_argument("--upgrade-installer", type=Path)
    parser.add_argument("--work-dir", type=Path, default=WORK)
    parser.add_argument("--product-name", default=PRODUCT_NAME)
    parser.add_argument("--install-dir", type=Path)
    parser.add_argument("--data-dir", type=Path)
    parser.add_argument("--fixture", type=Path)
    args = parser.parse_args()

    require(os.name == "nt", "Phase 7A installed E2E requires Windows")
    import winreg

    WORK = args.work_dir.resolve()
    require((ROOT / "target").resolve() in WORK.parents, "work directory must be under target")
    DATA = (args.data_dir or WORK / "installed-data").resolve()
    install_dir = (args.install_dir or WORK / "install 目录" / "序枢 XUSHU").resolve()
    fixture = (args.fixture or WORK / "fixture project" / "repo").resolve()
    relocated = WORK / "relocated 安装 目录" / "序枢 XUSHU 副本"
    require(args.product_name.startswith(PRODUCT_NAME), "expected a dedicated E2E product name")
    for directory in (DATA, install_dir, fixture.parent, relocated):
        require_fresh_directory(directory)
    installer = args.installer.resolve()
    v1_digest = require_isolated_installer(installer, DATA, args.product_name, "0.1.0")
    if args.upgrade_installer:
        upgrade_installer = args.upgrade_installer.resolve()
        v2_digest = require_isolated_installer(upgrade_installer, DATA, args.product_name, "0.1.1")

    shortcut = (
        Path(os.environ["APPDATA"]) / "Microsoft" / "Windows" / "Start Menu" / "Programs"
        / f"{args.product_name}.lnk"
    )
    require(not shortcut.exists(), "an E2E Start Menu shortcut already exists for this product")
    uninstall_key = fr"Software\Microsoft\Windows\CurrentVersion\Uninstall\{args.product_name}"
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, uninstall_key):
            raise RuntimeError("an E2E installation already exists for this product")
    except FileNotFoundError:
        pass
    ensure_port_free(CDP_PORT)

    normal_database = Path(os.environ["APPDATA"]) / NORMAL_IDENTIFIER / "xushu.db"
    normal_before = fingerprint(normal_database)

    WORK.mkdir(parents=True, exist_ok=True)
    fixture.parent.mkdir(parents=True)
    run(["git", "init", "--quiet", "--template=", str(fixture)])
    git_executable = shutil.which("git")
    require(git_executable, "git executable not found")

    evidence = {
        "environment": {
            "os": {
                "major": sys.getwindowsversion().major,
                "minor": sys.getwindowsversion().minor,
                "build": sys.getwindowsversion().build,
                "platform": sys.getwindowsversion().platform,
                "servicePack": sys.getwindowsversion().service_pack,
            },
            "python": sys.version,
            "normalDatabase": str(normal_database),
        },
        "installer": install_evidence(args.installer.resolve()),
        "installDirectory": str(install_dir),
        "dataDirectory": str(DATA),
        "fixture": str(fixture),
    }

    main_exe = install(installer, install_dir, clean=True, expected_digest=v1_digest)
    evidence["installedBinary"] = str(main_exe)
    evidence["installedVersion1"] = file_version(main_exe)
    require("0.1.0" in evidence["installedVersion1"], "unexpected version in installed v1 binary")

    port = CDP_PORT
    app = launch(main_exe, DATA, port)
    try:
        node_scenario(port, "full", fixture, git_executable)
        wait_exit(app)
    finally:
        if app.poll() is None:
            close_window(app.pid)
            app.wait(timeout=10)

    handler = read_settings(fixture)
    evidence["hookCommand"] = handler["command"]
    run_sidecar(handler, fixture, "session-one")

    port = CDP_PORT
    app = launch(main_exe, DATA, port)
    try:
        node_scenario(port, "reopen", fixture, git_executable)
        wait_exit(app)
    finally:
        if app.poll() is None:
            close_window(app.pid)
            app.wait(timeout=10)

    reopened = json.loads((WORK / "installed-reopen-result.json").read_text(encoding="utf-8"))
    require(reopened["integration"]["eventCount"] >= 1, "sidecar event missing after reopen")
    log_text = read_log()
    require("app_start version=0.1.0" in log_text, "app_start missing from diagnostic log")
    require("database_ready schema=7" in log_text, "database readiness missing from diagnostic log")
    require("app_exit" in log_text, "normal exit was not recorded in the diagnostic log")
    full_result = json.loads((WORK / "installed-full-result.json").read_text(encoding="utf-8"))
    synthetic_secrets = (*LOG_CANARIES, full_result["preflight"]["token"], str(fixture))
    require(not any(secret in log_text for secret in synthetic_secrets), "synthetic sensitive data reached the log")
    require(
        not any(pattern in log_text for pattern in ("sk-", "ghp_", "AKIA", "BEGIN PRIVATE KEY")),
        "sensitive credential pattern found in diagnostic log",
    )
    evidence["diagnosticLog"] = {"appExitRecorded": True, "syntheticSecretsAbsent": True}
    evidence["uncleanExit"] = unclean_exit_probe(main_exe)
    evidence["firstLaunch"] = {
        "eventCount": reopened["integration"]["eventCount"],
        "taskStatus": reopened["tasks"][0]["status"],
        "memoryTitle": reopened["memories"][0]["memory"]["title"],
    }

    if args.upgrade_installer:
        evidence["upgradeInstaller"] = install_evidence(upgrade_installer)
        main_exe = install(upgrade_installer, install_dir, clean=False, expected_digest=v2_digest)
        evidence["installedVersion2"] = file_version(main_exe)
        require("0.1.1" in evidence["installedVersion2"], "upgrade did not install version 0.1.1")

        port = CDP_PORT
        app = launch(main_exe, DATA, port)
        try:
            node_scenario(port, "upgrade", fixture, git_executable)
            wait_exit(app)
        finally:
            if app.poll() is None:
                close_window(app.pid)
                app.wait(timeout=10)

        handler = read_settings(fixture)
        run_sidecar(handler, fixture, "session-two")

        port = CDP_PORT
        app = launch(main_exe, DATA, port)
        try:
            node_scenario(port, "upgrade-reopen", fixture, git_executable)
            wait_exit(app)
        finally:
            if app.poll() is None:
                close_window(app.pid)
                app.wait(timeout=10)

        upgraded = json.loads(
            (WORK / "installed-upgrade-reopen-result.json").read_text(encoding="utf-8")
        )
        require(upgraded["integration"]["eventCount"] >= 2, "upgraded sidecar event missing")
        require(upgraded["tasks"][0]["status"] == "done", "upgrade lost task status")
        require(
            upgraded["memories"][0]["memory"]["title"] == "安装版恢复记忆",
            "upgrade lost project memory",
        )
        evidence["upgrade"] = {
            "eventCount": upgraded["integration"]["eventCount"],
            "taskStatus": upgraded["tasks"][0]["status"],
            "memoryTitle": upgraded["memories"][0]["memory"]["title"],
            "hookState": upgraded["integration"]["installState"],
            "receiverChanged": upgraded["integration"]["receiverChanged"],
        }

    shutil.copytree(install_dir, relocated)
    relocated_exe = find_main_executable(relocated)
    sidecar = next(relocated.glob("xushu-hook*.exe"))
    require(sidecar.is_file(), "sidecar missing from installed directory")

    evidence["sidecar"] = {"installedPath": str(sidecar), "sha256": digest(sidecar)}
    evidence["window"] = window_probe(relocated_exe, CDP_PORT, fixture, git_executable)

    saved_sidecar = sidecar.with_suffix(".saved")
    sidecar.rename(saved_sidecar)
    try:
        port = CDP_PORT
        app = launch(relocated_exe, DATA, port)
        try:
            node_scenario(port, "sidecar-unavailable", fixture, git_executable)
            wait_exit(app)
        finally:
            if app.poll() is None:
                close_window(app.pid)
                app.wait(timeout=10)
        evidence["sidecarMissing"] = "receiver_unavailable"
    finally:
        saved_sidecar.rename(sidecar)

    sidecar.write_bytes(b"not a valid executable")
    try:
        port = CDP_PORT
        app = launch(relocated_exe, DATA, port)
        try:
            node_scenario(port, "sidecar-unavailable", fixture, git_executable)
            wait_exit(app)
        finally:
            if app.poll() is None:
                close_window(app.pid)
                app.wait(timeout=10)
        evidence["sidecarCorrupt"] = "receiver_unavailable"
    finally:
        sidecar.unlink()
        shutil.copy2(install_dir / sidecar.name, sidecar)

    fake_source = WORK / "fake-hook.rs"
    fake_source.write_text(
        'fn main() { if std::env::args().any(|arg| arg == "--version") { println!("xushu-hook 9.9.9"); } }\n',
        encoding="utf-8",
    )
    fake_binary = WORK / "fake-hook.exe"
    run(["rustc", str(fake_source), "-O", "-o", str(fake_binary)], timeout=180)
    sidecar.unlink()
    shutil.copy2(fake_binary, sidecar)
    try:
        port = CDP_PORT
        app = launch(relocated_exe, DATA, port)
        try:
            node_scenario(port, "sidecar-unavailable", fixture, git_executable)
            wait_exit(app)
        finally:
            if app.poll() is None:
                close_window(app.pid)
                app.wait(timeout=10)
        evidence["sidecarVersionMismatch"] = "receiver_unavailable"
    finally:
        sidecar.unlink()
        shutil.copy2(install_dir / sidecar.name, sidecar)

    evidence["startMenuShortcutBeforeUninstall"] = shortcut.is_file()
    require(shortcut.is_file(), "Start Menu shortcut was not created by the installer")

    data_before_uninstall = sorted(str(path.relative_to(DATA)) for path in DATA.rglob("*"))
    database_before_uninstall = fingerprint(DATA / "xushu.db")
    backup_root = fixture.parent / "phase7a-backups"
    backup_files = {path: digest(path) for path in backup_root.rglob("*") if path.is_file()}
    require(backup_files, "the isolated run must create backup files before uninstall")
    fixture_settings = (fixture / ".claude" / "settings.local.json").read_text(encoding="utf-8")
    uninstall(install_dir)
    require(not install_dir.exists() or not any(install_dir.glob("*")), "install directory remains")
    require(not shortcut.exists(), "uninstaller left the Start Menu shortcut")
    require(DATA.is_dir(), "uninstaller removed the user data directory")
    require((DATA / "xushu.db").is_file(), "uninstaller removed the user database")
    require(fingerprint(DATA / "xushu.db") == database_before_uninstall, "uninstall modified the fixture database")
    require(
        {path: digest(path) for path in backup_root.rglob("*") if path.is_file()} == backup_files,
        "uninstall modified backup files",
    )
    require(fixture.is_dir(), "uninstaller removed the user project")
    require(
        (fixture / ".claude" / "settings.local.json").read_text(encoding="utf-8")
        == fixture_settings,
        "uninstaller modified user Claude settings",
    )
    evidence["uninstall"] = {
        "installDirectoryRemoved": True,
        "shortcutRemoved": True,
        "dataDirectoryPreserved": True,
        "databasePreserved": True,
        "backupFilesPreserved": True,
        "projectPreserved": True,
        "claudeSettingsPreserved": True,
        "dataEntriesBefore": len(data_before_uninstall),
    }

    normal_after = fingerprint(normal_database)
    require(normal_after == normal_before, "normal user database changed during installed E2E")
    evidence["normalDatabaseUnchanged"] = True

    (WORK / "installed-e2e-report.json").write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print("PHASE7A_INSTALLED_E2E_PASS", json.dumps(evidence, ensure_ascii=False))


if __name__ == "__main__":
    main()
