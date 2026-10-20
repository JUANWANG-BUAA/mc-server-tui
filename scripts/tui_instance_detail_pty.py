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
LIST_TASK_ID = "55555555-5555-4555-8555-555555555555"
VERIFY_TASK_ID = "66666666-6666-4666-8666-666666666666"
PLAN_TASK_ID = "77777777-7777-4777-8777-777777777777"
RESTORE_TASK_ID = "99999999-9999-4999-8999-999999999999"
BACKUP_ID = "88888888-8888-4888-8888-888888888888"
TOKEN = "v1:" + "a" * 64


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


def list_task_snapshot():
    task = backup_task_snapshot()
    task["id"] = LIST_TASK_ID
    task["kind"] = "list_world_backups"
    task["message"] = "Reading backup metadata and verifying this page."
    return task


def completed_list_task_snapshot():
    task = list_task_snapshot()
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "message": "Backup page listed and its contents verified.",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": "2026-10-17T00:00:01Z",
            "result": {
                "kind": "world_backups_listed",
                "data": {
                    "instance_id": INSTANCE_ID,
                    "page": {
                        "backups": [
                            {
                                "id": "88888888-8888-4888-8888-888888888888",
                                "created_at": "2026-10-17T00:00:00Z",
                                "consistency": "server_stopped",
                                "file_count": 12,
                                "total_bytes": 2097152,
                            }
                        ],
                        "next_cursor": None,
                    },
                },
            },
            "last_event_sequence": 2,
        }
    )
    return task


def operation_task_snapshot(task_id, kind, message):
    task = backup_task_snapshot()
    task["id"] = task_id
    task["kind"] = kind
    task["message"] = message
    return task


def completed_verify_task_snapshot():
    task = operation_task_snapshot(
        VERIFY_TASK_ID, "verify_world_backup", "Backup integrity verified."
    )
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": "2026-10-17T00:00:02Z",
            "result": {
                "kind": "world_backup_verified",
                "data": {
                    "instance_id": INSTANCE_ID,
                    "backup": {
                        "id": BACKUP_ID,
                        "created_at": "2026-10-17T00:00:00Z",
                        "consistency": "server_stopped",
                        "file_count": 12,
                        "total_bytes": 2097152,
                    },
                },
            },
            "last_event_sequence": 2,
        }
    )
    return task

def completed_plan_task_snapshot():
    task = operation_task_snapshot(
        PLAN_TASK_ID,
        "plan_world_backup_restore",
        "Restore plan verified.",
    )
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": "2026-10-17T00:00:03Z",
            "result": {
                "kind": "world_backup_restore_planned",
                "data": {
                    "instance_id": INSTANCE_ID,
                    "plan": {
                        "instance_id": INSTANCE_ID,
                        "backup_id": BACKUP_ID,
                        "created_at": "2026-10-17T00:00:00Z",
                        "current_worlds": ["world", "world_nether"],
                        "target_worlds": ["world", "world_the_end"],
                        "replaced_worlds": ["world"],
                        "added_worlds": ["world_the_end"],
                        "removed_worlds": ["world_nether"],
                        "required_space_bytes": 2097152,
                        "available_space_bytes": 10737418240,
                        "confirmation_token": TOKEN,
                    },
                },
            },
            "last_event_sequence": 2,
        }
    )
    return task


def completed_restore_task_snapshot():
    task = operation_task_snapshot(
        RESTORE_TASK_ID,
        "restore_world_backup",
        "World restore committed and verified.",
    )
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": "2026-10-17T00:00:04Z",
            "result": {
                "kind": "world_backup_restored",
                "data": {"instance_id": INSTANCE_ID, "backup_id": BACKUP_ID},
            },
            "last_event_sequence": 2,
        }
    )
    return task

def response_for(request, state):
    payload = request["payload"]
    operation = payload["operation"]
    arguments = payload.get("arguments") or {}
    if operation == "server_info":
        response = {
            "result": "server_info",
            "data": {
                "product": "mc-server-tui",
                "version": "0.1.0",
                "api_protocol_min": 8,
                "api_protocol_max": 11,
                "state_schema": 1,
                "runner_protocol": 1,
            },
        }
        return {"protocol_version": request.get("protocol_version", 9), "payload": response}
    if operation == "list_instances":
        response = {"result": "instances", "data": [instance_summary()]}
    elif operation == "list_tasks":
        tasks = [task for task in (state["list_task"], state["backup_task"]) if task]
        response = {"result": "tasks", "data": tasks}
    elif operation == "get_task":
        task_id = arguments["task_id"]
        queued_by_id = {
            LIST_TASK_ID: state["list_task"],
            VERIFY_TASK_ID: state["verify_task"],
            PLAN_TASK_ID: state["plan_task"],
            RESTORE_TASK_ID: state["restore_task"],
        }
        completed_by_id = {
            LIST_TASK_ID: completed_list_task_snapshot,
            VERIFY_TASK_ID: completed_verify_task_snapshot,
            PLAN_TASK_ID: completed_plan_task_snapshot,
            RESTORE_TASK_ID: completed_restore_task_snapshot,
        }
        if task_id in completed_by_id and queued_by_id[task_id]:
            polls = state["task_polls"].get(task_id, 0) + 1
            state["task_polls"][task_id] = polls
            data = queued_by_id[task_id] if polls <= 1 else completed_by_id[task_id]()
            response = {"result": "task", "data": data}
        else:
            response = {"result": "error", "data": {
                "code": "TASK_NOT_FOUND",
                "reason": f"unexpected task ID: {task_id}",
                "next_action": None,
                "evidence": None,
                "retryable": False,
            }}
    elif operation == "list_task_events":
        response = {"result": "task_events", "data": []}
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
    elif operation == "list_world_backups":
        state["list_requests"] += 1
        state["list_instance_id"] = arguments["instance_id"]
        state["list_arguments"] = arguments
        state["list_task"] = list_task_snapshot()
        response = {"result": "task_queued", "data": state["list_task"]}
    elif operation == "verify_world_backup":
        state["verify_requests"] += 1
        state["verify_arguments"] = arguments
        state["verify_task"] = operation_task_snapshot(
            VERIFY_TASK_ID,
            "verify_world_backup",
            "Verifying selected backup.",
        )
        response = {"result": "task_queued", "data": state["verify_task"]}
    elif operation == "plan_world_backup_restore":
        state["plan_requests"] += 1
        state["plan_arguments"] = arguments
        state["plan_task"] = operation_task_snapshot(
            PLAN_TASK_ID,
            "plan_world_backup_restore",
            "Building restore impact preview.",
        )
        response = {"result": "task_queued", "data": state["plan_task"]}
    elif operation == "restore_world_backup":
        state["restore_requests"] += 1
        state["restore_arguments"] = arguments
        state["restore_task"] = operation_task_snapshot(
            RESTORE_TASK_ID,
            "restore_world_backup",
            "Restoring verified world backup.",
        )
        response = {"result": "task_queued", "data": state["restore_task"]}
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
    elif operation == "restart_server":
        state["server_state"] = "running"
        state["message"] = "Old process exited; replacement is starting."
        state["restart_requests"] += 1
        state["logs"] += "SERVER_READY\n"
        response = {
            "result": "server_status",
            "data": server_snapshot(
                "starting", "Old process exited; replacement is starting."
            ),
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
    return {"protocol_version": request.get("protocol_version", 9), "payload": response}


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
            "restart_requests": 0,
            "backup_requests": 0,
            "backup_instance_id": None,
            "backup_task": None,
            "list_requests": 0,
            "list_instance_id": None,
            "list_arguments": None,
            "list_task": None,
            "verify_requests": 0,
            "verify_arguments": None,
            "verify_task": None,
            "plan_requests": 0,
            "plan_arguments": None,
            "plan_task": None,
            "restore_requests": 0,
            "restore_arguments": None,
            "restore_task": None,
            "task_polls": {},
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
                environment["MC_SERVER_TUI_THEME"] = "claude"
                # Color is part of this visual regression; do not inherit a
                # host-level NO_COLOR preference into the dedicated test TTY.
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

            if not wait_for("F1帮助".encode()):
                raise AssertionError("dashboard did not render")
            if b"\x1b[?1049h" not in captured:
                raise AssertionError("TUI did not enter the alternate screen")
            if not re.search(rb"\x1b\[[0-9;]*38;2;217;119;87[0-9;]*m", captured):
                raise AssertionError("TUI did not emit the configured brand color")

            os.write(master, b"/open\r")
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

            os.write(master, b"l")
            manager_offset = len(captured)
            if not wait_for("世界与备份".encode(), start_offset=manager_offset):
                raise AssertionError("backup manager did not open")
            loading_frames = set()
            animation_deadline = time.monotonic() + 1.2
            while time.monotonic() < animation_deadline and len(loading_frames) < 2:
                ready, _, _ = select.select([master], [], [], 0.05)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
                clean = ANSI_SEQUENCE.sub(b"", captured[manager_offset:])
                for glyph in ("◐", "◓", "◑", "◒"):
                    if glyph.encode() in clean:
                        loading_frames.add(glyph)
            if len(loading_frames) < 2:
                raise AssertionError("backup task spinner did not advance through two frames")

            if not wait_for("12个文件".encode(), start_offset=manager_offset):
                raise AssertionError("backup manager did not render the verified page")
            if not wait_for("2.0MiB".encode(), start_offset=manager_offset):
                raise AssertionError("backup manager did not render total size")
            with lock:
                if state["list_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one backup list request")
                if state["list_instance_id"] != INSTANCE_ID:
                    raise AssertionError("TUI listed backups for the wrong instance")
                if state["list_arguments"].get("limit") != 3:
                    raise AssertionError("TUI did not request the bounded first page")
                if state["list_arguments"].get("cursor") is not None:
                    raise AssertionError("TUI first-page request unexpectedly included a cursor")
                if state["task_polls"].get(LIST_TASK_ID, 0) < 2:
                    raise AssertionError("TUI did not poll the list task to completion")

            verify_offset = len(captured)
            os.write(master, b"v")
            if not wait_for("完整性已验证".encode(), start_offset=verify_offset):
                raise AssertionError("TUI did not surface successful backup verification")
            with lock:
                if state["verify_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one verify request")
                expected_verify = {
                    "instance_id": INSTANCE_ID,
                    "backup_id": BACKUP_ID,
                }
                if state["verify_arguments"] != expected_verify:
                    raise AssertionError("TUI submitted the wrong verify arguments")

            plan_offset = len(captured)
            os.write(master, b"\r")
            if not wait_for("恢复影响预览".encode(), start_offset=plan_offset):
                raise AssertionError("restore plan preview did not render")
            if not wait_for("RESTORE".encode(), start_offset=plan_offset):
                raise AssertionError("restore confirmation guidance was not visible")
            with lock:
                if state["plan_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one plan request")
                expected_plan = {
                    "instance_id": INSTANCE_ID,
                    "backup_id": BACKUP_ID,
                }
                if state["plan_arguments"] != expected_plan:
                    raise AssertionError("TUI submitted the wrong restore-plan arguments")

            confirm_offset = len(captured)
            os.write(master, b"c")
            if not wait_for("确认>".encode(), start_offset=confirm_offset):
                raise AssertionError("restore confirmation input did not open")
            os.write(master, b"RESTORE\r")
            if not wait_for("世界恢复完成".encode(), start_offset=confirm_offset):
                raise AssertionError("TUI did not render the successful restore result")
            with lock:
                if state["restore_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one restore request")
                expected_restore = {
                    "instance_id": INSTANCE_ID,
                    "backup_id": BACKUP_ID,
                    "confirmation_token": TOKEN,
                }
                if state["restore_arguments"] != expected_restore:
                    raise AssertionError("TUI submitted the wrong restore confirmation")

            detail_offset = len(captured)
            os.write(master, b"\x1b")
            if not wait_for("s启动".encode(), start_offset=detail_offset):
                raise AssertionError("Esc did not return from backup manager to instance detail")

            os.write(master, b"s")
            if not wait_for("SERVER_READY".encode()):
                raise AssertionError("start action did not refresh readiness and console logs")
            with lock:
                if state["start_requests"] != 1:
                    raise AssertionError("TUI did not submit exactly one start request")

            os.write(master, b":")
            if not wait_for("命令>".encode()):
                raise AssertionError("console input did not open after ':'")
            os.write(master, b"say hi")
            deadline = time.monotonic() + 2
            typed = False
            while time.monotonic() < deadline:
                clean = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured)
                ).replace(b"\xe2\x96\x8f", b"")
                if b"sayhi" in clean:
                    typed = True
                    break
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        captured.extend(os.read(master, 65536))
                    except OSError:
                        break
            if not typed:
                recent = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[-4000:])
                )
                raise AssertionError(
                    f"console input did not render typed text; recent={recent!r}"
                )
            time.sleep(0.3)
            os.write(master, b"\r")
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                with lock:
                    if "say hi" in state["console_commands"]:
                        break
                time.sleep(0.02)
            with lock:
                if state["console_commands"] != ["say hi"]:
                    recent = re.sub(
                        rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[-4000:])
                    )
                    raise AssertionError(
                        "TUI submitted an unexpected console command: "
                        f"{state['console_commands']!r}; recent output={recent!r}"
                    )
            if not wait_for("COMMAND:sayhi".encode()):
                raise AssertionError("TUI did not refresh the console output after command submission")
            if not wait_for("控制台命令已发送".encode()):
                raise AssertionError("console command completion did not clear the pending guard")

            restart_offset = len(captured)
            os.write(master, b"R")
            if not wait_for("replacement".encode(), start_offset=restart_offset):
                recent = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[-5000:])
                )
                raise AssertionError(f"restart action did not land; recent={recent}")
            with lock:
                if state["restart_requests"] != 1:
                    raise AssertionError(
                        f"TUI restart request count was {state['restart_requests']}"
                    )

            stop_offset = len(captured)
            os.write(master, b"x")
            if not wait_for("服务器已正常停止".encode(), start_offset=stop_offset):
                recent = re.sub(
                    rb"\s+", b"", ANSI_SEQUENCE.sub(b"", captured[-5000:])
                )
                raise AssertionError(f"stop action did not land; recent={recent}")
            with lock:
                if state["stop_requests"] != 1:
                    raise AssertionError(
                        f"TUI stop request count was {state['stop_requests']}"
                    )

            dashboard_offset = len(captured)
            os.write(master, b"\x1b")
            if not wait_for("F1帮助".encode(), start_offset=dashboard_offset):
                raise AssertionError("Esc did not return to the dashboard")
            os.write(master, b"/quit\r")
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
            print("ANSI truecolor theme output: passed")
            print("Stopped-world backup queue feedback: passed")
            print("Backup-task spinner advanced through two frames: passed")
            print("Bounded backup-list task and cursor request: passed")
            print("Direct backup verification and restore preview: passed")
            print("Explicit RESTORE confirmation and durable restore task: passed")
            print("Backup manager return to instance detail: passed")
            print("Start and readiness refresh: passed")
            print("Console command and incremental output: passed")
            print("Controlled restart request and replacement feedback: passed")
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
