#!/usr/bin/env python3
"""Verify documentation targets/anchors, release tags, and generated site freshness."""

import re
import sys
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

import build_docs_site
import build_landing_site

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "site"


class Page(HTMLParser):
    def __init__(self, source):
        super().__init__()
        self.ids = []
        self.targets = []
        self.feed(source.read_text())

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if values.get("id"):
            self.ids.append(values["id"])
        for attr in ("href", "src"):
            if values.get(attr):
                self.targets.append(values[attr])


def markdown_ids(source):
    ids = set(re.findall(r'<a\s+(?:name|id)="([^"]+)"', source))
    counts = Counter()
    for heading in re.findall(r"^#{1,6}\s+(.+)$", source, re.MULTILINE):
        slug = re.sub(r"[^\w\s-]", "", heading.lower())
        slug = re.sub(r"\s", "-", slug)
        index = counts[slug]
        counts[slug] += 1
        ids.add(f"{slug}-{index}" if index else slug)
    return ids


def main():
    errors = []
    checked = 0
    pages = {path: Page(path) for path in SITE.rglob("*.html")}

    def check(source, target):
        nonlocal checked
        url = urlsplit(target.strip("<>"))
        if url.scheme or url.netloc:
            return
        checked += 1
        if url.path.startswith("/mc-server-tui/"):
            destination = SITE / unquote(url.path.removeprefix("/mc-server-tui/"))
        elif url.path.startswith("/"):
            destination = SITE / unquote(url.path.lstrip("/"))
        else:
            destination = source.parent / unquote(url.path) if url.path else source
        destination = destination.resolve()
        if destination.is_dir():
            destination /= "index.html"
        if not destination.is_file():
            errors.append(f"{source.relative_to(ROOT)}: missing target {target}")
            return
        if url.fragment:
            fragment = unquote(url.fragment)
            if destination.suffix == ".html":
                document = pages.get(destination) or Page(destination)
                ids = document.ids
            elif destination.suffix == ".md":
                ids = markdown_ids(destination.read_text())
            else:
                return
            if fragment not in ids:
                errors.append(f"{source.relative_to(ROOT)}: missing anchor {target}")

    documents = list((ROOT / "docs").rglob("*.md"))
    documents += [
        ROOT / name
        for name in (
            "README.md",
            "README.zh-CN.md",
            "CHANGELOG.md",
            "CONTRIBUTING.md",
            "SECURITY.md",
        )
    ]
    for source in documents:
        markdown = re.sub(
            r"^```[^\n]*\n.*?^```\s*$", "", source.read_text(), flags=re.MULTILINE | re.DOTALL
        )
        targets = re.findall(r"\[[^\]]*\]\(([^)]+)\)", markdown)
        targets += re.findall(r'(?:src|href)="([^"]+)"', markdown)
        for target in targets:
            check(source, target)
    for source, page in pages.items():
        duplicates = [key for key, count in Counter(page.ids).items() if count > 1]
        if duplicates:
            errors.append(f"{source.relative_to(ROOT)}: duplicate ids {duplicates}")
        for target in page.targets:
            check(source, target)
    for page in build_docs_site.PAGES:
        output = page["output"]
        if not output.is_file() or output.read_text() != build_docs_site.render_page(
            page
        ):
            errors.append(
                f"{output.relative_to(ROOT)}: regenerate with scripts/build_docs_site.py"
            )
    for lang, output in build_landing_site.PAGES:
        if not output.is_file() or output.read_text() != build_landing_site.render_page(
            lang
        ):
            errors.append(
                f"{output.relative_to(ROOT)}: regenerate with scripts/build_landing_site.py"
            )
    for name in (
        "README.md",
        "README.zh-CN.md",
        "docs/QUICK_START.md",
        "docs/USER_GUIDE.md",
        "docs/USER_GUIDE.zh-CN.md",
        "docs/installation/macos.md",
        "docs/installation/linux.md",
    ):
        tags = re.findall(r"TAG=v([\w.+-]+)", (ROOT / name).read_text())
        if tags != [build_landing_site.VERSION]:
            errors.append(
                f"{name}: installer tag must match workspace {build_landing_site.VERSION}"
            )
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(
        f"PASS: {len(documents)} Markdown files, {len(pages)} HTML pages, {checked} local targets/anchors; all four generated pages and installer tags match sources"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
