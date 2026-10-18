#!/usr/bin/env python3
"""Exercise the guided create flow, byte progress, and cancellation via PTY."""

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
NOW = "2026-10-17T00:00:00Z"


def doctor_response():
    return {
        "platform": "macos",
        "architecture": "aarch64",
        "data_directory": "/tmp/mc-guided-create",
        "java_runtimes": [
            {
                "executable": "/opt/java21/bin/java",
                "version": "21.0.2",
                "major_version": 21,
                "architecture": "aarch64",
                "vendor": "Eclipse Adoptium",
            },
            {
                "executable": "/opt/java25/bin/java",
                "version": "25.0.1",
                "major_version": 25,
                "architecture": "aarch64",
                "vendor": "Eclipse Adoptium",
            },
        ],
        "checks": [],
    }


def paper_versions():
    return [
        {"id": "1.21.11", "group": "1.21"},
        {"id": "1.21.10", "group": "1.21"},
        {"id": "1.21.11-rc1", "group": "1.21"},
    ]


def running_task(cancelled=False):
    return {
        "id": TASK_ID,
        "kind": "create_paper_instance",
        "instance_id": None,
        "status": "cancelled" if cancelled else "running",
        "phase": "cancelled" if cancelled else "downloading_artifact",
        "message": "Task cancelled by user." if cancelled else "Downloading server.jar.",
        "progress": {
            "completed_steps": 1,
            "total_steps": 3,
            "transfer": {
                "downloaded_bytes": 27 * 1024 * 1024,
                "total_bytes": 54 * 1024 * 1024,
                "bytes_per_second": 3 * 1024 * 1024,
                "source": "Paper 官方 CDN · fill-data.papermc.io",
            },
        },
        "created_at": NOW,
        "updated_at": NOW,
        "finished_at": NOW if cancelled else None,
        "result": None,
        "error": None,
        "last_event_sequence": 1,
    }


def response_for(request, state):
    payload = request["payload"]
    operation = payload["operation"]
    arguments = payload.get("arguments") or {}
    if operation == "list_instances":
        return {"result": "instances", "data": []}
    if operation == "list_tasks":
        tasks = [state["created_task"]] if state["created_task"] else []
        return {"result": "tasks", "data": tasks}
    if operation == "doctor":
        return {"result": "doctor", "data": doctor_response()}
    if operation == "paper_versions":
        return {"result": "paper_versions", "data": paper_versions()}
    if operation == "create_paper_instance":
        state["create_arguments"] = arguments
        state["created_task"] = running_task()
        return {"result": "task_queued", "data": state["created_task"]}
    if operation == "get_task":
        if arguments["task_id"] != TASK_ID:
            raise ValueError("unexpected task id")
        task = running_task(cancelled=state["cancel_requested"])
        state["created_task"] = task
        return {"result": "task", "data": task}
    if operation == "list_task_events":
        return {"result": "task_events", "data": []}
    if operation == "cancel_task":
        if arguments["task_id"] != TASK_ID:
            raise ValueError("unexpected cancel task id")
        state["cancel_requests"] += 1
        state["cancel_requested"] = True
        return {"result": "task", "data": running_task()}
    return {
        "result": "error",
        "data": {
            "code": "PROTOCOL_INVALID",
            "reason": f"unexpected request: {operation}",
            "next_action": None,
            "evidence": None,
            "retryable": False,
        },
    }


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    tui = repo / "target/debug/mc-server-tui"
    if not tui.is_file():
        raise SystemExit("Build the TUI first: cargo build -p mc-tui")

    with tempfile.TemporaryDirectory(prefix="mc-tui-guided-create-") as directory:
        socket_path = str(pathlib.Path(directory) / "fake-daemon.sock")
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(socket_path)
        listener.listen(8)
        listener.settimeout(0.1)
        state = {
            "running": True,
            "created_task": None,
            "create_arguments": None,
            "cancel_requested": False,
            "cancel_requests": 0,
            "connections": [],
            "requests": [],
        }
        lock = threading.Lock()

        def serve(connection):
            with connection:
                connection.settimeout(3)
                line = bytearray()
                while b"\n" not in line:
                    part = connection.recv(4096)
                    if not part:
                        return
                    line.extend(part)
                request = json.loads(line)
                with lock:
                    state["requests"].append(request)
                    response = response_for(request, state)
                connection.sendall(
                    json.dumps(
                        {"protocol_version": 8, "payload": response},
                        ensure_ascii=False,
                    ).encode()
                    + b"\n"
                )

        def accept_requests():
            while state["running"]:
                try:
                    connection, _ = listener.accept()
                except socket.timeout:
                    continue
                except OSError:
                    return
                state["connections"].append(connection)
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
                environment.pop("NO_COLOR", None)
                os.execve(str(tui), [str(tui), "--socket", socket_path], environment)
            fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

            def wait_for(marker, timeout=5, start_offset=0):
                deadline = time.monotonic() + timeout
                while time.monotonic() < deadline:
                    clean = re.sub(
                        rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[start_offset:])
                    )
                    if marker in clean:
                        return True
                    ready, _, _ = select.select([master], [], [], 0.1)
                    if ready:
                        try:
                            captured.extend(os.read(master, 65536))
                        except OSError:
                            return False
                clean = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[start_offset:])
                )
                return marker in clean

            if not wait_for("q退出".encode()):
                raise AssertionError("dashboard did not render")
            os.write(master, b"/")
            if not wait_for("命令面板".encode()):
                raise AssertionError("slash command palette did not open")
            os.write(master, "新建".encode())
            if not wait_for("新建Paper服务器".encode()):
                raise AssertionError("command palette did not guide to create")
            os.write(master, b"\r")
            if not wait_for("实例与游戏版本".encode()):
                raise AssertionError("guided wizard did not open from command palette")
            if not wait_for("1.21.11".encode()):
                raise AssertionError("Paper stable versions did not load")

            os.write(master, b"GuidedServer")
            if not wait_for("GuidedServer".encode()):
                raise AssertionError("instance name input was not rendered")
            os.write(master, b"\t")
            os.write(master, b"\x1b[C")
            os.write(master, b"\r")
            if not wait_for("Java运行时".encode()):
                raise AssertionError("runtime picker did not open")
            if not wait_for("推荐".encode()):
                raise AssertionError("Java recommendation was not visible")
            os.write(master, b"\r")
            if not wait_for("安装计划".encode()):
                raise AssertionError("review plan did not render")
            os.write(master, b" \r")
            dashboard_offset = len(captured)
            if not wait_for("后台任务".encode(), start_offset=dashboard_offset):
                raise AssertionError("create submission did not return to dashboard")

            with lock:
                arguments = state["create_arguments"]
                if arguments is None:
                    raise AssertionError("create request was not captured")
                if arguments.get("name") != "GuidedServer":
                    raise AssertionError("instance name was not submitted")
                if arguments.get("game_version") != "1.21.10":
                    raise AssertionError(f"unexpected version: {arguments}")
                if arguments.get("java_path") != "/opt/java21/bin/java":
                    raise AssertionError(f"unexpected Java: {arguments}")
                if arguments.get("min_memory_mb") != 1024:
                    raise AssertionError(f"unexpected memory: {arguments}")
                if arguments.get("max_memory_mb") != 2048:
                    raise AssertionError(f"unexpected memory: {arguments}")
                if arguments.get("port", 0) < 25565:
                    raise AssertionError(f"invalid suggested port: {arguments}")
                if arguments.get("accept_eula") is not True:
                    raise AssertionError("EULA acceptance was not explicit")

            os.write(master, b"\t\r")
            if not wait_for("下载".encode()):
                raise AssertionError("task detail did not render download progress")
            if not wait_for("50%".encode()):
                raise AssertionError("task detail did not render byte percentage")
            if not wait_for("fill-data.papermc.io".encode()):
                raise AssertionError("task detail did not disclose download source")

            cancel_offset = len(captured)
            os.write(master, b"x")
            if not wait_for("再次按x".encode(), start_offset=cancel_offset):
                raise AssertionError("first cancel press did not ask for confirmation")
            os.write(master, b"x")
            if not wait_for("取消请求".encode(), start_offset=cancel_offset):
                recent = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[-5000:])
                )
                with lock:
                    count = state["cancel_requests"]
                raise AssertionError(
                    f"confirmed cancel request was not acknowledged; count={count}; recent={recent}"
                )
            if not wait_for("已取消".encode(), start_offset=cancel_offset):
                raise AssertionError("cancelled task state did not render")
            with lock:
                if state["cancel_requests"] != 1:
                    raise AssertionError("TUI did not send exactly one cancel request")

            os.write(master, b"q")
            deadline = time.monotonic() + 4
            status = None
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        pass
                waited, process_status = os.waitpid(pid, os.WNOHANG)
                if waited == pid:
                    status = process_status
                    reaped = True
                    break
            if status != 0:
                raise AssertionError(f"TUI exited with status {status}")
            if b"\x1b[?1049l" not in captured:
                raise AssertionError("alternate screen was not restored")
            print("Guided command palette create flow: passed")
            print("Stable version and Java pickers: passed")
            print("Explicit EULA and guided review: passed")
            print("Byte progress, speed, and source disclosure: passed")
            print("Two-step download cancellation: passed")
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
