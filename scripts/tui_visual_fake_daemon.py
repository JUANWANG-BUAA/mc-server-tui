#!/usr/bin/env python3
"""Serve deterministic protocol-v5 responses for a real Terminal visual run."""

import argparse
import json
import pathlib
import signal
import socket
import threading


INSTANCE_ID = "22222222-2222-4222-8222-222222222222"
TASK_ID = "11111111-1111-4111-8111-111111111111"
CREATED_TASK_ID = "33333333-3333-4333-8333-333333333333"
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


def task_events(after_sequence):
    events = [
        (1, "resolving_artifact", "已解析 Paper 稳定版本"),
        (2, "downloading_artifact", "服务端文件校验完成"),
        (3, "installing_instance", "正在准备实例目录"),
    ]
    return [
        {
            "sequence": sequence,
            "task_id": TASK_ID,
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
        "pid": 4312 if server_state in ("starting", "running", "stopping") else None,
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

    if operation == "list_instances":
        return {"result": "instances", "data": [instance_summary()]}
    if operation == "list_tasks":
        tasks = [task_snapshot()]
        if state["created_task"]:
            tasks.insert(0, state["created_task"])
        return {"result": "tasks", "data": tasks}
    if operation == "get_task":
        return {"result": "task", "data": task_snapshot()}
    if operation == "list_task_events":
        return {
            "result": "task_events",
            "data": task_events(arguments["after_sequence"]),
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
    if operation == "create_paper_instance":
        state["created_task"] = task_snapshot(
            CREATED_TASK_ID, sequence=0, status="queued"
        )
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
            response = {"protocol_version": 5, "payload": payload}
            connection.sendall(json.dumps(response, ensure_ascii=False).encode() + b"\n")
        except (KeyError, ValueError, OSError) as error:
            print(f"request failed: {error}", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--socket", type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.socket.parent.mkdir(parents=True, exist_ok=True)
    args.socket.unlink(missing_ok=True)
    state = {
        "server_state": "installed",
        "message": "已安装，当前未运行。",
        "logs": "",
        "console_commands": [],
        "created_task": None,
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
