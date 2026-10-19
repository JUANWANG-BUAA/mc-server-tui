#!/usr/bin/env python3
"""Merge per-target release manifests produced by package_release.py."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifests", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    merged = None
    for path in args.manifests:
        data = json.loads(path.read_text(encoding="utf-8"))
        if data.get("schema") != 1:
            raise SystemExit(f"{path}: unsupported manifest schema")
        if merged is None:
            merged = {key: value for key, value in data.items() if key != "targets"}
            merged["targets"] = {}
        else:
            for field in (
                "product",
                "version",
                "channel",
                "min_supported_version",
                "api_protocol",
                "state_schema",
            ):
                if data.get(field) != merged.get(field):
                    raise SystemExit(
                        f"{path}: {field} differs from the first manifest"
                    )
        for target, item in (data.get("targets") or {}).items():
            if target in merged["targets"] and merged["targets"][target] != item:
                raise SystemExit(f"duplicate target with different metadata: {target}")
            merged["targets"][target] = item

    if merged is None:
        raise SystemExit("no manifests supplied")
    if not merged["targets"]:
        raise SystemExit("merged manifest has no targets")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(merged, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"merged {len(args.manifests)} manifests -> {args.output}")
    print("targets: " + ", ".join(sorted(merged["targets"])))


if __name__ == "__main__":
    main()
