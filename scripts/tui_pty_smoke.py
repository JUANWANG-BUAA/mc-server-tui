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
FOOTER = "q退出".encode()
WIZARD_TITLE = "新建Paper实例".encode()
RUNTIME_STEP = "Java与资源".encode()
REVIEW_STEP = "预览与确认".encode()


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

            os.write(master, b"n")
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

            os.write(master, b"TestServer\t1.21.11\r")
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                clean_output = ANSI_SEQUENCE.sub(b"", captured)
                compact_output = re.sub(rb"\s+", b"", clean_output)
                if RUNTIME_STEP in compact_output:
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            clean_output = ANSI_SEQUENCE.sub(b"", captured)
            compact_output = re.sub(rb"\s+", b"", clean_output)
            if RUNTIME_STEP not in compact_output:
                raise AssertionError("wizard did not accept fields and advance while RPCs were pending")

            os.write(master, b"/fake/java\r")
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                clean_output = ANSI_SEQUENCE.sub(b"", captured)
                compact_output = re.sub(rb"\s+", b"", clean_output)
                if REVIEW_STEP in compact_output:
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            clean_output = ANSI_SEQUENCE.sub(b"", captured)
            compact_output = re.sub(rb"\s+", b"", clean_output)
            if REVIEW_STEP not in compact_output:
                raise AssertionError("wizard did not reach review while daemon requests were pending")

            os.write(master, b" \r")
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline and len(state["connections"]) < 3:
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            if len(state["connections"]) < 3:
                raise AssertionError("wizard did not submit the create request to the daemon")
            deadline = time.monotonic() + 1
            create_request = None
            while time.monotonic() < deadline and create_request is None:
                for request in state["requests"]:
                    try:
                        envelope = json.loads(request)
                    except (UnicodeDecodeError, json.JSONDecodeError):
                        continue
                    if envelope.get("payload", {}).get("operation") == "create_paper_instance":
                        create_request = envelope
                        break
                if create_request is None:
                    time.sleep(0.01)
            if create_request is None:
                raise AssertionError("daemon did not receive a Paper instance request")
            arguments = create_request["payload"]["arguments"]
            if (
                arguments.get("name") != "TestServer"
                or arguments.get("game_version") != "1.21.11"
                or arguments.get("build_id") is not None
                or arguments.get("java_path") != "/fake/java"
                or arguments.get("min_memory_mb") != 1024
                or arguments.get("max_memory_mb") != 2048
                or arguments.get("port") != 25565
                or arguments.get("accept_eula") is not True
            ):
                raise AssertionError("wizard submitted unexpected Paper instance parameters")

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
            print("Wizard fields and review while list RPCs are stalled: passed")
            print("Create submission reached daemon while list RPCs were stalled: passed")
            print(f"Ctrl-C response while create RPC was stalled: {time.monotonic() - started:.2f}s")
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
