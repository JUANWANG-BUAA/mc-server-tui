#!/usr/bin/env python3
"""Package workspace binaries into a versioned tarball and update a release manifest.

This is intentionally dependency-free so it can run on a clean macOS/Linux CI
runner. It does not sign anything yet; Phase 2 adds manifest/artifact signatures.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tarfile
from datetime import datetime, timezone
from pathlib import Path

BINARIES = (
    "mc",
    "mc-server-tui",
    "mc-server-tui-daemon",
    "mc-server-tui-cli",
)
DOCUMENTS = (
    "README.md",
    "README.zh-CN.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def runtime_versions(binary_dir: Path) -> dict[str, int]:
    try:
        output = subprocess.check_output(
            [str(binary_dir / "mc"), "version", "--json"],
            text=True,
            stderr=subprocess.STDOUT,
        )
        data = json.loads(output)
        return {
            "api_protocol_min": int(data["api_protocol_min"]),
            "api_protocol_max": int(data["api_protocol_max"]),
            "state_schema": int(data["state_schema"]),
            "runner_protocol": int(data["runner_protocol"]),
        }
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError) as error:
        raise SystemExit(
            f"could not read release protocol metadata from {binary_dir / 'mc'}: {error}"
        ) from error


def package(args: argparse.Namespace) -> tuple[Path, Path]:
    binary_dir = args.binary_dir
    if binary_dir is None:
        candidates = (
            Path("target") / args.target / "release",
            Path("target") / "release",
        )
        binary_dir = next((path for path in candidates if path.is_dir()), candidates[0])
    missing = [name for name in BINARIES if not (binary_dir / name).is_file()]
    if missing:
        raise SystemExit(
            f"missing release binaries in {binary_dir}: {', '.join(missing)}\n"
            "run `cargo build --workspace --release` first"
        )
    versions = runtime_versions(binary_dir)

    output_dir = args.output_dir / args.version
    output_dir.mkdir(parents=True, exist_ok=True)
    archive_name = f"mc-server-tui-{args.version}-{args.target}.tar.gz"
    archive_path = output_dir / archive_name
    root_name = f"mc-server-tui-{args.version}-{args.target}"

    with tarfile.open(archive_path, "w:gz") as archive:
        for name in BINARIES:
            source = binary_dir / name
            info = archive.gettarinfo(str(source), arcname=f"{root_name}/bin/{name}")
            info.mode = 0o755
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mtime = 0
            with source.open("rb") as handle:
                archive.addfile(info, handle)
        for name in DOCUMENTS:
            source = Path(name)
            if not source.is_file():
                raise SystemExit(f"missing project document: {name}")
            info = archive.gettarinfo(str(source), arcname=f"{root_name}/{name}")
            info.mode = 0o644
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mtime = 0
            with source.open("rb") as handle:
                archive.addfile(info, handle)

    digest = sha256(archive_path)
    checksum_path = archive_path.with_name(archive_name + ".sha256")
    checksum_path.write_text(f"{digest}  {archive_name}\n", encoding="utf-8")

    manifest_path = output_dir / "release-manifest.json"
    if manifest_path.is_file():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("version") != args.version:
            raise SystemExit(
                f"existing manifest is for {manifest.get('version')!r}, not {args.version!r}"
            )
        if manifest.get("channel") != args.channel:
            raise SystemExit(
                f"existing manifest channel is {manifest.get('channel')!r}, not {args.channel!r}"
            )
    else:
        manifest = {
            "schema": 1,
            "product": "mc-server-tui",
            "version": args.version,
            "channel": args.channel,
            "released_at": datetime.now(timezone.utc)
            .isoformat()
            .replace("+00:00", "Z"),
            "min_supported_version": args.min_supported_version or args.version,
            "api_protocol": {
                "min": args.api_min or versions["api_protocol_min"],
                "max": args.api_max or versions["api_protocol_max"],
            },
            "state_schema": versions["state_schema"],
            "runner_protocol": {
                "min": versions["runner_protocol"],
                "max": versions["runner_protocol"],
            },
            "notes_url": args.notes_url,
            "targets": {},
        }

    base_url = args.base_url.rstrip("/")
    url = f"{base_url}/{archive_name}" if base_url else archive_name
    manifest["targets"][args.target] = {
        "url": url,
        "sha256": digest,
        "size": archive_path.stat().st_size,
        "format": "tar.gz",
        "signature": None,
    }
    manifest_path.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return archive_path, manifest_path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--channel", default="stable", choices=("stable", "beta"))
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary-dir", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("dist"))
    parser.add_argument(
        "--base-url",
        default="https://example.invalid/mc-server-tui",
        help="public directory containing the tarball; URLs are written into the manifest",
    )
    parser.add_argument("--min-supported-version")
    parser.add_argument("--api-min", type=int)
    parser.add_argument("--api-max", type=int)
    parser.add_argument(
        "--notes-url",
        default="https://github.com/JUANWANG-BUAA/mc-server-tui/releases",
    )
    args = parser.parse_args()
    archive, manifest = package(args)
    print(f"archive:  {archive}")
    print(f"manifest: {manifest}")
    print(f"sha256:   {sha256(archive)}")


if __name__ == "__main__":
    main()
