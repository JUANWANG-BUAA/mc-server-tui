#!/usr/bin/env python3
"""Exercise instance detail, server actions, and console input through a PTY."""

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
INSTANCE_ID = "22222222-2222-4222-8222-222222222222"
BACKUP_TASK_ID = "44444444-4444-4444-8444-444444444444"


def instance_summary():
    return {
        "id": INSTANCE_ID,
        "name": "PTY Survival",
        "provider": "paper",
        "game_version": "1.21.11",
        "build_id": 17,
        "java_path": "/usr/bin/java",
        "min_memory_mb": 1024,
        "max_memory_mb": 2048,
        "port": 25565,
        "directory": "/tmp/pty-survival",
        "created_at": "2026-10-17T00:00:00Z",
    }


def server_snapshot(state, message):
    return {
        "instance_id": INSTANCE_ID,
        "state": state,
        "pid": 3210 if state in ("starting", "running", "stopping") else None,
        "port": 25565,
        "address": "127.0.0.1:25565",
        "started_at": "2026-10-17T00:00:00Z" if state != "installed" else None,
        "last_exit_code": 0 if state == "stopped" else None,
        "message": message,
    }


def backup_task_snapshot():
    return {
        "id": BACKUP_TASK_ID,
        "kind": "create_world_backup",
        "instance_id": INSTANCE_ID,
        "status": "queued",
        "phase": "queued",
        "message": "Waiting for an execution slot.",
        "progress": None,
        "created_at": "2026-10-17T00:00:00Z",
        "updated_at": "2026-10-17T00:00:00Z",
        "finished_at": None,
        "result": None,
        "error": None,
        "last_event_sequence": 0,
    }


def response_for(request, state):
    payload = request["payload"]
    operation = payload["operation"]
    arguments = payload.get("arguments") or {}
    if operation == "list_instances":
        response = {"result": "instances", "data": [instance_summary()]}
    elif operation == "list_tasks":
        tasks = [state["backup_task"]] if state["backup_task"] else []
        response = {"result": "tasks", "data": tasks}
    elif operation == "get_instance_detail":
        response = {
            "result": "instance_detail",
            "data": {
                "instance": instance_summary(),
                "server": server_snapshot(state["server_state"], state["message"]),
            },
        }
    elif operation == "read_server_logs":
        after = arguments["after_offset"]
        logs = state["logs"]
        start = min(after, len(logs))
        response = {
            "result": "server_logs",
            "data": {
                "instance_id": INSTANCE_ID,
                "start_offset": start,
                "next_offset": len(logs),
                "content": logs[start:],
                "truncated": False,
            },
        }
    elif operation == "create_world_backup":
        state["backup_requests"] += 1
        state["backup_instance_id"] = arguments["instance_id"]
        state["backup_task"] = backup_task_snapshot()
        response = {"result": "task_queued", "data": state["backup_task"]}
    elif operation == "start_server":
        state["server_state"] = "running"
        state["message"] = "Server is ready."
        state["logs"] += "SERVER_READY\n"
        state["start_requests"] += 1
        response = {
            "result": "server_status",
            "data": server_snapshot("starting", "Java started; waiting for readiness."),
        }
    elif operation == "send_console_command":
        command = arguments["command"]
        state["console_commands"].append(command)
        state["logs"] += f"COMMAND:{command}\n"
        response = {"result": "console_command_accepted"}
    elif operation == "stop_server":
        state["server_state"] = "stopped"
        state["message"] = "Server stopped successfully."
        state["stop_requests"] += 1
        response = {
            "result": "server_status",
            "data": server_snapshot("stopped", "Server stopped successfully."),
        }
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
    return {"protocol_version": 5, "payload": response}


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    tui = repo / "target/debug/mc-server-tui"
    if not tui.is_file():
        raise SystemExit("Build the TUI first: cargo build -p mc-tui")

    with tempfile.TemporaryDirectory(prefix="mc-tui-instance-pty-") as directory:
        socket_path = str(pathlib.Path(directory) / "fake-daemon.sock")
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(socket_path)
        listener.listen(8)
        listener.settimeout(0.1)
        state = {
            "running": True,
            "server_state": "installed",
            "message": "Installed and stopped.",
            "logs": "",
            "requests": [],
            "start_requests": 0,
            "stop_requests": 0,
            "backup_requests": 0,
            "backup_instance_id": None,
            "backup_task": None,
            "console_commands": [],
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
                connection.sendall(json.dumps(response).encode() + b"\n")

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
            if b"\x1b[?1049h" not in captured:
                raise AssertionError("TUI did not enter the alternate screen")

            os.write(master, b"\r")
            if not wait_for("127.0.0.1:25565".encode()):
                raise AssertionError("instance detail did not render server address")
            if not wait_for("s启动".encode()):
                raise AssertionError("instance detail did not render lifecycle controls")

            os.write(master, b"b")
            if not wait_for("世界备份任务已加入后台队列".encode()):
                raise AssertionError("backup action did not show queued-task feedback")
            with lock:
                if state["backup_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one backup request")
                if state["backup_instance_id"] != INSTANCE_ID:
                    raise AssertionError("TUI submitted the backup for the wrong instance")

            os.write(master, b"s")
            if not wait_for("SERVER_READY".encode()):
                raise AssertionError("start action did not refresh readiness and console logs")
            with lock:
                if state["start_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one start request")

            os.write(master, b":say hi\r")
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                with lock:
                    if "say hi" in state["console_commands"]:
                        break
                time.sleep(0.02)
            with lock:
                if state["console_commands"] != ["say hi"]:
                    raise AssertionError("TUI did not submit the console command")
            if not wait_for("COMMAND:sayhi".encode()):
                raise AssertionError("TUI did not refresh the console output after command submission")

            os.write(master, b"x")
            if not wait_for("已停止".encode()):
                raise AssertionError("stop action did not display stopped state")
            with lock:
                if state["stop_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one stop request")

            dashboard_offset = len(captured)
            os.write(master, b"\x1b")
            if not wait_for("Enter打开".encode(), start_offset=dashboard_offset):
                raise AssertionError("Esc did not return to the dashboard")
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
                tail = re.sub(rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured))[-400:]
                raise AssertionError(f"TUI exited with status {status}; output tail={tail!r}")
            if b"\x1b[?1049l" not in captured:
                raise AssertionError("TUI did not restore the terminal screen")
            print("Instance detail at 80×24: passed")
            print("Stopped-world backup queue feedback: passed")
            print("Start and readiness refresh: passed")
            print("Console command and incremental output: passed")
            print("Graceful stop and dashboard return: passed")
            print("Alternate-screen cleanup: passed")
        finally:
            state["running"] = False
            listener.close()
            if master is not None:
                try:
                    os.close(master)
                except OSError:
                    pass
            if pid is not None and not reaped:
                try:
                    os.kill(pid, signal.SIGTERM)
                    os.waitpid(pid, 0)
                except ProcessLookupError:
                    pass


if __name__ == "__main__":
    main()
