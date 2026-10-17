#!/usr/bin/env python3
"""Check that the TUI stays responsive while daemon RPCs are stalled."""

import fcntl
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
FOOTER = "q退出".encode()


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
        state = {"running": True, "connections": []}

        def hold_request(connection):
            try:
                connection.settimeout(1)
                connection.recv(1024)
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
                if FOOTER in ANSI_SEQUENCE.sub(b"", captured):
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break

            clean_output = ANSI_SEQUENCE.sub(b"", captured)
            if FOOTER not in clean_output:
                raise AssertionError("TUI did not draw while daemon replies were pending")
            if b"\x1b[?1049h" not in captured:
                raise AssertionError("TUI did not enter the alternate screen")

            deadline = time.monotonic() + 1
            while time.monotonic() < deadline and len(state["connections"]) < 2:
                time.sleep(0.01)
            if len(state["connections"]) < 2:
                raise AssertionError("instance and task requests were not concurrent")

            started = time.monotonic()
            os.write(master, b"q")
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
            print(f"Quit key response while stalled: {time.monotonic() - started:.2f}s")
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
