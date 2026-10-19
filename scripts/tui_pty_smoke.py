#!/usr/bin/env python3
"""Check that the TUI stays responsive while daemon RPCs are stalled."""

import fcntl
import json
import os
import pathlib
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


ANSI_SEQUENCE = re.compile(
    rb"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))"
)
FOOTER = "F1帮助".encode()
WIZARD_TITLE = "新建Paper服务器".encode()
LOADING_MARKER = "正在读取Paper稳定版本".encode()


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    tui = repo / "target/debug/mc-server-tui"
    if not tui.is_file():
        raise SystemExit("Build the TUI first: cargo build -p mc-tui")

    with tempfile.TemporaryDirectory(prefix="mc-tui-pty-") as directory:
        socket_path = str(pathlib.Path(directory) / "fake-daemon.sock")
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(socket_path)
        listener.listen(8)
        listener.settimeout(0.1)
        state = {"running": True, "connections": [], "requests": []}

        def hold_request(connection):
            try:
                connection.settimeout(1)
                request = bytearray()
                while b"\n" not in request and state["running"]:
                    chunk = connection.recv(1024)
                    if not chunk:
                        break
                    request.extend(chunk)
                state["requests"].append(request)
                while state["running"]:
                    time.sleep(0.05)
            except OSError:
                pass
            finally:
                connection.close()

        def accept_requests():
            while state["running"]:
                try:
                    connection, _ = listener.accept()
                except socket.timeout:
                    continue
                except OSError:
                    return
                state["connections"].append(connection)
                threading.Thread(
                    target=hold_request, args=(connection,), daemon=True
                ).start()

        threading.Thread(target=accept_requests, daemon=True).start()
        pid = None
        master = None
        reaped = False
        try:
            pid, master = pty.fork()
            if pid == 0:
                environment = os.environ.copy()
                environment["TERM"] = "xterm-256color"
                environment["LANG"] = "C.UTF-8"
                os.execve(str(tui), [str(tui), "--socket", socket_path], environment)

            fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
            captured = bytearray()
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                compact_output = re.sub(rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured))
                if FOOTER in compact_output:
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break

            clean_output = ANSI_SEQUENCE.sub(b"", captured)
            compact_output = re.sub(rb"\s+", b"", clean_output)
            if FOOTER not in compact_output:
                raise AssertionError(
                    "TUI did not draw while daemon replies were pending; "
                    f"terminal output={clean_output[-1200:]!r}"
                )
            if b"\x1b[?1049h" not in captured:
                raise AssertionError("TUI did not enter the alternate screen")

            deadline = time.monotonic() + 1
            while time.monotonic() < deadline and len(state["connections"]) < 2:
                time.sleep(0.01)
            if len(state["connections"]) < 2:
                raise AssertionError("instance and task requests were not concurrent")

            os.write(master, b"/new\r")
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                clean_output = ANSI_SEQUENCE.sub(b"", captured)
                compact_output = re.sub(rb"\s+", b"", clean_output)
                if WIZARD_TITLE in compact_output:
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            clean_output = ANSI_SEQUENCE.sub(b"", captured)
            compact_output = re.sub(rb"\s+", b"", clean_output)
            if WIZARD_TITLE not in compact_output:
                raise AssertionError("wizard did not open while daemon requests were pending")
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                clean_output = ANSI_SEQUENCE.sub(b"", captured)
                compact_output = re.sub(rb"\s+", b"", clean_output)
                if LOADING_MARKER in compact_output:
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            if LOADING_MARKER not in compact_output:
                raise AssertionError(
                    "guided wizard did not show its loading guidance; "
                    f"recent={clean_output[-1200:]!r}"
                )

            started = time.monotonic()
            os.write(master, b"\x03")
            deadline = started + 3
            exit_code = None
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        pass
                waited, status = os.waitpid(pid, os.WNOHANG)
                if waited:
                    reaped = True
                    exit_code = os.waitstatus_to_exitcode(status)
                    break
            if exit_code != 0:
                raise AssertionError("quit key did not exit cleanly during pending RPCs")
            if b"\x1b[?1049l" not in captured:
                raise AssertionError("TUI did not restore the alternate screen")

            print("PTY dashboard render at 80×24: passed")
            print("Concurrent instance/task requests while daemon is stalled: passed")
            print("Guided wizard opens and explains option loading: passed")
            print(f"Ctrl-C response while options RPC was stalled: {time.monotonic() - started:.2f}s")
            print("Alternate-screen cleanup: passed")
        finally:
            state["running"] = False
            listener.close()
            for connection in state["connections"]:
                try:
                    connection.close()
                except OSError:
                    pass
            if pid is not None and not reaped:
                try:
                    waited, _ = os.waitpid(pid, os.WNOHANG)
                    if not waited:
                        os.kill(pid, signal.SIGKILL)
                        os.waitpid(pid, 0)
                except ChildProcessError:
                    pass
            if master is not None:
                os.close(master)


if __name__ == "__main__":
    main()
