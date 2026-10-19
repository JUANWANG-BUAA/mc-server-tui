#!/usr/bin/env python3
"""Serve deterministic protocol-v8 responses for real TUI visual runs."""

import argparse
import json
import os
import pathlib
import signal
import socket
import threading


INSTANCE_ID = "22222222-2222-4222-8222-222222222222"
TASK_ID = "11111111-1111-4111-8111-111111111111"
CREATED_TASK_ID = "33333333-3333-4333-8333-333333333333"
BACKUP_LIST_TASK_ID = "55555555-5555-4555-8555-555555555555"
VERIFY_BACKUP_TASK_ID = "66666666-6666-4666-8666-666666666666"
PLAN_RESTORE_TASK_ID = "77777777-7777-4777-8777-777777777777"
BACKUP_ID = "88888888-8888-4888-8888-888888888888"
RESTORE_TOKEN = "v1:" + "a" * 64
NOW = "2026-10-17T00:00:00Z"


def instance_summary(name="视觉验收服"):
    return {
        "id": INSTANCE_ID,
        "name": name,
        "provider": "paper",
        "game_version": "1.21.11",
        "build_id": 17,
        "java_path": "/usr/bin/java",
        "min_memory_mb": 1024,
        "max_memory_mb": 2048,
        "port": 25565,
        "directory": "/tmp/mc-server-tui-visual/server",
        "created_at": NOW,
    }


def task_snapshot(task_id=TASK_ID, sequence=3, status="running"):
    phase = (
        "queued"
        if status == "queued"
        else "installing_instance"
        if sequence >= 3
        else "downloading_artifact"
    )
    return {
        "id": task_id,
        "kind": "create_paper_instance",
        "instance_id": INSTANCE_ID if task_id == CREATED_TASK_ID else None,
        "status": status,
        "phase": phase,
        "message": "安装 Paper 服务端文件",
        "progress": {"completed_steps": min(sequence, 3), "total_steps": 3},
        "created_at": NOW,
        "updated_at": NOW,
        "finished_at": None,
        "result": None,
        "error": None,
        "last_event_sequence": sequence,
    }


def download_task_snapshot(cancelled=False):
    task = task_snapshot(
        CREATED_TASK_ID, sequence=1, status="cancelled" if cancelled else "running"
    )
    task.update(
        {
            "phase": "cancelled" if cancelled else "downloading_artifact",
            "message": "Download cancelled; resumable partial file retained."
            if cancelled
            else "Downloading Paper server.jar.",
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
        }
    )
    return task

def backup_list_task_snapshot():
    return {
        "id": BACKUP_LIST_TASK_ID,
        "kind": "list_world_backups",
        "instance_id": INSTANCE_ID,
        "status": "queued",
        "phase": "queued",
        "message": "Waiting for an execution slot.",
        "progress": None,
        "created_at": NOW,
        "updated_at": NOW,
        "finished_at": None,
        "result": None,
        "error": None,
        "last_event_sequence": 0,
    }


def completed_backup_list_task_snapshot():
    task = backup_list_task_snapshot()
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "message": "Backup page listed and its contents verified.",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": NOW,
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
                            },
                            {
                                "id": "99999999-9999-4999-8999-999999999999",
                                "created_at": "2026-10-16T18:30:00Z",
                                "consistency": "server_stopped",
                                "file_count": 34,
                                "total_bytes": 73400320,
                            },
                            {
                                "id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                                "created_at": "2026-10-15T09:15:00Z",
                                "consistency": "server_stopped",
                                "file_count": 9,
                                "total_bytes": 3145728,
                            },
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
    task = backup_list_task_snapshot()
    task["id"] = task_id
    task["kind"] = kind
    task["message"] = message
    return task


def completed_verify_task_snapshot():
    task = operation_task_snapshot(
        VERIFY_BACKUP_TASK_ID,
        "verify_world_backup",
        "Backup integrity verified.",
    )
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": NOW,
            "result": {
                "kind": "world_backup_verified",
                "data": {
                    "instance_id": INSTANCE_ID,
                    "backup": {
                        "id": BACKUP_ID,
                        "created_at": NOW,
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
        PLAN_RESTORE_TASK_ID,
        "plan_world_backup_restore",
        "Restore plan verified.",
    )
    task.update(
        {
            "status": "succeeded",
            "phase": "completed",
            "progress": {"completed_steps": 1, "total_steps": 1},
            "finished_at": NOW,
            "result": {
                "kind": "world_backup_restore_planned",
                "data": {
                    "instance_id": INSTANCE_ID,
                    "plan": {
                        "instance_id": INSTANCE_ID,
                        "backup_id": BACKUP_ID,
                        "created_at": NOW,
                        "current_worlds": ["world", "world_nether"],
                        "target_worlds": ["world", "world_the_end"],
                        "replaced_worlds": ["world"],
                        "added_worlds": ["world_the_end"],
                        "removed_worlds": ["world_nether"],
                        "required_space_bytes": 2097152,
                        "available_space_bytes": 10737418240,
                        "confirmation_token": RESTORE_TOKEN,
                    },
                },
            },
            "last_event_sequence": 2,
        }
    )
    return task

def task_events(after_sequence, task_id=TASK_ID):
    events = [
        (1, "resolving_artifact", "已解析 Paper 稳定版本"),
        (2, "downloading_artifact", "服务端文件校验完成"),
        (3, "installing_instance", "正在准备实例目录"),
    ]
    return [
        {
            "sequence": sequence,
            "task_id": task_id,
            "occurred_at": NOW,
            "status": "running",
            "phase": phase,
            "message": message,
            "progress": {"completed_steps": sequence, "total_steps": 3},
            "error": None,
        }
        for sequence, phase, message in events
        if sequence > after_sequence
    ]


def server_snapshot(state, status=None, message=None):
    server_state = status or state["server_state"]
    return {
        "instance_id": INSTANCE_ID,
        "state": server_state,
        "pid": os.getpid() if server_state in ("starting", "running", "stopping") else None,
        "port": 25565,
        "address": "127.0.0.1:25565",
        "started_at": NOW if server_state != "installed" else None,
        "last_exit_code": 0 if server_state == "stopped" else None,
        "message": message or state["message"],
    }


def response_for(request, state):
    payload = request["payload"]
    operation = payload["operation"]
    arguments = payload.get("arguments") or {}

    if operation == "doctor":
        return {
            "result": "doctor",
            "data": {
                "platform": "macos",
                "architecture": "aarch64",
                "data_directory": "/tmp/mc-server-tui-visual",
                "java_runtimes": [
                    {
                        "executable": "/opt/homebrew/opt/openjdk@21/bin/java",
                        "version": "21.0.7",
                        "major_version": 21,
                        "architecture": "aarch64",
                        "vendor": "Eclipse Adoptium",
                    },
                    {
                        "executable": "/opt/homebrew/opt/openjdk@25/bin/java",
                        "version": "25.0.1",
                        "major_version": 25,
                        "architecture": "aarch64",
                        "vendor": "Eclipse Adoptium",
                    },
                ],
                "checks": [],
            },
        }
    if operation == "paper_versions":
        stable = [
            ("1.21.11", "1.21"),
            ("1.21.10", "1.21"),
            ("26.3", "26.3"),
            ("26.2", "26.2"),
            ("1.21.9", "1.21"),
            ("1.21.8", "1.21"),
            ("1.21.7", "1.21"),
            ("1.21.6", "1.21"),
            ("1.21.5", "1.21"),
            ("1.21.4", "1.21"),
            ("1.21.3", "1.21"),
            ("1.21.2", "1.21"),
            ("1.21.1", "1.21"),
            ("1.20.6", "1.20"),
            ("1.20.4", "1.20"),
            ("1.20.2", "1.20"),
            ("1.20.1", "1.20"),
            ("1.19.4", "1.19"),
            ("1.18.2", "1.18"),
        ]
        return {
            "result": "paper_versions",
            "data": [
                {"id": version, "group": group} for version, group in stable
            ]
            + [{"id": "1.21.11-rc1", "group": "1.21"}],
        }
    if operation == "list_instances":
        return {"result": "instances", "data": [instance_summary()]}
    if operation == "list_tasks":
        if state.get("idle"):
            return {"result": "tasks", "data": []}
        tasks = [download_task_snapshot(state["download_cancelled"]), task_snapshot()]
        if state["backup_list_task"]:
            tasks.insert(0, completed_backup_list_task_snapshot())
        return {"result": "tasks", "data": tasks}
    if operation == "get_task":
        task_id = arguments["task_id"]
        if task_id == CREATED_TASK_ID:
            return {
                "result": "task",
                "data": download_task_snapshot(state["download_cancelled"]),
            }
        if task_id == BACKUP_LIST_TASK_ID and state["backup_list_task"]:
            return {"result": "task", "data": completed_backup_list_task_snapshot()}
        if task_id == VERIFY_BACKUP_TASK_ID and state["verify_task"]:
            return {"result": "task", "data": completed_verify_task_snapshot()}
        if task_id == PLAN_RESTORE_TASK_ID and state["plan_task"]:
            return {"result": "task", "data": completed_plan_task_snapshot()}
        return {"result": "task", "data": task_snapshot()}
    if operation == "list_task_events":
        request_task_id = arguments["task_id"]
        if request_task_id == BACKUP_LIST_TASK_ID:
            return {"result": "task_events", "data": []}
        return {
            "result": "task_events",
            "data": task_events(arguments["after_sequence"], request_task_id),
        }
    if operation == "get_instance_detail":
        return {
            "result": "instance_detail",
            "data": {
                "instance": instance_summary(),
                "server": server_snapshot(state),
            },
        }
    if operation == "read_server_logs":
        logs = state["logs"]
        encoded = logs.encode("utf-8")
        start = min(arguments["after_offset"], len(encoded))
        end = min(start + arguments.get("max_bytes", 16384), len(encoded))
        return {
            "result": "server_logs",
            "data": {
                "instance_id": INSTANCE_ID,
                "start_offset": start,
                "next_offset": end,
                "content": encoded[start:end].decode("utf-8", errors="ignore"),
                "truncated": False,
            },
        }
    if operation == "start_server":
        state["server_state"] = "running"
        state["message"] = "Paper 已就绪，监听 25565 端口。"
        state["logs"] += 'Done (2.4s)! For help, type "help"\n'
        return {
            "result": "server_status",
            "data": server_snapshot(
                state, "starting", "Java 已启动，正在等待就绪标记。"
            ),
        }
    if operation == "send_console_command":
        command = arguments["command"]
        state["console_commands"].append(command)
        state["logs"] += f"> {command}\n命令已执行。\n"
        return {"result": "console_command_accepted"}
    if operation == "stop_server":
        state["server_state"] = "stopped"
        state["message"] = "服务器已优雅停止。"
        state["logs"] += "Server closed.\n"
        return {
            "result": "server_status",
            "data": server_snapshot(state),
        }
    if operation == "list_world_backups":
        if arguments.get("instance_id") != INSTANCE_ID or arguments.get("limit") != 3:
            raise ValueError("unexpected backup list request arguments")
        state["backup_list_task"] = backup_list_task_snapshot()
        return {"result": "task_queued", "data": state["backup_list_task"]}
    if operation == "verify_world_backup":
        state["verify_task"] = operation_task_snapshot(
            VERIFY_BACKUP_TASK_ID,
            "verify_world_backup",
            "Verifying selected backup.",
        )
        return {"result": "task_queued", "data": state["verify_task"]}
    if operation == "plan_world_backup_restore":
        state["plan_task"] = operation_task_snapshot(
            PLAN_RESTORE_TASK_ID,
            "plan_world_backup_restore",
            "Building restore impact preview.",
        )
        return {"result": "task_queued", "data": state["plan_task"]}
    if operation == "analyze_modpack":
        return {
            "result": "modpack_analysis",
            "data": {
                "path": arguments["path"],
                "name": "Create: Above and Beyond",
                "version_id": "1.4.0",
                "minecraft_version": "1.21.1",
                "loader": {"dependency": "fabric-loader", "version": "0.16.9"},
                "status": "review",
                "install_files": 42,
                "review_files": 3,
                "excluded_files": 18,
                "mod_files": 35,
                "total_install_bytes": 187904819,
                "server_override_files": 6,
                "issues": [
                    {
                        "severity": "warning",
                        "code": "server_compatibility_unknown",
                        "title": "3 个文件需要人工确认",
                        "detail": "这些 JAR 没有声明 server=required，不能安全地自动安装。",
                        "action": "检查项目页面的服务端支持信息，或选择作者提供的服务端包。",
                    },
                    {
                        "severity": "info",
                        "code": "download_hosts",
                        "title": "下载域名已列出",
                        "detail": "cdn.modrinth.com",
                        "action": "真正下载前仍会对每个文件校验元数据哈希。",
                    },
                ],
                "files": [],
                "files_truncated": False,
            },
        }
    if operation == "cancel_task":
        state["download_cancelled"] = True
        return {
            "result": "task",
            "data": download_task_snapshot(state["download_cancelled"]),
        }
    if operation == "create_paper_instance":
        state["created_task"] = download_task_snapshot()
        state["created_task"]["message"] = f"已提交创建：{arguments['name']}"
        return {"result": "task_queued", "data": state["created_task"]}

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


def serve_connection(connection, state, lock):
    with connection:
        connection.settimeout(5)
        line = bytearray()
        while b"\n" not in line:
            chunk = connection.recv(8192)
            if not chunk:
                return
            line.extend(chunk)
            if len(line) > 65536:
                return
        try:
            request = json.loads(line)
            with lock:
                payload = response_for(request, state)
            response = {"protocol_version": 8, "payload": payload}
            connection.sendall(json.dumps(response, ensure_ascii=False).encode() + b"\n")
        except (KeyError, ValueError, OSError) as error:
            print(f"request failed: {error}", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--socket", type=pathlib.Path, required=True)
    parser.add_argument("--idle", action="store_true", help="return no background tasks")
    args = parser.parse_args()
    args.socket.parent.mkdir(parents=True, exist_ok=True)
    args.socket.unlink(missing_ok=True)
    state = {
        "server_state": "running",
        "message": "Paper 已就绪，监听 25565 端口。",
        "logs": 'Starting minecraft server version 1.21.11\n'
        'Preparing level "world"\n'
        'Done (2.4s)! For help, type "help"\n'
        'Player Steve joined the game\n',
        "console_commands": [],
        "created_task": None,
        "idle": args.idle,
        "download_cancelled": False,
        "backup_list_task": None,
        "verify_task": None,
        "plan_task": None,
    }
    lock = threading.Lock()
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(args.socket))
    listener.listen(16)
    listener.settimeout(0.2)
    running = threading.Event()
    running.set()

    def stop(_signal, _frame):
        running.clear()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    print(f"READY {args.socket}", flush=True)
    try:
        while running.is_set():
            try:
                connection, _ = listener.accept()
            except socket.timeout:
                continue
            except OSError:
                break
            threading.Thread(
                target=serve_connection, args=(connection, state, lock), daemon=True
            ).start()
    finally:
        listener.close()
        args.socket.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
