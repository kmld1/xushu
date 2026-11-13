"""Run Windows desktop E2E with fresh, disposable Tauri app directories."""

import argparse
from contextlib import closing, contextmanager
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
DESKTOP = ROOT / 'apps' / 'desktop'
BUILD = ROOT / 'target' / 'desktop-e2e-build'


def fingerprint(database):
    result = []
    for suffix in ('', '-wal', '-shm'):
        path = database.with_name(database.name + suffix)
        try:
            with path.open('rb') as file:
                result.append(hashlib.file_digest(file, 'sha256').digest())
        except FileNotFoundError:
            result.append(None)
    return result


def free_port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def close_window(pid):
    user32 = ctypes.WinDLL('user32')
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    @callback_type
    def visit(window, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(window, ctypes.byref(owner))
        if owner.value == pid:
            user32.PostMessageW(window, 0x0010, 0, 0)  # WM_CLOSE
        return True

    user32.EnumWindows(visit, 0)


@contextmanager
def process(args, cwd, *, desktop_data=None):
    env = None
    if desktop_data is not None:
        env = {**os.environ, 'XUSHU_E2E_DATA_DIR': str(desktop_data)}
    child = subprocess.Popen(args, cwd=cwd, env=env, creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        yield child
    finally:
        if desktop_data is not None and child.poll() is None:
            close_window(child.pid)
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
        if child.poll() is None:
            child.terminate()
        child.wait(timeout=10)


@contextmanager
def exclusive_build():
    import msvcrt

    BUILD.mkdir(exist_ok=True)
    require(BUILD.resolve() == BUILD, 'E2E build cache must be inside target')
    with (BUILD / 'run.lock').open('a+b') as lock:
        try:
            msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
        except OSError as error:
            raise RuntimeError('another desktop E2E run owns the build cache') from error
        yield


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def copy_snapshot(snapshot, database):
    # Copy DB and WAL before opening SQLite so the original SHM is untouched.
    before = fingerprint(snapshot)
    database.parent.mkdir()
    shutil.copyfile(snapshot, database)
    if before[1] is not None:
        shutil.copyfile(snapshot.with_name(snapshot.name + '-wal'), database.with_name('xushu.db-wal'))
    require(
        fingerprint(snapshot)[:2] == before[:2] == fingerprint(database)[:2],
        'evidence database changed while copying',
    )


def run(workspace, production, baseline, snapshot, scenario):
    data = workspace / 'app-data'
    database = data / 'xushu.db'
    port, debug_port = free_port(), free_port()
    require(port != debug_port, 'Vite and CDP need distinct ports')
    url = f'http://127.0.0.1:{port}'
    config = {
        'identifier': f'dev.xushu.e2e.{workspace.name}',
        'build': {'devUrl': url},
        'app': {'appDirectoriesOverride': str(data)},
        'bundle': {'active': False},
    }
    window = json.loads((DESKTOP / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))['app']['windows'][0]
    window.update(
        title='XUSHU — 隔离 E2E',
        additionalBrowserArgs=f'--remote-debugging-port={debug_port}',
    )
    config['app']['windows'] = [window]
    build_env = {**os.environ, 'TAURI_CONFIG': json.dumps(config)}
    subprocess.run(
        ['cargo', 'build', '--locked', '--target-dir', str(BUILD), '-p', 'xushu-desktop', '-p', 'xushu-hook'],
        cwd=ROOT, env=build_env, check=True,
    )

    repo = workspace / 'fixture-repo'
    subprocess.run(['git', 'init', '--quiet', '--template=', str(repo)], check=True)
    vite = DESKTOP / 'node_modules/vite/bin/vite.js'
    binary = BUILD / 'debug/xushu-desktop.exe'
    with process(['node', str(vite), '--host', '127.0.0.1', '--port', str(port), '--strictPort'], DESKTOP) as server:
        if snapshot:
            copy_snapshot(snapshot, database)
        data.mkdir(exist_ok=True)
        for attempt in range(50):
            require(server.poll() is None, 'Vite exited before startup')
            try:
                with urllib.request.urlopen(url, timeout=1):
                    break
            except OSError:
                if attempt == 49:
                    raise
                time.sleep(0.1)
        with process([str(binary)], workspace, desktop_data=data) as desktop:
            for _ in range(200):
                require(desktop.poll() is None and server.poll() is None, 'E2E process exited before startup')
                if database.is_file() and (data / 'hook-inbox').is_dir() and (data / 'EBWebView').is_dir():
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError('isolated SQLite, inbox or WebView profile was not created')
            require(fingerprint(production) == baseline, 'normal database changed at startup')
            subprocess.run(
                ['node', str(scenario), str(debug_port), url, str(repo), str(desktop.pid),
                 'snapshot' if snapshot else 'fresh'],
                cwd=ROOT, check=True, timeout=240,
            )
            require(desktop.wait(timeout=15) == 0, 'desktop did not exit successfully')
    require(not (repo / '.claude').exists(), 'E2E wrote Claude Code settings')
    with closing(sqlite3.connect(database.as_uri() + '?mode=ro', uri=True)) as isolated:
        require(isolated.execute('PRAGMA integrity_check').fetchone() == ('ok',), 'isolated SQLite integrity check failed')
        require(isolated.execute(
            "SELECT count(*) FROM tasks JOIN projects ON projects.id = tasks.project_id "
            "WHERE projects.name = 'Desktop E2E isolation probe' AND tasks.title = 'E2E isolation task'"
        ).fetchone() == (1,), 'fixture project/task missing from the isolated database')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', type=Path, help='Phase 4.1 SQLite backup to copy into the E2E directory')
    parser.add_argument('--scenario', type=Path, default=Path(__file__).with_suffix('.mjs'))
    args = parser.parse_args()
    require(os.name == 'nt', 'this scenario uses Windows WebView2')
    normal_config = json.loads((DESKTOP / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
    production = Path(os.environ['APPDATA']) / normal_config['identifier'] / 'xushu.db'
    snapshot = args.snapshot.resolve(strict=True) if args.snapshot else None
    if snapshot:
        require(not production.exists() or not snapshot.samefile(production), 'use an evidence backup, not the normal database')
    baseline = fingerprint(production)
    target = ROOT / 'target'
    target.mkdir(exist_ok=True)
    require(target.resolve() == target, 'target directory must be inside the workspace')
    with exclusive_build():
        run_and_clean(target, production, baseline, snapshot, args.scenario.resolve(strict=True))


def run_and_clean(target, production, baseline, snapshot, scenario):
    temp = tempfile.TemporaryDirectory(prefix='run-', dir=target)
    workspace = Path(temp.name).resolve(strict=True)
    require(workspace.parent == target, 'temporary directory escaped target')
    try:
        run(workspace, production, baseline, snapshot, scenario)
    finally:
        try:
            require(fingerprint(production) == baseline, 'normal database changed during E2E')
        finally:
            # The WebView browser process can keep its cache open after Tauri exits.
            for attempt in range(60):
                try:
                    temp.cleanup()
                    break
                except OSError as error:
                    if attempt == 59 or error.winerror not in (5, 32, 145):
                        raise
                    time.sleep(0.3)
            require(not workspace.exists(), 'E2E directory was not removed')
    print('PASS: project/task in isolated SQLite, normal database unchanged, run directory removed')


if __name__ == '__main__':
    main()
