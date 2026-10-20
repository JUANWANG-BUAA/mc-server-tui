#!/usr/bin/env python3
"""Render the Homebrew formula template from a release manifest."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument(
        "--template",
        type=Path,
        default=Path("packaging/homebrew/mc-server-tui.rb"),
    )
    parser.add_argument("--output", type=Path, default=Path("dist/mc-server-tui.rb"))
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    targets = manifest["targets"]

    def target(name: str) -> dict[str, str]:
        try:
            return targets[name]
        except KeyError as error:
            raise SystemExit(f"manifest has no macOS target {name!r}") from error

    arm = target("aarch64-apple-darwin")
    template = args.template.read_text(encoding="utf-8")
    replacements = {
        "{{VERSION}}": manifest["version"],
        "{{URL_ARM64}}": arm["url"],
        "{{SHA256_ARM64}}": arm["sha256"],
    }
    for marker, value in replacements.items():
        template = template.replace(marker, value)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(template, encoding="utf-8")
    print(args.output)


if __name__ == "__main__":
    main()
