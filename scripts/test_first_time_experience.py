#!/usr/bin/env python3
"""Run the compiled TUI in a PTY against an isolated, deterministic fake daemon.

Covers the default welcome, Paper discovery failure/details/dismissal, and Fabric
command dispatch. No Java, real daemon, downloads, or server files are used.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import socket
import struct
import tempfile
import termios
import threading
import time

from tui_guided_create_pty import ANSI_SEQUENCE, response_for


def main():
    tui = Path(__file__).resolve().parents[1] / "target/debug/mc-server-tui"
    if not tui.is_file():
        raise SystemExit("Build first: cargo build --workspace --locked")
    with tempfile.TemporaryDirectory(prefix="mc-first-use-") as directory:
        socket_path = str(Path(directory) / "daemon.sock")
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(socket_path)
        listener.listen(8)
        listener.settimeout(0.1)
        running = threading.Event()
        running.set()
        requests = []
        failures = []
        lock = threading.Lock()
        fixture = {"created_task": None}

        def serve(connection):
            try:
                with connection:
                    connection.settimeout(3)
                    line = bytearray()
                    while b"\n" not in line:
                        chunk = connection.recv(4096)
                        if not chunk:
                            return
                        line.extend(chunk)
                    request = json.loads(line)
                    with lock:
                        requests.append(request)
                        operation = request["payload"]["operation"]
                        if operation in ("doctor", "manage_instance"):
                            payload = {"result": "error", "data": {
                                "code": "INVALID_INPUT", "reason": "First-use test failure",
                                "evidence": "FIRST_USE_PROBE=42",
                                "next_action": "Correct the input and submit again",
                                "retryable": False,
                            }}
                        else:
                            payload = response_for(request, fixture)
                    connection.sendall(json.dumps({
                        "protocol_version": request["protocol_version"], "payload": payload,
                    }).encode() + b"\n")
            except Exception as error:
                with lock:
                    failures.append(str(error))

        def accept_requests():
            while running.is_set():
                try:
                    connection, _ = listener.accept()
                except socket.timeout:
                    continue
                except OSError:
                    return
                threading.Thread(target=serve, args=(connection,), daemon=True).start()

        thread = threading.Thread(target=accept_requests, daemon=True)
        thread.start()
        pid, master = pty.fork()
        if pid == 0:
            environment = os.environ.copy()
            environment.update(TERM="xterm-256color", LANG="C.UTF-8")
            os.execve(str(tui), [str(tui), "--socket", socket_path], environment)
        captured = bytearray()
        reaped = False
        try:
            fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

            def expect(marker, start=0):
                needle = re.sub(rb"\s+", b"", marker.encode())
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    clean = re.sub(rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[start:]))
                    if needle in clean:
                        return
                    ready, _, _ = select.select([master], [], [], 0.1)
                    if ready:
                        try:
                            captured.extend(os.read(master, 65536))
                        except OSError:
                            break
                raise AssertionError(f"Missing {marker!r}; recent output={captured[-1500:]!r}")

            expect("还没有服务器")
            for marker in ("/create", "/fabric", "/import", "管理服务已连接"):
                expect(marker)
            offset = len(captured)
            os.write(master, b"/create\r")
            expect("操作失败", offset)
            expect("First-use test failure", offset)
            os.write(master, b"d")
            expect("FIRST_USE_PROBE=42", offset)
            offset = len(captured)
            os.write(master, b"\x1b")
            # The title may be outside the modal and unchanged in Ratatui's
            # diff output; the restored wizard footer is repainted on dismissal.
            expect("选择版本", offset)
            offset = len(captured)
            os.write(master, b"\x1b")
            expect("输入/命令，或直接输入控制台命令", offset)
            offset = len(captured)
            os.write(master, b"/fabric\r")
            expect("--accept-eula", offset)
            with lock:
                assert not any(r["payload"]["operation"] == "manage_instance" for r in requests), \
                    "Incomplete Fabric command was submitted"
            offset = len(captured)
            os.write(master, b"/fabric FirstFabric 1.21.11 --accept-eula\r")
            expect("操作失败", offset)
            with lock:
                submitted = [r["payload"]["arguments"] for r in requests
                             if r["payload"]["operation"] == "manage_instance"]
                assert len(submitted) == 1, submitted
                action = submitted[0]["action"]
                assert action == {"action": "create_fabric", "name": "FirstFabric",
                                  "game_version": "1.21.11", "accept_eula": True}, action
                assert not failures, failures
            os.write(master, b"\x03")
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                child, status = os.waitpid(pid, os.WNOHANG)
                if child:
                    reaped = True
                    assert os.waitstatus_to_exitcode(status) == 0
                    break
                ready, _, _ = select.select([master], [], [], 0.05)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        pass
            assert reaped, "TUI did not exit after Ctrl-C"
            print("PASS: default welcome, failure details/dismissal, Fabric dispatch, clean exit")
        finally:
            running.clear()
            listener.close()
            thread.join(timeout=1)
            os.close(master)
            if not reaped:
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    if os.waitpid(pid, os.WNOHANG)[0]:
                        break
                    time.sleep(0.05)


if __name__ == "__main__":
    main()
