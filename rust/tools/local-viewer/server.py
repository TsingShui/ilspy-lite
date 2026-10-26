#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Loopback-only developer viewer. Python stdlib; existing Rust and ILSpy engines."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import difflib
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from urllib.parse import parse_qs, urlsplit

HERE = Path(__file__).resolve().parent
RUST = HERE.parents[1]
REPO = RUST.parent
REPORT = RUST / "report/local-viewer"
MAX_INPUT = 64 * 1024 * 1024
MAX_OUTPUT = 16 * 1024 * 1024


def run(command, timeout=40):
    """Bound execution and capture output on disk instead of an unbounded pipe."""
    started = time.monotonic()
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen([str(v) for v in command], stdout=stdout, stderr=stderr)
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise ValueError("Engine timed out; choose a smaller class.")
        if stdout.tell() > MAX_OUTPUT or stderr.tell() > MAX_OUTPUT:
            raise ValueError("Engine output exceeds the 16 MiB viewer limit; use the CLI directly.")
        stdout.seek(0)
        stderr.seek(0)
        return {
            "text": stdout.read().decode("utf-8", errors="replace"),
            "stderr": stderr.read().decode("utf-8", errors="replace"),
            "exitCode": code, "elapsedMs": round((time.monotonic() - started) * 1000),
        }


def latest10(path):
    values = [p for p in path.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)]
    if not values:
        raise ValueError("An existing .NET 10 SDK/reference pack/runtime is required for the ILSpy adapter.")
    return max(values, key=lambda p: tuple(map(int, p.name.split("."))))


class Viewer:
    def __init__(self, options):
        self.options = options
        self.token = secrets.token_hex(24)
        self.files = {}
        self.catalogs = {}
        self.cache = {}
        self.lock = threading.RLock()
        self.build_lock = threading.Lock()
        self.slots = threading.BoundedSemaphore(4)
        self.cli = RUST / "target/release/ilspy-lite"
        self.dotnet_root = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
        self.dotnet = self.dotnet_root / "dotnet"
        self.bridge = REPORT / "bin/IlSpyBridge.dll"
        self.engine = Path(options.engine).resolve() if options.engine else next((p for p in (
            REPO / "ICSharpCode.Decompiler/bin/Release/netstandard2.0/ICSharpCode.Decompiler.dll",
            RUST / "report/openra/wait/ilspy-comparison/ICSharpCode.Decompiler.dll",
        ) if p.is_file()), None)
        if self.engine is None:
            raise ValueError("No existing ILSpy engine DLL found. Supply --engine; nothing is downloaded.")
        self.prepare_bridge()
        self.build()
        self.version = json.loads(run([self.dotnet, self.bridge, "version"])["text"])["version"]
        self.scan()

    def prepare_bridge(self):
        self.bridge.parent.mkdir(parents=True, exist_ok=True)
        engine = self.bridge.parent / "ICSharpCode.Decompiler.dll"
        if not engine.exists() or engine.stat().st_mtime_ns != self.engine.stat().st_mtime_ns:
            shutil.copy2(self.engine, engine)
        if self.bridge.exists() and self.bridge.stat().st_mtime >= max(
                (HERE / "IlSpyBridge.cs").stat().st_mtime, self.engine.stat().st_mtime):
            return
        sdk = latest10(self.dotnet_root / "sdk")
        refs = sorted((latest10(self.dotnet_root / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
        if not refs:
            raise ValueError("Existing .NET 10 reference pack is missing.")
        response = self.bridge.with_suffix(".rsp")
        response.write_text("\n".join([
            "-nologo", "-target:exe", "-langversion:14", "-warnaserror+", "-warnaserror-:1701", "-deterministic+",
            f'-out:"{self.bridge}"', *[f'-r:"{p}"' for p in refs], f'-r:"{engine}"',
            f'"{HERE / "IlSpyBridge.cs"}"',
        ]) + "\n")
        result = run([self.dotnet, sdk / "Roslyn/bincore/csc.dll", "@" + str(response)], 120)
        # The existing ILSpy build references Metadata 9; the standalone host uses
        # the installed .NET 10 framework. Retain CS1701 as a logged warning.
        (REPORT / "adapter-build.log").write_text(result["text"] + result["stderr"])
        if result["exitCode"]:
            raise ValueError("ILSpy adapter compilation failed:\n" + result["text"] + result["stderr"])
        runtime = latest10(self.dotnet_root / "shared/Microsoft.NETCore.App")
        self.bridge.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
            "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))

    def build(self):
        with self.build_lock:
            changed = max(p.stat().st_mtime_ns for p in (RUST / "src").rglob("*.rs"))
            changed = max(changed, (RUST / "Cargo.toml").stat().st_mtime_ns, (RUST / "Cargo.lock").stat().st_mtime_ns)
            if self.cli.exists() and (self.options.no_build or self.cli.stat().st_mtime_ns >= changed):
                return
            if self.options.no_build:
                raise ValueError("Rust release binary missing. Run cargo build --release --locked --offline.")
            result = run(["cargo", "build", "--manifest-path", RUST / "Cargo.toml", "--release", "--locked", "--offline"], 180)
            if result["exitCode"]:
                raise ValueError("Rust build failed:\n" + result["stderr"])

    def register(self, path, group, name=None):
        path = path.resolve()
        if not path.is_file() or path.stat().st_size > MAX_INPUT:
            return None
        key = hashlib.sha256(str(path).encode()).hexdigest()[:20]
        with self.lock:
            self.files[key] = {"path": path, "id": key, "name": name or path.name,
                               "group": group, "bytes": path.stat().st_size}
        return key

    def scan(self):
        for directory, group, recursive in [
            (RUST / "report/openra/assemblies", "OpenRA", False),
            (RUST / "tests/fixtures", "Owned fixtures", False),
            (REPO / "ILSpy-tests", "ILSpy-tests", True),
            *[(Path(p), "Extra samples", True) for p in self.options.samples],
        ]:
            if not directory.is_dir():
                continue
            paths = directory.rglob("*") if recursive else directory.iterdir()
            count = 0
            for path in paths:
                if path.suffix.lower() in (".dll", ".exe", ".bin") and path.is_file():
                    self.register(path, group)
                    count += 1
                    if count >= 4096:
                        break

    def file(self, key):
        with self.lock:
            record = self.files.get(key)
        if not record:
            raise ValueError("Unknown assembly; refresh the file list.")
        return record

    def catalog(self, key):
        item = self.file(key)
        path = item["path"]
        stamp = (key, path.stat().st_mtime_ns, path.stat().st_size)
        with self.lock:
            cached = self.catalogs.get(stamp)
        if cached:
            return cached
        result = run([self.dotnet, self.bridge, "catalog", path])
        if result["exitCode"]:
            raise ValueError(result["stderr"] or "Not a readable managed PE assembly.")
        catalog = json.loads(result["text"])
        with self.lock:
            self.catalogs[stamp] = catalog
        return catalog

    def compare(self, request):
        key = request["assembly"]
        item = self.file(key)
        selected = next((t for t in self.catalog(key)["types"] if t["token"] == request.get("type")), None)
        if selected is None:
            raise ValueError("TypeDef token is not in the selected assembly.")
        mode = request.get("mode", "readable")
        if mode not in ("readable", "details", "il"):
            raise ValueError("Unknown Rust view.")
        method = request.get("method")
        if mode == "il" and method not in {m["token"] for m in selected["methods"]}:
            raise ValueError("Choose an original method belonging to this type.")
        automatic = request.get("automatic", True) is True
        self.build()
        path = item["path"]
        stamp = (key, path.stat().st_mtime_ns, path.stat().st_size, selected["token"],
                 mode, method if mode == "il" else None, automatic, self.cli.stat().st_mtime_ns)
        with self.lock:
            cached = self.cache.get(stamp)
        if cached and request.get("fresh") is not True:
            return dict(cached, cached=True)
        command = [self.cli, "il" if mode == "il" else "decompile", path]
        command += ["--method", method] if mode == "il" else ["--type", selected["name"]]
        if mode == "details":
            command += ["--details"]
        if mode != "il" and not automatic:
            command += ["--no-auto-references"]

        def reference():
            result = run([self.dotnet, self.bridge, "decompile", path, selected["token"],
                          RUST / "report/openra/runtime-deps"])
            if result["exitCode"] == 0:
                data = json.loads(result["text"])
                result.update(text=data["text"], diagnostics=data["diagnostics"],
                              errorCount=data["errorCount"], version=data["version"])
            return result

        with ThreadPoolExecutor(max_workers=2) as pool:
            left = pool.submit(reference)
            right = pool.submit(run, command)
            ilspy, ours = left.result(), right.result()
        match = re.search(r"decompiled=(\d+) declarations=(\d+) partial=(\d+) failed=(\d+)", ours["stderr"])
        ours["summary"] = dict(zip(("decompiled", "declarations", "partial", "failed"), map(int, match.groups()))) if match else None
        a, b = ilspy["text"].splitlines(), ours["text"].splitlines()
        changes = {"left": [], "right": []}
        if len(a) + len(b) <= 16000:
            for tag, i, j, k, l in difflib.SequenceMatcher(None, a, b).get_opcodes():
                if tag != "equal":
                    changes["left"].extend(range(i + 1, j + 1))
                    changes["right"].extend(range(k + 1, l + 1))
        result = {"ilspy": ilspy, "ours": ours, "type": selected, "changes": changes,
                  "cached": False, "mode": mode}
        with self.lock:
            if len(self.cache) >= 24:
                self.cache.pop(next(iter(self.cache)))
            self.cache[stamp] = result
        return result


class Handler(BaseHTTPRequestHandler):
    server_version = "IlspyLocalViewer/1"
    protocol_version = "HTTP/1.1"

    @property
    def viewer(self):
        return self.server.viewer

    def valid_host(self):
        return self.headers.get("Host", "") in {f"127.0.0.1:{self.server.server_port}", f"localhost:{self.server.server_port}"}

    def respond(self, status, value, mime="application/json; charset=utf-8"):
        data = value if isinstance(value, bytes) else json.dumps(value, ensure_ascii=True).encode()
        self.send_response(status)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if not self.valid_host():
            self.respond(403, {"error": "Loopback Host required."})
            return
        url = urlsplit(self.path)
        try:
            if url.path == "/api/bootstrap":
                self.viewer.scan()
                with self.viewer.lock:
                    files = [{k: v for k, v in row.items() if k != "path"} for row in self.viewer.files.values()]
                self.respond(200, {"token": self.viewer.token, "assemblies": sorted(files, key=lambda f: (f["group"], f["name"])),
                                  "ilspyVersion": self.viewer.version, "maxUploadBytes": MAX_INPUT,
                                  "pythonVersion": sys.version.split()[0]})
            elif url.path == "/api/types":
                key = parse_qs(url.query).get("assembly", [""])[0]
                with self.viewer.slots:
                    self.respond(200, self.viewer.catalog(key))
            elif url.path in {"/", "/index.html", "/app.js", "/style.css"}:
                filename = "index.html" if url.path == "/" else url.path[1:]
                mime = {"index.html": "text/html; charset=utf-8", "app.js": "text/javascript; charset=utf-8", "style.css": "text/css; charset=utf-8"}[filename]
                self.respond(200, (HERE / filename).read_bytes(), mime)
            else:
                self.respond(404, {"error": "Not found."})
        except (ValueError, OSError, KeyError) as error:
            self.respond(400, {"error": str(error)})

    def do_POST(self):
        origin = self.headers.get("Origin")
        allowed = {f"http://127.0.0.1:{self.server.server_port}", f"http://localhost:{self.server.server_port}"}
        if not self.valid_host() or (origin and origin not in allowed) or self.headers.get("X-Viewer-Token") != self.viewer.token:
            self.close_connection = True
            self.respond(403, {"error": "Same-origin viewer session required."})
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
            url = urlsplit(self.path)
            limit = MAX_INPUT if url.path == "/api/upload" else 8192
            if not 0 < length <= limit:
                raise ValueError("Empty request or request exceeds size limit.")
            self.connection.settimeout(40)
            body = self.rfile.read(length)
            if len(body) != length:
                raise ValueError("Incomplete request body.")
            if url.path == "/api/compare":
                request = json.loads(body)
                if not isinstance(request, dict):
                    raise ValueError("Expected a JSON object.")
                if not self.viewer.slots.acquire(blocking=False):
                    self.respond(429, {"error": "Engines busy; try again shortly."})
                    return
                try:
                    self.respond(200, self.viewer.compare(request))
                finally:
                    self.viewer.slots.release()
            elif url.path == "/api/upload":
                query = parse_qs(url.query)
                name = query.get("name", [""])[0]
                batch = query.get("batch", [""])[0]
                if not re.fullmatch(r"[a-f0-9]{32}", batch) or not name or name != Path(name).name or any(c in name for c in "\\/:\x00") or name.startswith("."):
                    raise ValueError("Invalid upload name or batch.")
                if Path(name).suffix.lower() not in (".dll", ".exe", ".bin") or body[:2] != b"MZ":
                    raise ValueError("Select a managed PE DLL/EXE/BIN, not a text file.")
                directory = REPORT / "uploads" / batch
                directory.mkdir(parents=True, exist_ok=True)
                path = directory / name
                with path.open("xb") as stream:
                    stream.write(body)
                key = self.viewer.register(path, "Uploaded")
                self.respond(200, {"id": key})
            else:
                self.respond(404, {"error": "Not found."})
        except (ValueError, OSError, KeyError, TypeError) as error:
            self.respond(400, {"error": str(error)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--engine", help="Existing ICSharpCode.Decompiler.dll; never downloaded")
    parser.add_argument("--samples", action="append", default=[], help="Additional local sample directory (repeatable)")
    parser.add_argument("--no-build", action="store_true", help="Use existing Rust release binary without rebuilding")
    options = parser.parse_args()
    REPORT.mkdir(parents=True, exist_ok=True)
    viewer = Viewer(options)
    with ThreadingHTTPServer(("127.0.0.1", options.port), Handler) as server:
        server.daemon_threads = True
        server.viewer = viewer
        (REPORT / "server.pid").write_text(str(os.getpid()) + "\n")
        print(f"Python: {sys.executable}", flush=True)
        print(f"Local viewer: http://127.0.0.1:{server.server_port}", flush=True)
        print(f"ILSpy: {viewer.version}; Rust: {viewer.cli}", flush=True)
        server.serve_forever()


if __name__ == "__main__":
    main()
