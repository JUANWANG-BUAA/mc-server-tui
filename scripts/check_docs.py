#!/usr/bin/env python3
"""Check user-facing local links and generated guide freshness without writing files."""
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

import build_docs_site

ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = [
    "README.md", "README.zh-CN.md", "CHANGELOG.md", "docs/README.md",
    "docs/QUICK_START.md", "docs/TROUBLESHOOTING_CHEATSHEET.md",
    "docs/USER_GUIDE.md", "docs/USER_GUIDE.zh-CN.md",
    "docs/PRODUCTIZATION_STATUS.md", "docs/INTERACTION_DESIGN.md",
    "docs/installation/linux.md", "docs/installation/macos.md",
    "docs/installation/TESTING_CHECKLIST.md",
]


def main():
    errors = []
    checked = 0
    for name in DOCUMENTS:
        source = ROOT / name
        # Ignore shell examples and other fenced code blocks.
        markdown = re.sub(r"^```[^\n]*\n.*?^```\s*$", "", source.read_text(), flags=re.M | re.S)
        targets = re.findall(r"\[[^\]]*\]\(([^)]+)\)", markdown)
        targets += re.findall(r'(?:src|href)="([^"]+)"', markdown)
        for target in targets:
            url = urlsplit(target.strip("<>"))
            if url.scheme or url.netloc or not url.path:
                continue
            destination = source.parent / unquote(url.path)
            checked += 1
            if not destination.exists():
                errors.append(f"{name}: missing local target {target}")
    for page in build_docs_site.PAGES:
        output = page["output"]
        if not output.is_file() or output.read_text() != build_docs_site.render_page(page):
            errors.append(f"{output.relative_to(ROOT)}: regenerate with scripts/build_docs_site.py")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"PASS: {checked} local links; both generated guides match their sources")
    return 0


if __name__ == "__main__":
    sys.exit(main())
