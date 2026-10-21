#!/usr/bin/env python3
"""Build the static bilingual user-guide pages from the Markdown sources."""

from __future__ import annotations

import html
import re
import struct
import sys
from dataclasses import dataclass
from pathlib import Path

from build_landing_site import VERSION

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Section:
    anchor: str
    label: str
    group: str


EN_SECTIONS = (
    Section("read-before-hosting", "Start here", "Get started"),
    Section("supported-scope", "Supported scope", "Get started"),
    Section("install", "Install", "Get started"),
    Section("create-the-first-server", "First server", "Get started"),
    Section("session-shell", "Session shell", "Day to day"),
    Section("lifecycle-readiness", "Lifecycle", "Day to day"),
    Section("common-configuration", "Configuration", "Day to day"),
    Section("backups-restore", "Backups and restore", "Day to day"),
    Section("players-logs-workspaces", "Players and logs", "Day to day"),
    Section("fabric-addons-server-packs", "Fabric and packs", "Day to day"),
    Section("cli", "CLI", "Operate"),
    Section("service-update-data", "Service and data", "Operate"),
    Section("troubleshooting", "Troubleshooting", "Operate"),
    Section("network-exposure", "Network exposure", "Operate"),
    Section("evidence-current-limitations", "Evidence and limits", "Trust"),
    Section("license-no-warranty-notice", "License and disclaimer", "Trust"),
)

ZH_SECTIONS = (
    Section("read-before-hosting", "开始阅读", "开始使用"),
    Section("supported-scope", "支持范围", "开始使用"),
    Section("install", "安装", "开始使用"),
    Section("create-the-first-server", "创建首个服务器", "开始使用"),
    Section("session-shell", "会话 shell", "日常操作"),
    Section("lifecycle-readiness", "生命周期", "日常操作"),
    Section("common-configuration", "常用配置", "日常操作"),
    Section("backups-restore", "备份与恢复", "日常操作"),
    Section("players-logs-workspaces", "玩家与日志", "日常操作"),
    Section("fabric-addons-server-packs", "Fabric 与整合包", "日常操作"),
    Section("cli", "CLI", "运维"),
    Section("service-update-data", "服务与数据", "运维"),
    Section("troubleshooting", "排错", "运维"),
    Section("network-exposure", "网络暴露", "运维"),
    Section("evidence-current-limitations", "证据与限制", "可信信息"),
    Section("license-no-warranty-notice", "许可证与免责声明", "可信信息"),
)


PAGES = (
    {
        "lang": "en",
        "source": ROOT / "docs" / "USER_GUIDE.md",
        "output": ROOT / "site" / "docs" / "index.html",
        "asset_base": "../assets/",
        "home": "../",
        "docs_home": "./",
        "alternate": "zh-CN/",
        "alternate_lang": "zh-CN",
        "alternate_label": "简体中文",
        "title": "User Guide · mc-server-tui",
        "description": (
            "Install, run, back up, troubleshoot, and understand the safety limits "
            "of the mc-server-tui v__VERSION__ Minecraft server manager."
        ),
        "eyebrow": "USER GUIDE · V__VERSION__ ALPHA",
        "hero_title": "Your first server.<br><em>Then your everyday workflow.</em>",
        "hero_lede": (
            "A complete field guide to installing the alpha, creating a Paper or "
            "Fabric server, running it day to day, and recovering a world with care. "
            "Follow the expected results at each step, and use the failure guide when an operation needs attention."
        ),
        "hero_meta": ("Unsigned alpha", "16 chapters", "About 20 minutes"),
        "quick_install": "Install",
        "quick_create": "Create a server",
        "quick_backup": "Back up a world",
        "header_home": "Home",
        "header_guide": "Guide",
        "header_troubleshooting": "Troubleshooting",
        "sidebar_title": "Contents",
        "search_label": "Filter chapters",
        "search_placeholder": "Find a chapter…",
        "search_empty": "No matching chapter.",
        "mobile_label": "Open chapter list",
        "toc_title": "On this page",
        "read_label": "Read",
        "source_note_label": "Alpha note",
        "source_note": (
            "This guide documents the shipped v__VERSION__ behavior. The release is "
            "unsigned and does not promise production stability or data recovery."
        ),
        "source_note_link": "Read the full safety guidance",
        "callout_label": "Notice",
        "copy_label": "Copy",
        "copied_label": "Copied",
        "copy_failed_label": "Select",
        "footer_next_label": "Continue with confidence",
        "footer_next_title": "A backup is part of the workflow.",
        "footer_next_body": (
            "Keep an independent copy of important worlds and test restoration "
            "before changing server software, plugins, mods, or configuration."
        ),
        "footer_next_link": "Backups and restore",
        "sections": EN_SECTIONS,
    },
    {
        "lang": "zh-CN",
        "source": ROOT / "docs" / "USER_GUIDE.zh-CN.md",
        "output": ROOT / "site" / "docs" / "zh-CN" / "index.html",
        "asset_base": "../../assets/",
        "home": "../../zh-CN/",
        "docs_home": "./",
        "alternate": "../",
        "alternate_lang": "en",
        "alternate_label": "English",
        "title": "使用手册 · mc-server-tui",
        "description": (
            "mc-server-tui v__VERSION__ 中文使用手册：安装、创建服务器、日常操作、"
            "备份恢复、排错以及 alpha 版本的安全边界。"
        ),
        "eyebrow": "使用手册 · V__VERSION__ ALPHA",
        "hero_title": "从第一个服务器，<br><em>学会日常管理。</em>",
        "hero_lede": (
            "从安装 alpha、创建 Paper 或 Fabric 服务器，到日常运行与谨慎恢复世界，"
            "这份手册覆盖完整操作流程。把重要世界交给它之前，请先阅读安全须知。"
        ),
        "hero_meta": ("未签名 alpha", "16 个章节", "约 20 分钟"),
        "quick_install": "安装",
        "quick_create": "创建服务器",
        "quick_backup": "备份世界",
        "header_home": "首页",
        "header_guide": "文档",
        "header_troubleshooting": "排错",
        "sidebar_title": "目录",
        "search_label": "筛选章节",
        "search_placeholder": "查找章节…",
        "search_empty": "没有匹配的章节。",
        "mobile_label": "展开章节列表",
        "toc_title": "本页内容",
        "read_label": "阅读",
        "source_note_label": "Alpha 提示",
        "source_note": (
            "本手册描述 v__VERSION__ 已发布行为。当前版本未签名，"
            "不承诺生产稳定性或数据一定能够恢复。"
        ),
        "source_note_link": "阅读完整安全说明",
        "callout_label": "提示",
        "copy_label": "复制",
        "copied_label": "已复制",
        "copy_failed_label": "手动选择",
        "footer_next_label": "下一步",
        "footer_next_title": "把备份纳入日常流程。",
        "footer_next_body": (
            "为重要世界保留应用之外的独立副本，并在更换服务端软件、插件、"
            "模组或配置前验证恢复流程。"
        ),
        "footer_next_link": "备份与恢复",
        "sections": ZH_SECTIONS,
    },
)


def inline(text: str) -> str:
    """Render the small inline Markdown subset used by the guides."""
    code_spans: list[tuple[str, str]] = []

    def keep_code(match: re.Match[str]) -> str:
        token = f"@@CODE{len(code_spans)}@@"
        code_spans.append((token, match.group(1)))
        return token

    text = re.sub(r"`([^`]+)`", keep_code, text)
    text = html.escape(text, quote=False)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)

    def render_link(match: re.Match[str]) -> str:
        label = match.group(1)
        href = match.group(2)
        external = href.startswith(("http://", "https://"))
        suffix = ' target="_blank" rel="noreferrer"' if external else ""
        return f'<a href="{href}"{suffix}>{label}</a>'

    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", render_link, text)
    for token, value in code_spans:
        text = text.replace(token, f"<code>{html.escape(value, quote=False)}</code>")
    return text


def slugify(value: str) -> str:
    value = re.sub(r"[^\w\s-]", "", value.lower())
    return re.sub(r"[-\s]+", "-", value).strip("-") or "section"


def render_table(lines: list[str]) -> str:
    rows = []
    for line in lines:
        cells = [inline(cell.strip()) for cell in line.strip().strip("|").split("|")]
        rows.append(cells)
    head = rows[0]
    body = rows[2:]
    output = ['<div class="docs-table-wrap"><table class="docs-table"><thead><tr>']
    output.extend(f"<th>{cell}</th>" for cell in head)
    output.append("</tr></thead><tbody>")
    for row in body:
        output.append("<tr>")
        output.extend(f"<td>{cell}</td>" for cell in row)
        output.append("</tr>")
    output.append("</tbody></table></div>")
    return "".join(output)


def render_code(
    language: str,
    lines: list[str],
    copy_label: str,
    copied_label: str,
    copy_failed_label: str,
    language_label: str,
) -> str:
    label = {
        "sh": language_label,
        "text": "TUI",
        "json": "JSON",
    }.get(language, language.upper() or "Code")
    value = html.escape("\n".join(lines), quote=False)
    return (
        '<div class="docs-code">'
        '<div class="docs-code-bar">'
        f"<span>{label}</span>"
        f'<button class="docs-code-copy" type="button" '
        f'data-copy-label="{html.escape(copy_label, quote=True)}" '
        f'data-copied-label="{html.escape(copied_label, quote=True)}" '
        f'data-failed-label="{html.escape(copy_failed_label, quote=True)}">'
        f"{html.escape(copy_label)}</button>"
        "</div>"
        f"<pre><code>{value}</code></pre>"
        "</div>"
    )


def render_guide(page: dict[str, object]) -> tuple[str, list[tuple[str, str]]]:
    source = Path(page["source"])
    display_sections = page["sections"]
    if not isinstance(display_sections, tuple):
        raise TypeError("sections must be a tuple")
    copy_label = str(page["copy_label"])
    copied_label = str(page["copied_label"])
    copy_failed_label = str(page["copy_failed_label"])
    language_label = "终端" if page["lang"] == "zh-CN" else "Shell"

    lines = source.read_text(encoding="utf-8").splitlines()
    output: list[str] = []
    headings: list[tuple[str, str]] = []
    section_index = -1
    in_section = False
    seen: set[str] = set()
    i = 0

    def close_section() -> None:
        nonlocal in_section
        if in_section:
            if section_index + 1 < len(display_sections):
                next_section = display_sections[section_index + 1]
                next_label = "下一章" if page["lang"] == "zh-CN" else "Next chapter"
                output.append(
                    f'<nav class="chapter-next" aria-label="{next_label}">'
                    f'<span>{next_label}</span><a href="#{next_section.anchor}">'
                    f"{html.escape(next_section.label)} →</a></nav>"
                )
            output.append("</section>")
            in_section = False

    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if not stripped:
            i += 1
            continue

        if section_index == -1 and not stripped.startswith("## "):
            i += 1
            continue

        if stripped.startswith("```"):
            language = stripped[3:].strip()
            code_lines: list[str] = []
            i += 1
            while i < len(lines) and not lines[i].strip().startswith("```"):
                code_lines.append(lines[i])
                i += 1
            output.append(
                render_code(
                    language,
                    code_lines,
                    copy_label,
                    copied_label,
                    copy_failed_label,
                    language_label,
                )
            )
            i += 1
            continue

        if stripped.startswith("# "):
            i += 1
            continue

        if stripped.startswith("## "):
            close_section()
            section_index += 1
            if section_index >= len(display_sections):
                raise ValueError(
                    f"More H2 headings than configured sections in {source}"
                )
            meta = display_sections[section_index]
            title = re.sub(r"^\d+\.\s*", "", stripped[3:].strip())
            heading_text = html.escape(title, quote=False)
            headings.append((meta.anchor, title))
            output.append(
                f'<section class="doc-section" id="{meta.anchor}">'
                '<header class="doc-section-head">'
                f'<span class="doc-section-kicker">{section_index + 1:02d} / '
                f"{html.escape(meta.group)}</span>"
                f"<h2>{heading_text}</h2>"
                "</header>"
            )
            in_section = True
            i += 1
            continue

        image = re.fullmatch(r"!\[([^\]]*)\]\(([^)]+)\)", stripped)
        if image:
            label, target = image.groups()
            if not target.startswith("../site/assets/"):
                raise ValueError(f"Unsupported guide image path: {target}")
            asset = str(page["asset_base"]) + target.removeprefix("../site/assets/")
            image_path = source.parent / target
            width, height = struct.unpack(">II", image_path.read_bytes()[16:24])
            output.append(
                '<figure class="doc-figure">'
                f'<img src="{html.escape(asset, quote=True)}" '
                f'alt="{html.escape(label, quote=True)}" loading="lazy" width="{width}" height="{height}">'
                f"<figcaption>{html.escape(label)}</figcaption></figure>"
            )
            i += 1
            continue

        if stripped.startswith("### "):
            title = stripped[4:].strip()
            base = slugify(title)
            anchor = base
            counter = 2
            while anchor in seen:
                anchor = f"{base}-{counter}"
                counter += 1
            seen.add(anchor)
            output.append(f'<h3 id="{anchor}">{html.escape(title, quote=False)}</h3>')
            i += 1
            continue

        if stripped.startswith(">"):
            quote_lines: list[str] = []
            while i < len(lines) and lines[i].strip().startswith(">"):
                quote_lines.append(re.sub(r"^\s*>\s?", "", lines[i]))
                i += 1
            quote = " ".join(part.strip() for part in quote_lines if part.strip())
            output.append(
                '<aside class="doc-callout doc-callout-warning">'
                f'<span class="doc-callout-label">'
                f"{html.escape(str(page['callout_label']))}</span>"
                f"<p>{inline(quote)}</p></aside>"
            )
            continue

        if (
            "|" in stripped
            and i + 1 < len(lines)
            and re.match(r"^\s*\|?[\s:|-]+\|[\s:|-]*$", lines[i + 1])
        ):
            table_lines = [stripped, lines[i + 1].strip()]
            i += 2
            while i < len(lines) and "|" in lines[i] and lines[i].strip():
                table_lines.append(lines[i].strip())
                i += 1
            output.append(render_table(table_lines))
            continue

        unordered = re.match(r"^\s*-\s+(.*)$", line)
        ordered = re.match(r"^\s*\d+\.\s+(.*)$", line)
        if unordered or ordered:
            tag = "ul" if unordered else "ol"
            items: list[str] = []
            pattern = r"^\s*-\s+(.*)$" if unordered else r"^\s*\d+\.\s+(.*)$"
            while i < len(lines):
                match = re.match(pattern, lines[i])
                if not match:
                    break
                item = match.group(1).strip()
                i += 1
                while (
                    i < len(lines)
                    and lines[i].strip()
                    and not re.match(r"^\s*(?:-\s+|\d+\.\s+|#{1,3}\s|>|```)", lines[i])
                ):
                    item += " " + lines[i].strip()
                    i += 1
                items.append(f"<li>{inline(item)}</li>")
            output.append(f"<{tag}>" + "".join(items) + f"</{tag}>")
            continue

        paragraph_lines = [stripped]
        i += 1
        while i < len(lines):
            candidate = lines[i].strip()
            if (
                not candidate
                or candidate.startswith(("#", ">", "```", "- "))
                or re.match(r"^\d+\.\s+", candidate)
                or (
                    "|" in candidate
                    and i + 1 < len(lines)
                    and re.match(r"^\s*\|?[\s:|-]+\|[\s:|-]*$", lines[i + 1])
                )
            ):
                break
            paragraph_lines.append(candidate)
            i += 1
        output.append(f"<p>{inline(' '.join(paragraph_lines))}</p>")

    close_section()
    if section_index + 1 != len(display_sections):
        raise ValueError(
            f"Expected {len(display_sections)} H2 headings in {source}, "
            f"found {section_index + 1}"
        )
    return "\n".join(output), headings


def render_sidebar(page: dict[str, object]) -> str:
    sections = page["sections"]
    if not isinstance(sections, tuple):
        raise TypeError("sections must be a tuple")
    groups: dict[str, list[Section]] = {}
    for section in sections:
        groups.setdefault(section.group, []).append(section)

    output = ['<div class="docs-nav-groups">']
    for group, entries in groups.items():
        output.append('<div class="docs-nav-group" data-doc-group>')
        output.append(f'<p class="docs-nav-label">{html.escape(group)}</p>')
        for entry in entries:
            output.append(
                f'<a href="#{entry.anchor}" data-doc-link>{html.escape(entry.label)}</a>'
            )
        output.append("</div>")
    output.append("</div>")
    return "".join(output)


def render_toc(page: dict[str, object], headings: list[tuple[str, str]]) -> str:
    return "".join(
        f'<a href="#{anchor}" data-toc-link>{html.escape(label)}</a>'
        for anchor, label in headings
    )


def render_mobile_nav(page: dict[str, object]) -> str:
    sections = page["sections"]
    if not isinstance(sections, tuple):
        raise TypeError("sections must be a tuple")
    return "".join(
        f'<a href="#{section.anchor}">{html.escape(section.label)}</a>'
        for section in sections
    )


def render_page(page: dict[str, object]) -> str:
    article, headings = render_guide(page)
    sections = page["sections"]
    if not isinstance(sections, tuple):
        raise TypeError("sections must be a tuple")

    sidebar = render_sidebar(page)
    toc = render_toc(page, headings)
    mobile_nav = render_mobile_nav(page)
    hero_meta = "".join(
        f"<span>{html.escape(item)}</span>" for item in page["hero_meta"]
    )

    template = """<!doctype html>
<html lang="__LANG__" data-theme="claude">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__TITLE__</title>
  <meta name="description" content="__DESCRIPTION__">
  <meta name="theme-color" content="#f5f6ef">
  <link rel="canonical" href="__CANONICAL__">
  <link rel="alternate" hreflang="en" href="__CANONICAL_EN__">
  <link rel="alternate" hreflang="zh-CN" href="__CANONICAL_ZH__">
  <link rel="alternate" hreflang="x-default" href="__CANONICAL_EN__">
  <link rel="icon" href="__ASSET__favicon.svg" type="image/svg+xml">
  <link rel="stylesheet" href="__ASSET__styles.css?v=__VERSION__">
  <meta property="og:type" content="article">
  <meta property="og:title" content="__TITLE__">
  <meta property="og:description" content="__DESCRIPTION__">
  <meta property="og:url" content="__CANONICAL__">
  <meta property="og:image" content="https://juanwang-buaa.github.io/mc-server-tui/assets/session-claude-0.2.1.png">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="__TITLE__">
  <meta name="twitter:description" content="__DESCRIPTION__">
  <meta name="twitter:image" content="https://juanwang-buaa.github.io/mc-server-tui/assets/session-claude-0.2.1.png">
  <script src="__ASSET__app.js?v=__VERSION__" defer></script>
  <script src="__ASSET__docs.js?v=__VERSION__" defer></script>
</head>
<body class="docs-page">
  <a class="skip-link" href="#main">__SKIP__</a>
  <div class="docs-progress" aria-hidden="true"><span id="docs-progress-bar"></span></div>
  <div class="page-glow" aria-hidden="true"></div>

  <header class="site-header docs-site-header" id="site-header">
    <div class="header-inner">
      <a class="brand" href="__HOME__" aria-label="mc-server-tui">
        <img class="brand-mark" src="__ASSET__favicon.svg" alt="" width="28" height="28">
        <span>mc-server-tui</span>
        <span class="alpha-tag">ALPHA</span>
      </a>
      <nav class="nav docs-top-nav" aria-label="__MAIN_NAV__">
        <a href="__HOME__">__HEADER_HOME__</a>
        <a class="is-active" href="__DOCS_HOME__" aria-current="page">__HEADER_GUIDE__</a>
        <a href="#troubleshooting">__HEADER_TROUBLESHOOTING__</a>
      </nav>
      <div class="docs-top-actions">
        <a class="docs-lang-switch" href="__ALTERNATE__" hreflang="__ALTERNATE_LANG__" lang="__ALTERNATE_LANG__">__ALTERNATE_LABEL__</a>
        <a class="header-github" href="https://github.com/JUANWANG-BUAA/mc-server-tui" target="_blank" rel="noreferrer">GitHub <span aria-hidden="true">↗</span></a>
      </div>
    </div>
  </header>

  <main id="main" class="docs-main">
    <section class="docs-hero">
      <div class="docs-hero-copy">
        <p class="eyebrow"><span class="pulse-dot" aria-hidden="true"></span>__EYEBROW__</p>
        <h1>__HERO_TITLE__</h1>
        <p class="docs-hero-lede">__HERO_LEDE__</p>
        <div class="docs-hero-meta">__HERO_META__</div>
        <div class="docs-quicklinks">
          <a href="#install">__QUICK_INSTALL__ <span aria-hidden="true">→</span></a>
          <a href="#create-the-first-server">__QUICK_CREATE__ <span aria-hidden="true">→</span></a>
          <a href="#backups-restore">__QUICK_BACKUP__ <span aria-hidden="true">→</span></a>
        </div>
      </div>
      <div class="docs-hero-card">
        <span class="docs-card-index">00 / __SOURCE_NOTE_LABEL__</span>
        <p>__SOURCE_NOTE__</p>
        <a href="#read-before-hosting">__SOURCE_NOTE_LINK__ <span aria-hidden="true">↓</span></a>
      </div>
    </section>

    <details class="docs-mobile-menu">
      <summary>__MOBILE_LABEL__</summary>
      <nav aria-label="__SIDEBAR_TITLE__">__MOBILE_NAV__</nav>
    </details>

    <div class="docs-layout">
      <aside class="docs-sidebar">
        <p class="docs-sidebar-title">__SIDEBAR_TITLE__</p>
        <label class="docs-search">
          <span>__SEARCH_LABEL__</span>
          <input id="docs-nav-filter" type="search" placeholder="__SEARCH_PLACEHOLDER__" autocomplete="off">
        </label>
        <nav aria-label="__SIDEBAR_TITLE__">__SIDEBAR__</nav>
        <p class="docs-nav-empty" id="docs-nav-empty" hidden>__SEARCH_EMPTY__</p>
        <div class="docs-sidebar-note">
          <span>ALPHA</span>
          <p>__SOURCE_NOTE__</p>
        </div>
      </aside>

      <article class="docs-article" id="guide">
__ARTICLE__
        <div class="docs-next">
          <span>__FOOTER_NEXT_LABEL__</span>
          <h2>__FOOTER_NEXT_TITLE__</h2>
          <p>__FOOTER_NEXT_BODY__</p>
          <a class="button button-primary" href="#backups-restore">__FOOTER_NEXT_LINK__ <span aria-hidden="true">→</span></a>
        </div>
      </article>

      <aside class="docs-toc">
        <p>__TOC_TITLE__</p>
        <nav aria-label="__TOC_TITLE__">__TOC__</nav>
      </aside>
    </div>
  </main>

  <footer class="site-footer docs-footer">
    <div class="footer-brand">
      <img src="__ASSET__favicon.svg" alt="" width="22" height="22">
      <span>mc-server-tui</span>
      <span class="footer-alpha">alpha</span>
    </div>
    <p>__FOOTER_TAGLINE__</p>
    <div class="footer-links">
      <a href="__HOME__">__HEADER_HOME__</a>
      <a href="__DOCS_HOME__">__HEADER_GUIDE__</a>
      <a href="__ALTERNATE__" hreflang="__ALTERNATE_LANG__">__ALTERNATE_LABEL__</a>
      <a href="https://github.com/JUANWANG-BUAA/mc-server-tui" target="_blank" rel="noreferrer">GitHub</a>
    </div>
    <p class="fine-print">Not affiliated with Mojang, Microsoft, PaperMC, Modrinth, Fabric, or Forge. Minecraft is a trademark of Mojang Synergies AB. <span id="year">2026</span></p>
  </footer>
</body>
</html>
"""

    lang = str(page["lang"])
    if lang == "zh-CN":
        canonical = "https://juanwang-buaa.github.io/mc-server-tui/docs/zh-CN/"
        skip = "跳到正文"
        main_nav = "主导航"
        footer_tagline = (
            "面向 macOS 与 Linux 的终端优先 Minecraft Java 服务端管理工具。"
        )
    else:
        canonical = "https://juanwang-buaa.github.io/mc-server-tui/docs/"
        skip = "Skip to content"
        main_nav = "Main navigation"
        footer_tagline = (
            "Terminal-first Minecraft Java server management for macOS and Linux."
        )

    replacements = {
        "__LANG__": lang,
        "__TITLE__": html.escape(str(page["title"]), quote=True),
        "__DESCRIPTION__": html.escape(str(page["description"]), quote=True),
        "__CANONICAL__": canonical,
        "__CANONICAL_EN__": "https://juanwang-buaa.github.io/mc-server-tui/docs/",
        "__CANONICAL_ZH__": "https://juanwang-buaa.github.io/mc-server-tui/docs/zh-CN/",
        "__ASSET__": str(page["asset_base"]),
        "__HOME__": str(page["home"]),
        "__DOCS_HOME__": str(page["docs_home"]),
        "__ALTERNATE__": str(page["alternate"]),
        "__ALTERNATE_LANG__": str(page["alternate_lang"]),
        "__ALTERNATE_LABEL__": str(page["alternate_label"]),
        "__SKIP__": skip,
        "__MAIN_NAV__": main_nav,
        "__HEADER_HOME__": str(page["header_home"]),
        "__HEADER_GUIDE__": str(page["header_guide"]),
        "__HEADER_TROUBLESHOOTING__": str(page["header_troubleshooting"]),
        "__EYEBROW__": str(page["eyebrow"]),
        "__HERO_TITLE__": str(page["hero_title"]),
        "__HERO_LEDE__": str(page["hero_lede"]),
        "__HERO_META__": hero_meta,
        "__QUICK_INSTALL__": str(page["quick_install"]),
        "__QUICK_CREATE__": str(page["quick_create"]),
        "__QUICK_BACKUP__": str(page["quick_backup"]),
        "__SOURCE_NOTE_LABEL__": str(page["source_note_label"]),
        "__SOURCE_NOTE__": html.escape(str(page["source_note"]), quote=False),
        "__SOURCE_NOTE_LINK__": str(page["source_note_link"]),
        "__MOBILE_LABEL__": str(page["mobile_label"]),
        "__MOBILE_NAV__": mobile_nav,
        "__SIDEBAR_TITLE__": str(page["sidebar_title"]),
        "__SEARCH_LABEL__": str(page["search_label"]),
        "__SEARCH_PLACEHOLDER__": str(page["search_placeholder"]),
        "__SEARCH_EMPTY__": str(page["search_empty"]),
        "__SIDEBAR__": sidebar,
        "__ARTICLE__": article,
        "__FOOTER_NEXT_LABEL__": str(page["footer_next_label"]),
        "__FOOTER_NEXT_TITLE__": str(page["footer_next_title"]),
        "__FOOTER_NEXT_BODY__": str(page["footer_next_body"]),
        "__FOOTER_NEXT_LINK__": str(page["footer_next_link"]),
        "__TOC_TITLE__": str(page["toc_title"]),
        "__TOC__": toc,
        "__FOOTER_TAGLINE__": footer_tagline,
    }
    result = template
    for key, value in replacements.items():
        result = result.replace(key, value)
    return result.replace("__VERSION__", VERSION)


def main() -> int:
    for page in PAGES:
        output = Path(page["output"])
        output.parent.mkdir(parents=True, exist_ok=True)
        rendered = render_page(page)
        output.write_text(rendered, encoding="utf-8")
        print(f"wrote {output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
