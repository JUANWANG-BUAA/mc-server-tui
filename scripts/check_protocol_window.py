#!/usr/bin/env python3
"""Start an isolated real daemon and check the API N/N-1 protocol window.

This test needs no Minecraft download and no Java: it only exercises the
daemon transport. The state and cache directories live in a temporary
directory, and the socket is relative so the macOS 104-byte path limit is not
an issue.
"""

import json
import os
import pathlib
import signal
import socket
import subprocess
import sys
import tempfile
import time


MIN_PROTOCOL = 8
MAX_PROTOCOL = 9


def request(path, protocol, payload):
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.settimeout(5)
    try:
        connection.connect(str(path))
        connection.sendall(
            json.dumps(
                {"protocol_version": protocol, "payload": payload},
                ensure_ascii=False,
            ).encode()
            + b"\n"
        )
        line = bytearray()
        while not line.endswith(b"\n"):
            chunk = connection.recv(4096)
            if not chunk:
                break
            line.extend(chunk)
    finally:
        connection.close()
    if not line:
        raise AssertionError(f"daemon closed the connection for protocol {protocol}")
    return json.loads(line)


def expect(label, response, protocol, result, evidence=None):
    actual_result = response["payload"].get("result")
    if response.get("protocol_version") != protocol:
        raise AssertionError(
            f"{label}: expected response protocol {protocol}, "
            f"got {response.get('protocol_version')}"
        )
    if actual_result != result:
        raise AssertionError(f"{label}: expected result {result!r}, got {actual_result!r}")
    if evidence is not None:
        error = response["payload"].get("data") or {}
        actual = error.get("evidence") or ""
        if evidence not in actual:
            raise AssertionError(
                f"{label}: expected evidence containing {evidence!r}, got {actual!r}"
            )
    print(f"{label}: passed")


def wait_for_socket(path, process, deadline_seconds=5):
    deadline = time.monotonic() + deadline_seconds
    while time.monotonic() < deadline:
        if path.exists():
            return
        if process.poll() is not None:
            raise AssertionError(
                f"daemon exited before binding its socket (exit {process.returncode})"
            )
        time.sleep(0.05)
    raise AssertionError(f"daemon did not bind {path} within {deadline_seconds}s")


def main():
    repo = pathlib.Path(__file__).resolve().parents[1]
    daemon = repo / "target" / "debug" / "mc-server-tui-daemon"
    if not daemon.is_file():
        raise SystemExit(
            "Build the daemon first: cargo build -p mc-daemon"
        )

    with tempfile.TemporaryDirectory(prefix="mcst-protocol-") as directory:
        root = pathlib.Path(directory)
        os.chmod(root, 0o700)
        socket_path = root / "run" / "daemon.sock"
        log_path = root / "daemon.log"
        environment = os.environ.copy()
        environment["MC_SERVER_TUI_CONFIG"] = str(root / "config.toml")
        with log_path.open("wb") as log:
            process = subprocess.Popen(
                [
                    str(daemon),
                    "--state-dir",
                    "state",
                    "--cache-dir",
                    "cache",
                    "--socket",
                    "run/daemon.sock",
                ],
                cwd=root,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            try:
                wait_for_socket(socket_path, process)
                expect(
                    "v8 ping",
                    request(socket_path, 8, {"operation": "ping"}),
                    8,
                    "pong",
                )
                expect(
                    "v9 ping",
                    request(socket_path, 9, {"operation": "ping"}),
                    9,
                    "pong",
                )
                info = request(socket_path, 8, {"operation": "server_info"})
                expect("v8 server_info", info, 8, "server_info")
                data = info["payload"]["data"]
                if data.get("api_protocol_min") != MIN_PROTOCOL or data.get(
                    "api_protocol_max"
                ) != MAX_PROTOCOL:
                    raise AssertionError(f"unexpected protocol range: {data}")
                if data.get("state_schema") != 1 or data.get("runner_protocol") != 1:
                    raise AssertionError(f"unexpected state/runner versions: {data}")

                management = request(
                    socket_path,
                    8,
                    {
                        "operation": "manage_instance",
                        "arguments": {
                            "instance_id": "00000000-0000-0000-0000-000000000000",
                            "action": {"action": "list_addons"},
                        },
                    },
                )
                expect(
                    "v8 manage_instance rejected",
                    management,
                    8,
                    "error",
                    evidence="requires API protocol 9",
                )
                expect(
                    "v7 rejected",
                    request(socket_path, 7, {"operation": "ping"}),
                    8,
                    "error",
                    evidence="daemon protocol range",
                )
                expect(
                    "v10 rejected",
                    request(socket_path, 10, {"operation": "ping"}),
                    9,
                    "error",
                    evidence="daemon protocol range",
                )
            finally:
                # SIGINT takes the daemon's graceful shutdown path and removes
                # its socket; SIGTERM would bypass that handler.
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
                if process.returncode not in (0, -signal.SIGINT):
                    log.flush()
                    print(log_path.read_text(errors="replace"), file=sys.stderr)

    print("Real daemon protocol window: passed")


if __name__ == "__main__":
    main()
