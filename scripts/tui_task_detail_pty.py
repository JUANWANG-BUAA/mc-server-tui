#!/usr/bin/env python3
"""Drive task-detail refresh and event-cursor reconnection through a PTY."""

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
TASK_ID = "11111111-1111-4111-8111-111111111111"


def task_snapshot(sequence):
    return {
        "id": TASK_ID,
        "kind": "create_paper_instance",
        "instance_id": None,
        "status": "running",
        "phase": "downloading_artifact" if sequence < 3 else "installing_instance",
        "message": "Installing Paper server artifact",
        "progress": {"completed_steps": 1, "total_steps": 3},
        "created_at": "2026-10-17T00:00:00Z",
        "updated_at": "2026-10-17T00:00:01Z",
        "finished_at": None,
        "result": None,
        "error": None,
        "last_event_sequence": sequence,
    }


def task_event(sequence, phase, message):
    return {
        "sequence": sequence,
        "task_id": TASK_ID,
        "occurred_at": f"2026-10-17T00:00:0{sequence}Z",
        "status": "running",
        "phase": phase,
        "message": message,
        "progress": {"completed_steps": sequence, "total_steps": 3},
        "error": None,
    }


def response_for(request):
    payload = request["payload"]
    operation = payload["operation"]
    arguments = payload.get("arguments") or {}
    if operation == "server_info":
        return {
            "protocol_version": request.get("protocol_version", 9),
            "payload": {
                "result": "server_info",
                "data": {
                    "product": "mc-server-tui",
                    "version": "0.1.0",
                    "api_protocol_min": 8,
                    "api_protocol_max": 9,
                    "state_schema": 1,
                    "runner_protocol": 1,
                },
            },
        }
    if operation == "list_instances":
        response = {"result": "instances", "data": []}
    elif operation == "list_tasks":
        response = {"result": "tasks", "data": [task_snapshot(2)]}
    elif operation == "get_task":
        response = {"result": "task", "data": task_snapshot(3)}
    elif operation == "list_task_events":
        after = arguments["after_sequence"]
        if after == 0:
            events = [
                task_event(1, "resolving_artifact", "EVENT_ONE_RESOLVED"),
                task_event(2, "downloading_artifact", "EVENT_TWO_DOWNLOADED"),
            ]
        elif after == 2:
            events = [task_event(3, "installing_instance", "EVENT_THREE_INSTALLING")]
        else:
            events = []
        response = {"result": "task_events", "data": events}
    else:
        response = {
            "result": "error",
            "data": {
                "code": "PROTOCOL_INVALID",
                "reason": f"unexpected request: {operation}",
                "next_action": None,
                "evidence": None,
                "retryable": False,
            },
        }
    return {"protocol_version": request.get("protocol_version", 9), "payload": response}


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    tui = repo / "target/debug/mc-server-tui"
    if not tui.is_file():
        raise SystemExit("Build the TUI first: cargo build -p mc-tui")

    with tempfile.TemporaryDirectory(prefix="mc-tui-detail-pty-") as directory:
        socket_path = str(pathlib.Path(directory) / "fake-daemon.sock")
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(socket_path)
        listener.listen(8)
        listener.settimeout(0.1)
        state = {"running": True, "requests": []}
        state_lock = threading.Lock()

        def serve(connection):
            with connection:
                connection.settimeout(2)
                stream = connection.makefile("rb")
                line = stream.readline()
                request = json.loads(line)
                with state_lock:
                    state["requests"].append(request)
                response = json.dumps(response_for(request)).encode() + b"\n"
                connection.sendall(response)

        def accept_requests():
            while state["running"]:
                try:
                    connection, _ = listener.accept()
                except socket.timeout:
                    continue
                except OSError:
                    return
                threading.Thread(target=serve, args=(connection,), daemon=True).start()

        threading.Thread(target=accept_requests, daemon=True).start()
        pid = None
        master = None
        reaped = False
        captured = bytearray()
        try:
            pid, master = pty.fork()
            if pid == 0:
                environment = os.environ.copy()
                environment["TERM"] = "xterm-256color"
                environment["LANG"] = "C.UTF-8"
                os.execve(str(tui), [str(tui), "--socket", socket_path], environment)

            fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

            def wait_for_output(marker, timeout=4):
                deadline = time.monotonic() + timeout
                while time.monotonic() < deadline:
                    clean = ANSI_SEQUENCE.sub(b"", captured)
                    if marker in re.sub(rb"\s+", b"", clean):
                        return True
                    ready, _, _ = select.select([master], [], [], 0.1)
                    if ready:
                        try:
                            captured.extend(os.read(master, 65536))
                        except OSError:
                            return False
                return marker in re.sub(rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured))

            def wait_for_animation_frames(timeout=3):
                frames = ("◐".encode(), "◓".encode(), "◑".encode(), "◒".encode())
                observed = set()
                deadline = time.monotonic() + timeout
                while time.monotonic() < deadline:
                    clean = ANSI_SEQUENCE.sub(b"", captured)
                    observed.update(frame for frame in frames if frame in clean)
                    if len(observed) >= 2:
                        return observed
                    ready, _, _ = select.select([master], [], [], 0.1)
                    if ready:
                        try:
                            captured.extend(os.read(master, 65536))
                        except OSError:
                            break
                return observed

            if not wait_for_output("F1帮助".encode()):
                raise AssertionError("dashboard did not render with the fake daemon")
            if not wait_for_output("下载校验".encode()):
                raise AssertionError("dashboard did not load the running task")
            animation_frames = wait_for_animation_frames()
            if len(animation_frames) < 2:
                raise AssertionError("running-task spinner did not advance between frames")

            os.write(master, b"/task 1\r")
            if not wait_for_output("EVENT_TWO_DOWNLOADED".encode()):
                raise AssertionError("task detail did not render its initial event history")

            os.write(master, b"r")
            if not wait_for_output("EVENT_THREE_INSTALLING".encode()):
                raise AssertionError("task detail did not append the next event")
            with state_lock:
                cursors = [
                    request["payload"]["arguments"]["after_sequence"]
                    for request in state["requests"]
                    if request["payload"]["operation"] == "list_task_events"
                ]
            if 0 not in cursors or 2 not in cursors:
                raise AssertionError(f"event requests did not advance their cursor: {cursors}")

            os.write(master, b"\x1b")
            time.sleep(0.2)
            os.write(master, b"/task 1\r")
            deadline = time.monotonic() + 3
            resumed = False
            while time.monotonic() < deadline and not resumed:
                with state_lock:
                    resumed = any(
                        request["payload"]["operation"] == "list_task_events"
                        and request["payload"]["arguments"]["after_sequence"] == 3
                        for request in state["requests"]
                    )
                if not resumed:
                    ready, _, _ = select.select([master], [], [], 0.1)
                    if ready:
                        try:
                            captured.extend(os.read(master, 65536))
                        except OSError:
                            break
            if not resumed:
                raise AssertionError("reopening task detail did not resume from sequence 3")

            os.write(master, b"/quit\r")
            deadline = time.monotonic() + 3
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
                raise AssertionError("task detail did not return to the dashboard cleanly")
            if b"\x1b[?1049l" not in captured:
                raise AssertionError("TUI did not restore the alternate screen")

            print("Task detail render at 80×24: passed")
            print("Running-task animation advanced between frames: passed")
            print(f"Event cursor refresh (0 → 2 → 3): {cursors} → passed")
            print("Reopen resumes from the last event sequence: passed")
            print("Task detail exit and alternate-screen cleanup: passed")
        finally:
            state["running"] = False
            listener.close()
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
